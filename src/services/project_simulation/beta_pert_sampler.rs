use rand::Rng;
use rand_distr::{Beta, Distribution};

pub trait ThreePointSampler {
    fn sample(&mut self, optimistic: f32, most_likely: f32, pessimistic: f32) -> Result<f32, ()>;
}

/// Calculates the PERT expected value: (optimistic + 4 * most_likely + pessimistic) / 6
pub fn pert_expected_value(optimistic: f32, most_likely: f32, pessimistic: f32) -> Result<f32, PertError> {
    if pessimistic < optimistic {
        return Err(PertError::PessimisticLessThanOptimistic);
    }
    if most_likely < optimistic || most_likely > pessimistic {
        return Err(PertError::MostLikelyOutOfRange);
    }
    Ok((optimistic + 4.0 * most_likely + pessimistic) / 6.0)
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum PertError {
    #[error("pessimistic value must be >= optimistic value")]
    PessimisticLessThanOptimistic,
    #[error("most_likely value must be between optimistic and pessimistic")]
    MostLikelyOutOfRange,
}

pub struct BetaPertSampler<R: Rng> {
    rng: R,
}

impl<R: Rng> BetaPertSampler<R> {
    pub fn new(rng: R) -> Self {
        Self { rng }
    }
}

impl<R: Rng> ThreePointSampler for BetaPertSampler<R> {
    fn sample(&mut self, optimistic: f32, most_likely: f32, pessimistic: f32) -> Result<f32, ()> {
        if pessimistic < optimistic {
            return Err(());
        }
        if (pessimistic - optimistic).abs() < f32::EPSILON {
            return Ok(optimistic);
        }
        if most_likely < optimistic || most_likely > pessimistic {
            return Err(());
        }

        let range = (pessimistic - optimistic) as f64;
        let alpha = 1.0 + 4.0 * ((most_likely - optimistic) as f64 / range);
        let beta = 1.0 + 4.0 * ((pessimistic - most_likely) as f64 / range);
        let beta_dist = Beta::new(alpha, beta).map_err(|_| ())?;
        let sample = beta_dist.sample(&mut self.rng) as f32;
        Ok(optimistic + sample * (pessimistic - optimistic))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expected_value_symmetric_estimate() {
        let result = pert_expected_value(1.0, 5.0, 9.0).unwrap();
        assert!((result - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn expected_value_skewed_estimate() {
        // (2 + 4*3 + 10) / 6 = 24/6 = 4.0
        let result = pert_expected_value(2.0, 3.0, 10.0).unwrap();
        assert!((result - 4.0).abs() < f32::EPSILON);
    }

    #[test]
    fn expected_value_zero_duration() {
        let result = pert_expected_value(0.0, 0.0, 0.0).unwrap();
        assert!((result - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn expected_value_equal_bounds() {
        let result = pert_expected_value(5.0, 5.0, 5.0).unwrap();
        assert!((result - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn expected_value_rejects_pessimistic_less_than_optimistic() {
        let result = pert_expected_value(10.0, 5.0, 2.0);
        assert_eq!(result, Err(PertError::PessimisticLessThanOptimistic));
    }

    #[test]
    fn expected_value_rejects_most_likely_below_optimistic() {
        let result = pert_expected_value(5.0, 3.0, 10.0);
        assert_eq!(result, Err(PertError::MostLikelyOutOfRange));
    }

    #[test]
    fn expected_value_rejects_most_likely_above_pessimistic() {
        let result = pert_expected_value(1.0, 12.0, 10.0);
        assert_eq!(result, Err(PertError::MostLikelyOutOfRange));
    }
}
