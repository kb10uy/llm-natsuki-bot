use crate::{datetime::LogicalDay, menstruation::MensePhase, rng::SaltedRng};

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

/// その論理日について確定した基礎体温のジッター。
#[derive(Debug, Clone)]
pub struct TemperaturePlan {
    jitter: f64,
}

impl TemperatureConfiguration {
    /// 日替わりのジッターを決定する。
    pub fn plan(&self, rng: &mut SaltedRng<LogicalDay>) -> TemperaturePlan {
        let jitter_distr = {
            let (mu, sigma) = self.jitter_mu_sigma;
            Normal::new(mu, sigma).expect("invalid distribution")
        };
        TemperaturePlan {
            jitter: jitter_distr.sample(rng),
        }
    }

    /// baseline は時刻で変動するので観測フェーズで評価する。
    pub fn observe(&self, plan: &TemperaturePlan, phase: MensePhase) -> f64 {
        let canonical_t = self.canonicalize_t(phase);
        let base_value = self.baseline + self.scale * self.calculate_fourier(canonical_t);
        base_value + plan.jitter
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
