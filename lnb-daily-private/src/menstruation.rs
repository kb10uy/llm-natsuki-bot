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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum MensePhase {
    /// 卵胞期
    Follicular,

    /// 黄体期
    Luteal,
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
    pub phase: MensePhase,
    pub bleeding_days: Option<u16>,
    pub absorbent: Option<MenstruationAbsorbent>,
}

impl MenstruationConfiguration {
    pub fn calculate_cycle_starting_ordinals<R: RngCore + ?Sized>(&self, annual_rng: &mut R) -> Vec<u16> {
        let cycle_distr = {
            let (mu, sigma) = self.cycle_mu_sigma;
            Normal::new(mu, sigma).expect("invalid distribution")
        };

        let mut next_starting_ordinal = 1;
        let mut starting_ordinals = vec![];
        while next_starting_ordinal <= 366 {
            starting_ordinals.push(next_starting_ordinal);
            next_starting_ordinal += cycle_distr.sample(annual_rng).min(1.0) as u16;
        }
        starting_ordinals
    }

    pub fn construct_status<R: RngCore + ?Sized>(
        &self,
        rng: &mut R,
        starting_ordinals: &[u16],
        logical_ordinal: u16,
    ) -> MenstruationStatus {
        let starting_ordinal = starting_ordinals
            .iter()
            .filter(|so| logical_ordinal >= **so)
            .max()
            .expect("at least one cycle");
        let cycle_days = logical_ordinal - starting_ordinal;

        let phase = if cycle_days < self.ovulation_day {
            MensePhase::Follicular
        } else {
            MensePhase::Luteal
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
