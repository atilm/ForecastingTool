use rand::Rng;
use rand_distr::{Beta, Distribution};

pub trait ThreePointSampler {
    fn sample(&mut self, optimistic: f32, most_likely: f32, pessimistic: f32) -> Result<f32, ()>;
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
