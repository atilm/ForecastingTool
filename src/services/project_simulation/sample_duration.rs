use crate::domain::estimate::{
    Estimate, HistogramReferenceEstimate, ReferenceEstimate, StoryPointEstimate,
    ThreePointEstimate,
};
use crate::services::project_simulation::beta_pert_sampler::ThreePointSampler;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SamplingError {
    #[error("Invalid estimate for issue {0}")]
    InvalidEstimate(String),
    #[error("Missing velocity for story point estimate")]
    MissingVelocity,
    #[error("Invalid velocity value: must be greater than 0")]
    InvalidVelocityValue,
}

pub(crate) fn sample_duration_days<R: ThreePointSampler + ?Sized>(
    estimate: &Estimate,
    velocity: Option<f32>,
    sampler: &mut R,
    issue_id: &str,
) -> Result<f32, SamplingError> {
    let (optimistic, most_likely, pessimistic, is_story_point_estimate) = match estimate {
        Estimate::StoryPoint(estimate) => to_story_point_triplet(estimate, issue_id)?,
        Estimate::ThreePoint(estimate) => to_three_point_triplet(estimate)?,
        Estimate::Reference(estimate) => to_reference_triplet(estimate, issue_id)?,
        Estimate::HistogramReference(estimate) => {
            return sample_histogram_duration_days(estimate, sampler, issue_id);
        }
        Estimate::Milestone => (0.0, 0.0, 0.0, false),
    };

    let sampled = sampler
        .sample(optimistic, most_likely, pessimistic)
        .map_err(|_| {
            SamplingError::InvalidEstimate(format!("Sampling failed: {}", issue_id.to_string()))
        })?;

    if !sampled.is_finite() {
        return Err(SamplingError::InvalidEstimate(format!(
            "sample is infinite: {}",
            issue_id.to_string()
        )));
    }

    if is_story_point_estimate {
        let velocity = velocity.ok_or(SamplingError::MissingVelocity)?;
        if velocity <= 0.0 {
            return Err(SamplingError::InvalidVelocityValue);
        }
        Ok(sampled / velocity)
    } else {
        Ok(sampled)
    }
}

fn sample_histogram_duration_days<R: ThreePointSampler + ?Sized>(
    estimate: &HistogramReferenceEstimate,
    sampler: &mut R,
    issue_id: &str,
) -> Result<f32, SamplingError> {
    let histogram = estimate.cached_histogram.as_ref().ok_or_else(|| {
        SamplingError::InvalidEstimate(format!(
            "Missing referenced histogram: {}",
            issue_id.to_string()
        ))
    })?;

    let sampled = sampler.sample_histogram(histogram).map_err(|_| {
        SamplingError::InvalidEstimate(format!(
            "Sampling histogram failed: {}",
            issue_id.to_string()
        ))
    })?;

    if !sampled.is_finite() {
        return Err(SamplingError::InvalidEstimate(format!(
            "sample is infinite: {}",
            issue_id.to_string()
        )));
    }

    Ok(sampled + estimate.elapsed_days)
}

fn to_reference_triplet(
    reference: &ReferenceEstimate,
    issue_id: &str,
) -> Result<(f32, f32, f32, bool), SamplingError> {
    let cached = reference.cached_estimate.as_ref().ok_or_else(|| {
        SamplingError::InvalidEstimate(format!(
            "Missing referenced estimate: {}",
            issue_id.to_string()
        ))
    })?;
    to_three_point_triplet(cached)
}

fn to_story_point_triplet(
    story_points: &StoryPointEstimate,
    issue_id: &str,
) -> Result<(f32, f32, f32, bool), SamplingError> {
    let value = story_points.estimate.ok_or_else(|| {
        SamplingError::InvalidEstimate(format!(
            "Missing story point estimate: {}",
            issue_id.to_string()
        ))
    })?;
    let (lower, upper) = fibonacci_bounds(value);
    let is_story_point_estimate = true;
    Ok((lower, value, upper, is_story_point_estimate))
}

fn to_three_point_triplet(
    estimate: &ThreePointEstimate,
) -> Result<(f32, f32, f32, bool), SamplingError> {
    let optimistic = estimate
        .optimistic
        .ok_or_else(|| SamplingError::InvalidEstimate("missing optimistic value".to_string()))?;
    let most_likely = estimate
        .most_likely
        .ok_or_else(|| SamplingError::InvalidEstimate("missing most likely value".to_string()))?;
    let pessimistic = estimate
        .pessimistic
        .ok_or_else(|| SamplingError::InvalidEstimate("missing pessimistic value".to_string()))?;
    let is_story_point_estimate = false;
    Ok((
        optimistic,
        most_likely,
        pessimistic,
        is_story_point_estimate,
    ))
}

