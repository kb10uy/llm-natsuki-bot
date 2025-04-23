use rand::prelude::*;
use rand_distr::Normal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MenstruationConfiguration {
    pub cycle_mu_sigma: (f64, f64),
    pub bleeding_days: u16,
    pub ovulation_day: u16,
    pub pad_length_variations: Vec<usize>,
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
    Pad { length_centimeters: usize, has_wing: bool },
    Tampon,
    Cup,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MenstruationStatus {
    #[serde(skip_serializing)]
    pub phase: MensePhase,

    pub bleeding_days: Option<u16>,
    pub absorbent: Option<MenstruationAbsorbent>,
}

impl MenstruationConfiguration {
    pub fn calculate_cycles<R: RngCore + ?Sized>(&self, annual_rng: &mut R) -> Vec<(u16, u16)> {
        let cycle_distr = {
            let (mu, sigma) = self.cycle_mu_sigma;
            Normal::new(mu, sigma).expect("invalid distribution")
        };

        let mut next_starting_ordinal = 1;
        let mut starting_ordinals = vec![];
        while next_starting_ordinal <= 366 {
            let cycle_length = cycle_distr.sample(annual_rng).max(1.0).round_ties_even() as u16;
            starting_ordinals.push((next_starting_ordinal, cycle_length));
            next_starting_ordinal += cycle_length;
        }
        starting_ordinals
    }

    pub fn construct_status<R: RngCore + ?Sized>(
        &self,
        rng: &mut R,
        cycles: &[(u16, u16)],
        logical_ordinal: u16,
        day_progress: f64,
    ) -> MenstruationStatus {
        let (starting_ordinal, cycle_length) = cycles
            .iter()
            .filter(|so| logical_ordinal >= so.0)
            .max()
            .expect("at least one cycle");
        let cycle_days = logical_ordinal - starting_ordinal;

        let phase = if cycle_days < self.ovulation_day {
            let phase_progress = (cycle_days as f64 + day_progress) / self.ovulation_day as f64;
            MensePhase::Follicular(phase_progress)
        } else {
            let phase_length = (cycle_length - self.ovulation_day).max(1) as f64;
            let phase_progress = (cycle_days as f64 + day_progress - self.ovulation_day as f64) / phase_length;
            MensePhase::Luteal(phase_progress)
        };
        let bleeding_days = (cycle_days < self.bleeding_days).then_some(cycle_days + 1);

        let absorbent = {
            let length = self.pad_length_variations.choose(rng).unwrap_or(&0);
            let has_wing = rng.random();
            match (0..2).choose(rng).expect("variant error") {
                0 => MenstruationAbsorbent::Pad {
                    length_centimeters: *length,
                    has_wing,
                },
                1 => MenstruationAbsorbent::Tampon,
                2 => MenstruationAbsorbent::Cup,
                _ => unreachable!("invalid range"),
            }
        };

        MenstruationStatus {
            phase,
            bleeding_days,
            absorbent: bleeding_days.and(Some(absorbent)),
        }
    }
}
