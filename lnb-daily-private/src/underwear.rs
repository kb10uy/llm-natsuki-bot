use crate::{day_routine::DayStep, menstruation::MenstruationAbsorbent};

use rand::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UnderwearConfiguration {
    pub separate_colors: Vec<String>,
    pub separate_designs: Vec<String>,
    pub no_bra_ratio: f64,
    pub no_panty_ratio: f64,
    pub unified_ratio: f64,
    pub no_wear_reasons: Vec<String>,
    pub masturbating_reason: String,
    pub bathtime_reason: String,
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
        let no_bp_reason = self.no_wear_reasons.choose(rng).map(|s| &s[..]).unwrap_or_default();

        if matches!(masturbation_progress, Some(p) if p >= 0.5) {
            // オナニーの進行度が半分以上なら常に全脱ぎ
            return UnderwearStatus::NoBraNoPanty {
                reason: self.masturbating_reason.clone(),
            };
        } else if day_step == DayStep::Bathtime {
            // 風呂なのでもちろん脱ぐ
            return UnderwearStatus::NoBraNoPanty {
                reason: self.bathtime_reason.clone(),
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
                no_panty_reason: no_bp_reason.to_string(),
            },
            // ノーパン
            (_, true, false) => UnderwearStatus::PantyOnly {
                no_bra_reason: no_bp_reason.to_string(),
                panty_design,
                is_sanitary,
            },
            // ノーブラノーパン
            (_, true, true) => UnderwearStatus::NoBraNoPanty {
                reason: no_bp_reason.to_string(),
            },
        }
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