fn fibonacci_bounds(value: f32) -> (f32, f32) {
    let series = [
        0.0, 1.0, 2.0, 3.0, 5.0, 8.0, 13.0, 20.0, 40.0, 100.0, 200.0, 400.0,
    ];

    let lower_than_one = (series[0] + series[1]) / 2.0;
    let greater_than_one = (series[1] + series[2]) / 2.0;

    if value <= lower_than_one {
        return (series[0], series[2]);
    }

    if value > lower_than_one && value <= greater_than_one {
        return (series[0], series[3]);
    }

    for window in series.windows(5) {
        let lower_limit = (window[1] + window[2]) / 2.0;
        let upper_limit = (window[2] + window[3]) / 2.0;
        if value > lower_limit && value <= upper_limit {
            return (window[0], window[4]);
        }
    }

    let lower_than_200 = (series[9] + series[10]) / 2.0;
    let greater_than_200 = (series[10] + series[11]) / 2.0;
    if value > lower_than_200 && value <= greater_than_200 {
        return (series[8], series[11]);
    }

    if value > greater_than_200 && value <= 400.0 {
        return (series[9], series[11]);
    }

    (value, value)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::project_simulation::beta_pert_sampler::ThreePointSamplerError;
    use crate::services::util::histogram::{Histogram, HistogramError};

    struct HistogramSamplerStub {
        histogram_result: Option<f32>,
        histogram_error: Option<HistogramError>,
    }

    impl ThreePointSampler for HistogramSamplerStub {
        fn sample(
            &mut self,
            _optimistic: f32,
            _most_likely: f32,
            _pessimistic: f32,
        ) -> Result<f32, ThreePointSamplerError> {
            Ok(0.0)
        }

        fn sample_histogram(&mut self, _histogram: &Histogram) -> Result<f32, HistogramError> {
            match (self.histogram_result, &self.histogram_error) {
                (Some(value), _) => Ok(value),
                (None, Some(HistogramError::EmptyData)) => Err(HistogramError::EmptyData),
                (None, None) => Ok(0.0),
            }
        }
    }

    #[test]
    fn story_point_triplets_span_two_fibonacci_steps() {
        let test_cases = vec![
            // Points around 5 story points should span from 2 to 13
            (5.0, (2.0, 5.0, 13.0)),
            (4.01, (2.0, 4.01, 13.0)),
            (6.49, (2.0, 6.49, 13.0)),
            // Test values at lower bounds
            (-1.0, (0.0, 0.0, 2.0)),
            (0.0, (0.0, 0.0, 2.0)),
            (0.49, (0.0, 0.49, 2.0)),
            (0.51, (0.0, 0.51, 3.0)),
            (1.49, (0.0, 1.49, 3.0)),
            (1.51, (0.0, 1.51, 5.0)),
            (30.01, (13.0, 30.01, 200.0)),
            // Test values at upper bounds
            (200.0, (40.0, 200.0, 400.0)),
            (300.01, (100.0, 300.01, 400.0)),
            (401.0, (401.0, 401.0, 401.0)),
        ];

        for (input, expected) in test_cases {
            let (lower, most_likely, upper, is_story_point_estimate) = to_story_point_triplet(
                &StoryPointEstimate {
                    estimate: Some(input),
                },
                "test-issue",
            )
            .unwrap();
            assert_eq!(
                lower, expected.0,
                "Lower bound does not match expected value for input {}",
                input
            );
            assert_eq!(
                most_likely, input,
                "Most likely value should match the input for input {}",
                input
            );
            assert_eq!(
                upper, expected.2,
                "Upper bound does not match expected value for input {}",
                input
            );
            assert!(is_story_point_estimate);
        }
    }

    #[test]
    fn histogram_reference_samples_duration_and_adds_elapsed_days() {
        let estimate = HistogramReferenceEstimate {
            histogram_file_path: "histogram.yaml".to_string(),
            cached_histogram: Some(Histogram::from_parts(2.0, 4.0, 1.0, vec![1, 1])),
            elapsed_days: 3.0,
        };
        let mut sampler = HistogramSamplerStub {
            histogram_result: Some(4.5),
            histogram_error: None,
        };

        let sampled = sample_duration_days(
            &Estimate::HistogramReference(estimate),
            None,
            &mut sampler,
            "HIST-1",
        )
        .unwrap();

        assert_eq!(sampled, 7.5);
    }

    #[test]
    fn histogram_reference_requires_cached_histogram() {
        let estimate = HistogramReferenceEstimate {
            histogram_file_path: "histogram.yaml".to_string(),
            cached_histogram: None,
            elapsed_days: 0.0,
        };
        let mut sampler = HistogramSamplerStub {
            histogram_result: Some(1.0),
            histogram_error: None,
        };

        let error = sample_duration_days(
            &Estimate::HistogramReference(estimate),
            None,
            &mut sampler,
            "HIST-2",
        )
        .unwrap_err();

        assert!(matches!(error, SamplingError::InvalidEstimate(message) if message.contains("Missing referenced histogram")));
    }

    #[test]
    fn histogram_reference_rejects_non_finite_sample() {
        let estimate = HistogramReferenceEstimate {
            histogram_file_path: "histogram.yaml".to_string(),
            cached_histogram: Some(Histogram::from_parts(0.0, 1.0, 1.0, vec![1])),
            elapsed_days: 0.0,
        };
        let mut sampler = HistogramSamplerStub {
            histogram_result: Some(f32::INFINITY),
            histogram_error: None,
        };

        let error = sample_duration_days(
            &Estimate::HistogramReference(estimate),
            None,
            &mut sampler,
            "HIST-3",
        )
        .unwrap_err();

        assert!(matches!(error, SamplingError::InvalidEstimate(message) if message.contains("sample is infinite")));
    }
}
