use std::ops::Range;

use rand::prelude::*;
use rand_distr::{Normal, Poisson, StandardUniform};
use serde::{Deserialize, Serialize};

// 理論上無限回出るので上限を決める
const TECHNO_BREAK_LIMIT: f64 = 12.0;
const MINUTES_PER_DAY: f64 = 24.0 * 60.0;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MasturbationConfiguration {
    pub duration_minutes_mu_sigma: (f64, f64),
    pub daily_count_lambda: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MasturbationStatus {
    pub completed_count: usize,
    pub playing_now: bool,
}

impl MasturbationConfiguration {
    pub fn get_playing_ranges<R: RngCore + ?Sized>(&self, rng: &mut R) -> Vec<Range<f64>> {
        let count_distr = Poisson::new(self.daily_count_lambda).expect("invalid range");
        let duration_distr = {
            let (mu, sigma) = self.duration_minutes_mu_sigma;
            Normal::new(mu, sigma).expect("invalid distribution")
        };

        let daily_total = count_distr.sample(rng).min(TECHNO_BREAK_LIMIT) as usize;
        let mut ranges: Vec<_> = (0..daily_total)
            .map(|_| {
                let timing: f64 = StandardUniform.sample(rng);
                let duration = duration_distr.sample(rng);
                timing..(timing + duration / MINUTES_PER_DAY)
            })
            .collect();
        ranges.sort_by(|lhs, rhs| lhs.start.partial_cmp(&rhs.start).expect("total order"));
        ranges
    }

    pub fn construct_status(&self, ranges: &[Range<f64>], day_progress: f64) -> MasturbationStatus {
        let completed_count = ranges.iter().filter(|mr| day_progress >= mr.end).count();
        let playing_now = ranges.iter().any(|mr| mr.contains(&day_progress));
        MasturbationStatus {
            completed_count,
            playing_now,
        }
    }
}
