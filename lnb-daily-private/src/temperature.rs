use crate::menstruation::MensePhase;

use std::f64::consts::TAU;

use rand::prelude::*;
use rand_distr::Normal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TemperatureConfiguration {
    baseline: f64,
    scale: f64,
    jitter_mu_sigma: (f64, f64),
    fourier_coefficients: Vec<(f64, f64)>,
    ovulation_t: f64,
}

impl TemperatureConfiguration {
    pub fn calculate<R: RngCore + ?Sized>(&self, rng: &mut R, phase: MensePhase) -> f64 {
        // baseline は時刻で変動するけど jitter は日替わり
        let jitter_distr = {
            let (mu, sigma) = self.jitter_mu_sigma;
            Normal::new(mu, sigma).expect("invalid distribution")
        };

        let canonical_t = self.canonicalize_t(phase);
        let base_value = self.baseline + self.scale * self.calculate_fourier(canonical_t);
        let jitter = jitter_distr.sample(rng);
        base_value + jitter
    }

    fn canonicalize_t(&self, phase: MensePhase) -> f64 {
        match phase {
            MensePhase::Follicular(p) => p * self.ovulation_t,
            MensePhase::Luteal(p) => self.ovulation_t + p * (1.0 - self.ovulation_t),
        }
    }

    fn calculate_fourier(&self, canonical_t: f64) -> f64 {
        self.fourier_coefficients
            .iter()
            .enumerate()
            .map(|(i, &(co_x, co_y))| (co_x, co_y, i as f64 * canonical_t * TAU))
            .fold(0.0, |a, (co_x, co_y, t)| a + co_x * t.cos() + co_y * t.sin())
    }
}
