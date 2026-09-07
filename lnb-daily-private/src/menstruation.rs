use crate::{
    DailyPrivateError,
    datetime::{LogicalDay, LongTermCycle},
    rng::{RngDomain, RngSource, SaltedRng},
    schedule::HolidayEvent,
};

use std::ops::Range;

use rand::prelude::*;
use rand_distr::Normal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MenstruationConfiguration {
    pub cycle_mu_sigma: (u64, f64),
    pub bleeding_days: usize,
    pub ovulation_day: usize,
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
    Tampon { due_to_event: String },
}

/// 長周期ぶんの生理周期の区切り。
#[derive(Debug, Clone)]
pub struct MenstruationCycles(Vec<Range<usize>>);

/// その論理日について確定した生理の状態。時刻に依存する値を含まない。
#[derive(Debug, Clone)]
pub struct MenstruationPlan {
    /// 周期内での経過日数。
    pub cycle_days: usize,

    /// 属する周期の長さ。
    pub cycle_length: usize,

    pub bleeding_days: Option<usize>,
    pub absorbent: Option<MenstruationAbsorbent>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MenstruationStatus {
    #[serde(skip_serializing)]
    pub phase: MensePhase,

    pub bleeding_days: Option<usize>,
    pub absorbent: Option<MenstruationAbsorbent>,
}

impl MenstruationCycles {
    pub fn ranges(&self) -> &[Range<usize>] {
        &self.0
    }

    fn find(&self, long_term_days: usize) -> &Range<usize> {
        self.0
            .iter()
            .find(|r| r.contains(&long_term_days))
            .expect("invalid cycles")
    }
}

impl MenstruationConfiguration {
    /// 長周期ぶんの周期区切りを決定する。
    pub fn plan_cycles(
        &self,
        source: &RngSource<LongTermCycle>,
        long_term: &LongTermCycle,
    ) -> Result<MenstruationCycles, DailyPrivateError> {
        let rng = &mut source.derive(RngDomain::Menstruation);
        let long_term_duration = long_term.span_days as u64;

        // 長期収束のために割り切れれて正の商になる必要がある
        if !long_term_duration.is_multiple_of(self.cycle_mu_sigma.0) || long_term_duration < self.cycle_mu_sigma.0 {
            return Err(DailyPrivateError::LongTermMismatch);
        }
        let long_term_cycles = (long_term_duration / self.cycle_mu_sigma.0) as usize;

        // ジッターの各周期後の総和が 2σ を超えないように生成
        let jitter_distr = Normal::new(0.0, self.cycle_mu_sigma.1).expect("invalid distribution");
        let jitter_limit = (self.cycle_mu_sigma.1 * 2.0).round() as i64;
        let mut jitters = vec![0i64; long_term_cycles];
        let mut jitter_sum = 0i64;
        for jitter in &mut jitters {
            let jitter_candidate = jitter_distr.sample(rng).round() as i64;
            let clamped_jitter = jitter_candidate.clamp(-jitter_limit - jitter_sum, jitter_limit - jitter_sum);
            *jitter = clamped_jitter;
            jitter_sum += clamped_jitter;
        }
        // 最後だけ合わせる
        jitters[long_term_cycles - 1] -= jitter_sum;

        let cycles = jitters
            .iter()
            .map(|j| (j + self.cycle_mu_sigma.0 as i64) as usize)
            .scan(0, |prev_end, length| {
                let range = *prev_end..(*prev_end + length);
                *prev_end += length;
                Some(range)
            })
            .collect();
        Ok(MenstruationCycles(cycles))
    }

    /// その論理日の生理の状態を決定する。
    pub fn plan(
        &self,
        source: &RngSource<LogicalDay>,
        cycles: &MenstruationCycles,
        day: &LogicalDay,
        event: Option<&HolidayEvent>,
    ) -> MenstruationPlan {
        let rng = &mut source.derive(RngDomain::Menstruation);
        let cycle_range = cycles.find(day.long_term_days);
        let cycle_length = cycle_range.end - cycle_range.start;
        let cycle_days = day.long_term_days - cycle_range.start;
        let bleeding_days = (cycle_days < self.bleeding_days).then_some(cycle_days + 1);

        let absorbent = self.choose_absorbent(rng, event);

        MenstruationPlan {
            cycle_days,
            cycle_length,
            bleeding_days,
            absorbent: bleeding_days.and(absorbent),
        }
    }

    /// 確定済みの状態に時刻を当てはめて周期の進行度を求める。
    pub fn observe(&self, plan: &MenstruationPlan, day_progress: f64) -> MenstruationStatus {
        let phase = if plan.cycle_days < self.ovulation_day {
            let phase_progress = (plan.cycle_days as f64 + day_progress) / self.ovulation_day as f64;
            MensePhase::Follicular(phase_progress)
        } else {
            let phase_length = (plan.cycle_length - self.ovulation_day).max(1) as f64;
            let phase_progress = (plan.cycle_days as f64 + day_progress - self.ovulation_day as f64) / phase_length;
            MensePhase::Luteal(phase_progress)
        };

        MenstruationStatus {
            phase,
            bleeding_days: plan.bleeding_days,
            absorbent: plan.absorbent.clone(),
        }
    }

    fn choose_absorbent(
        &self,
        rng: &mut SaltedRng<LogicalDay>,
        event: Option<&HolidayEvent>,
    ) -> Option<MenstruationAbsorbent> {
        let pad_variation = self.pad_variations.choose(rng);

        let Some(event) = event else {
            return pad_variation.cloned().map(MenstruationAbsorbent::Pad);
        };
        if event.tampon_required {
            Some(MenstruationAbsorbent::Tampon {
                due_to_event: event.title.to_string(),
            })
        } else {
            pad_variation.cloned().map(MenstruationAbsorbent::Pad)
        }
    }
}
