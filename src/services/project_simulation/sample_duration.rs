use crate::services::project_simulation::beta_pert_sampler::ThreePointSampler;
use crate::domain::estimate::{Estimate, ReferenceEstimate, StoryPointEstimate, ThreePointEstimate};
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
        Estimate::Milestone => (0.0, 0.0, 0.0, false),
    };

    let sampled = sampler
        .sample(optimistic, most_likely, pessimistic)
        .map_err(|_| {
            SamplingError::InvalidEstimate(format!(
                "Sampling failed: {}",
                issue_id.to_string()
            ))
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
    let optimistic = estimate.optimistic.ok_or_else(|| {
        SamplingError::InvalidEstimate("missing optimistic value".to_string())
    })?;
    let most_likely = estimate.most_likely.ok_or_else(|| {
        SamplingError::InvalidEstimate("missing most likely value".to_string())
    })?;
    let pessimistic = estimate.pessimistic.ok_or_else(|| {
        SamplingError::InvalidEstimate("missing pessimistic value".to_string())
    })?;
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
        0.0, 1.0, 2.0, 3.0, 5.0, 8.0, 13.0, 21.0, 34.0, 55.0, 89.0, 144.0, 233.0, 377.0, 610.0,
        987.0,
    ];

    if value <= series[0] {
        return (series[0], series[1]);
    }

    for window in series.windows(2) {
        let lower = window[0];
        let upper = window[1];
        if value <= upper {
            return (lower, upper);
        }
    }

    let last = *series.last().unwrap();
    (last, last)
}