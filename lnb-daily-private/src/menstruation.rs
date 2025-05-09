use crate::DailyPrivateError;

use std::ops::Range;

use rand::prelude::*;
use rand_distr::Normal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MenstruationConfiguration {
    pub cycle_mu_sigma: (u64, f64),
    pub bleeding_days: i64,
    pub ovulation_day: i64,
    pub pad_variations: Vec<PadVariation>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PadVariation {
    pub length_centimeters: f64,
    pub has_wing: bool,
    pub thickness: PadThickness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Hash)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum PadThickness {
    VeryThin,
    Thin,
    Normal,
    Thick,
    VeryThick,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "phase", content = "progress")]
pub enum MensePhase {
    /// 卵胞期
    Follicular(f64),

    /// 黄体期
    Luteal(f64),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "details")]
pub enum MenstruationAbsorbent {
    Pad(PadVariation),
    Tampon,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MenstruationStatus {
    #[serde(skip_serializing)]
    pub phase: MensePhase,

    pub bleeding_days: Option<i64>,
    pub absorbent: Option<MenstruationAbsorbent>,
}

impl MenstruationConfiguration {
    pub fn calculate_cycles<R: RngCore + ?Sized>(
        &self,
        long_term_rng: &mut R,
        long_term_days: u64,
    ) -> Result<Vec<Range<i64>>, DailyPrivateError> {
        // 長期収束のために割り切れれて正の商になる必要がある
        if long_term_days % self.cycle_mu_sigma.0 != 0 || long_term_days < self.cycle_mu_sigma.0 {
            return Err(DailyPrivateError::LongTermMismatch);
        }
        let long_term_cycles = (long_term_days / self.cycle_mu_sigma.0) as usize;

        // ジッターの各周期後の総和が 2σ を超えないように生成
        let jitter_distr = Normal::new(0.0, self.cycle_mu_sigma.1).expect("invalid distribution");
        let jitter_limit = (self.cycle_mu_sigma.1 * 2.0).round() as i64;
        let mut jitters = vec![0i64; long_term_cycles];
        let mut jitter_sum = 0i64;
        for jitter in &mut jitters {
            let jitter_candidate = jitter_distr.sample(long_term_rng).round() as i64;
            let clamped_jitter = jitter_candidate.clamp(-jitter_limit - jitter_sum, jitter_limit - jitter_sum);
            *jitter = clamped_jitter;
            jitter_sum += clamped_jitter;
        }
        // 最後だけ合わせる
        jitters[long_term_cycles - 1] -= jitter_sum;

        let cycles = jitters
            .iter()
            .map(|j| j + self.cycle_mu_sigma.0 as i64)
            .scan(0i64, |prev_end, length| {
                let range = *prev_end..(*prev_end + length);
                *prev_end += length;
                Some(range)
            })
            .collect();
        Ok(cycles)
    }

    pub fn construct_status<R: RngCore + ?Sized>(
        &self,
        rng: &mut R,
        cycles: &[Range<i64>],
        logical_in_long_term: i64,
        day_progress: f64,
    ) -> MenstruationStatus {
        let cycle_range = cycles
            .iter()
            .find(|r| r.contains(&logical_in_long_term))
            .expect("invalid cycles");
        let cycle_length = cycle_range.end - cycle_range.start;
        let cycle_days = logical_in_long_term - cycle_range.start;

        let phase = if cycle_days < self.ovulation_day {
            let phase_progress = (cycle_days as f64 + day_progress) / self.ovulation_day as f64;
            MensePhase::Follicular(phase_progress)
        } else {
            let phase_length = (cycle_length - self.ovulation_day).max(1) as f64;
            let phase_progress = (cycle_days as f64 + day_progress - self.ovulation_day as f64) / phase_length;
            MensePhase::Luteal(phase_progress)
        };
        let bleeding_days = (cycle_days < self.bleeding_days).then_some(cycle_days + 1);

        let absorbent = self.choose_absorbent(rng);

        MenstruationStatus {
            phase,
            bleeding_days,
            absorbent: bleeding_days.and(absorbent),
        }
    }

    fn choose_absorbent<R: RngCore + ?Sized>(&self, rng: &mut R) -> Option<MenstruationAbsorbent> {
        let pad_variation = self.pad_variations.choose(rng);
        let should_use_tampon = self.should_use_tampon(rng);
        if should_use_tampon {
            return Some(MenstruationAbsorbent::Tampon);
        }
        pad_variation.cloned().map(MenstruationAbsorbent::Pad)
    }

    fn should_use_tampon<R: RngCore + ?Sized>(&self, _rng: &mut R) -> bool {
        // TODO: なんか作る
        false
    }
}
