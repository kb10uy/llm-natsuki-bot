use rand::prelude::*;
use rand_distr::Normal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TemperatureConfig {
    baseline: f64,
    scale: f64,
    jitter_mu_sigma: (f64, f64),
    fourier_coefficients: Vec<(f64, f64)>,
}

impl TemperatureConfig {
    pub fn calculate<R: RngCore + ?Sized>(&self, rng: &mut R, canonical_t: f64) {
        let base_value = self.baseline + self.scale * self.calculate_fourier(canonical_t);
    }

    pub fn calculate_fourier(&self, canonical_t: f64) -> f64 {
        self.fourier_coefficients
            .iter()
            .enumerate()
            .map(|(i, &(co_x, co_y))| (co_x, co_y, i as f64 * canonical_t))
            .fold(0.0, |a, (co_x, co_y, t)| a + co_x * t.cos() + co_y * t.sin())
    }
}
