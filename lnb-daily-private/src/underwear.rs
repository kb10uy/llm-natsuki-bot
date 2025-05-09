use crate::{day_routine::DayStep, menstruation::MenstruationAbsorbent};

use std::collections::HashSet;

use rand::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UnderwearConfiguration {
    pub separate_colors: Vec<String>,
    pub separate_designs: Vec<String>,
    pub no_bra_ratio: f64,
    pub no_panty_ratio: f64,
    pub unified_ratio: f64,
    pub unworn_reasons: Vec<UnwornReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct UnwornReason {
    pub text: String,
    pub usage: HashSet<UnwornReasonUsage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnwornReasonUsage {
    NoBra,
    NoPanty,
    Naked,
    Bathtime,
    Masturbating,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "status", content = "details")]
pub enum UnderwearStatus {
    /// セットの下着を着ている
    IntegratedDesignBraAndPanty { design: UnderwearDesign, is_sanitary: bool },

    /// 別々のを着ている
    SeparateBraAndPanty {
        bra_design: UnderwearDesign,
        panty_design: UnderwearDesign,
        is_sanitary: bool,
    },

    /// ノーパン
    BraOnly {
        bra_design: UnderwearDesign,
        no_panty_reason: String,
    },

    /// ノーブラ
    PantyOnly {
        no_bra_reason: String,
        panty_design: UnderwearDesign,
        is_sanitary: bool,
    },

    /// ノーガード
    NoBraNoPanty { reason: String },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UnderwearDesign {
    pub color: String,
    pub pattern: String,
}

impl UnderwearConfiguration {
    pub fn generate_status<R: RngCore + ?Sized>(
        &self,
        rng: &mut R,
        day_step: DayStep,
        absorbent: &Option<MenstruationAbsorbent>,
        masturbation_progress: Option<f64>,
    ) -> UnderwearStatus {
        let (bra_design, panty_design) = match (self.generate_part(rng), self.generate_part(rng)) {
            (Some(c1), Some(c2)) => (c1, c2),
            _ => {
                return UnderwearStatus::NoBraNoPanty {
                    reason: "下着を持ってない".to_string(),
                };
            }
        };

        let unified = rng.random::<f64>() < self.unified_ratio;
        let no_bra = rng.random::<f64>() < self.no_bra_ratio;
        let no_panty = rng.random::<f64>() < self.no_panty_ratio;

        let masturbating_reason = self.choose_unworn_reason(rng, UnwornReasonUsage::Masturbating);
        let bathtime_reason = self.choose_unworn_reason(rng, UnwornReasonUsage::Bathtime);
        let no_bra_reason = self.choose_unworn_reason(rng, UnwornReasonUsage::NoBra);
        let no_panty_reason = self.choose_unworn_reason(rng, UnwornReasonUsage::NoPanty);
        let naked_reason = self.choose_unworn_reason(rng, UnwornReasonUsage::Naked);

        if matches!(masturbation_progress, Some(p) if p >= 0.5) {
            // オナニーの進行度が半分以上なら常に全脱ぎ
            return UnderwearStatus::NoBraNoPanty {
                reason: masturbating_reason.to_string(),
            };
        } else if day_step == DayStep::Bathtime {
            // 風呂なのでもちろん脱ぐ
            return UnderwearStatus::NoBraNoPanty {
                reason: bathtime_reason.to_string(),
            };
        }

        let is_sanitary = matches!(absorbent, Some(MenstruationAbsorbent::Pad { .. }));
        match (unified, no_bra, no_panty) {
            // 両方セット
            (true, false, false) => UnderwearStatus::IntegratedDesignBraAndPanty {
                design: bra_design,
                is_sanitary,
            },
            // 両方別々
            (false, false, false) => UnderwearStatus::SeparateBraAndPanty {
                bra_design,
                panty_design,
                is_sanitary,
            },
            // ノーブラ
            (_, false, true) => UnderwearStatus::BraOnly {
                bra_design,
                no_panty_reason: no_panty_reason.to_string(),
            },
            // ノーパン
            (_, true, false) => UnderwearStatus::PantyOnly {
                no_bra_reason: no_bra_reason.to_string(),
                panty_design,
                is_sanitary,
            },
            // ノーブラノーパン
            (_, true, true) => UnderwearStatus::NoBraNoPanty {
                reason: naked_reason.to_string(),
            },
        }
    }

    fn choose_unworn_reason<R: RngCore + ?Sized>(&self, rng: &mut R, usage: UnwornReasonUsage) -> &str {
        let chosen_reason = self
            .unworn_reasons
            .iter()
            .filter(|r| r.usage.contains(&usage))
            .choose(rng);
        chosen_reason.map(|r| r.text.as_str()).unwrap_or_default()
    }

    fn generate_part<R: RngCore + ?Sized>(&self, rng: &mut R) -> Option<UnderwearDesign> {
        let color = self.separate_colors.choose(rng)?;
        let design = self.separate_designs.choose(rng)?;
        Some(UnderwearDesign {
            color: color.clone(),
            pattern: design.clone(),
        })
    }
}
