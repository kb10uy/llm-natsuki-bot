use std::ops::{Not, Range};

use rand::prelude::*;
use rand_distr::{Normal, Poisson, StandardUniform};
use serde::{Deserialize, Serialize};
use time::Weekday;

use crate::logical_date::LogicalDateTime;

// 理論上無限回出るので上限を決める
const TECHNO_BREAK_LIMIT: f64 = 12.0;
const MINUTES_PER_DAY: f64 = 24.0 * 60.0;
const MIN_LAMBDA: f64 = 1.0;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MasturbationConfiguration {
    pub duration_minutes_mu_sigma: (f64, f64),
    pub daily_count_lambda: f64,
    pub holiday_boost_scale: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MasturbationStatus {
    pub completed_count: usize,

    // 殊更にしてないのを強調してほしいわけじゃないので true のときだけシリアライズする
    #[serde(skip_serializing_if = "<&bool>::not")]
    pub playing_now: bool,
}

impl MasturbationConfiguration {
    pub fn calculate_daily_playing_ranges<R: RngCore + ?Sized>(
        &self,
        rng: &mut R,
        bleeding_days: Option<usize>,
        logical_datetime: &LogicalDateTime,
    ) -> Vec<Range<f64>> {
        let total_lambda = {
            let bleeding_debuff = bleeding_days
                .map(|days| 1.0 - (1.0 / days.max(1) as f64))
                .unwrap_or(1.0);
            let holiday_boost = match logical_datetime.weekday() {
                Weekday::Saturday | Weekday::Sunday => self.holiday_boost_scale,
                _ => 1.0,
            };
            (self.daily_count_lambda * bleeding_debuff * holiday_boost).max(MIN_LAMBDA)
        };
        let count_distr = Poisson::new(total_lambda).expect("invalid range");
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

    pub fn construct_status_progress(
        &self,
        ranges: &[Range<f64>],
        day_progress: f64,
    ) -> (MasturbationStatus, Option<f64>) {
        let completed_count = ranges.iter().filter(|mr| day_progress >= mr.end).count();
        let current_play = ranges
            .iter()
            .filter_map(|mr| {
                mr.contains(&day_progress)
                    .then_some((day_progress - mr.start) / (mr.end - mr.start))
            })
            .next();
        (
            MasturbationStatus {
                completed_count,
                playing_now: current_play.is_some(),
            },
            current_play,
        )
    }
}
