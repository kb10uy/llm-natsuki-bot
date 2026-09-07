use crate::{datetime::LogicalDay, day_routine::DayStep, menstruation::MenstruationAbsorbent, rng::SaltedRng};

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

/// その論理日について確定した下着の選択。
#[derive(Debug, Clone)]
pub struct UnderwearPlan {
    /// 選ばれた下着。設定に下着がひとつもない場合は `None`。
    choice: Option<UnderwearChoice>,
}

/// 着る下着と、脱ぐ場合の理由。
#[derive(Debug, Clone)]
struct UnderwearChoice {
    bra_design: UnderwearDesign,
    panty_design: UnderwearDesign,
    unified: bool,
    no_bra: bool,
    no_panty: bool,
    reasons: UnwornReasons,
}

/// 用途ごとに確定した「着てない理由」。
#[derive(Debug, Clone)]
pub struct UnwornReasons {
    pub masturbating: String,
    pub bathtime: String,
    pub no_bra: String,
    pub no_panty: String,
    pub naked: String,
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
    /// その論理日の下着と、脱ぐ場合の理由をすべて決定する。
    pub fn plan(&self, rng: &mut SaltedRng<LogicalDay>) -> UnderwearPlan {
        let (bra_design, panty_design) = match (self.generate_part(rng), self.generate_part(rng)) {
            (Some(c1), Some(c2)) => (c1, c2),
            _ => return UnderwearPlan { choice: None },
        };

        let unified = rng.random::<f64>() < self.unified_ratio;
        let no_bra = rng.random::<f64>() < self.no_bra_ratio;
        let no_panty = rng.random::<f64>() < self.no_panty_ratio;

        let reasons = UnwornReasons {
            masturbating: self
                .choose_unworn_reason(rng, UnwornReasonUsage::Masturbating)
                .to_string(),
            bathtime: self.choose_unworn_reason(rng, UnwornReasonUsage::Bathtime).to_string(),
            no_bra: self.choose_unworn_reason(rng, UnwornReasonUsage::NoBra).to_string(),
            no_panty: self.choose_unworn_reason(rng, UnwornReasonUsage::NoPanty).to_string(),
            naked: self.choose_unworn_reason(rng, UnwornReasonUsage::Naked).to_string(),
        };

        UnderwearPlan {
            choice: Some(UnderwearChoice {
                bra_design,
                panty_design,
                unified,
                no_bra,
                no_panty,
                reasons,
            }),
        }
    }

    /// 確定済みの選択に時刻由来の状況を当てはめて着用状態を求める。
    pub fn observe(
        &self,
        plan: &UnderwearPlan,
        day_step: DayStep,
        absorbent: Option<&MenstruationAbsorbent>,
        masturbation_progress: Option<f64>,
    ) -> UnderwearStatus {
        let Some(UnderwearChoice {
            bra_design,
            panty_design,
            unified,
            no_bra,
            no_panty,
            reasons,
        }) = &plan.choice
        else {
            return UnderwearStatus::NoBraNoPanty {
                reason: "下着を持ってない".to_string(),
            };
        };

        if matches!(masturbation_progress, Some(p) if p >= 0.5) {
            // オナニーの進行度が半分以上なら常に全脱ぎ
            return UnderwearStatus::NoBraNoPanty {
                reason: reasons.masturbating.clone(),
            };
        } else if day_step == DayStep::Bathtime {
            // 風呂なのでもちろん脱ぐ
            return UnderwearStatus::NoBraNoPanty {
                reason: reasons.bathtime.clone(),
            };
        }

        let is_sanitary = matches!(absorbent, Some(MenstruationAbsorbent::Pad { .. }));
        match (*unified, *no_bra, *no_panty) {
            // 両方セット
            (true, false, false) => UnderwearStatus::IntegratedDesignBraAndPanty {
                design: bra_design.clone(),
                is_sanitary,
            },
            // 両方別々
            (false, false, false) => UnderwearStatus::SeparateBraAndPanty {
                bra_design: bra_design.clone(),
                panty_design: panty_design.clone(),
                is_sanitary,
            },
            // ノーブラ
            (_, false, true) => UnderwearStatus::BraOnly {
                bra_design: bra_design.clone(),
                no_panty_reason: reasons.no_panty.clone(),
            },
            // ノーパン
            (_, true, false) => UnderwearStatus::PantyOnly {
                no_bra_reason: reasons.no_bra.clone(),
                panty_design: panty_design.clone(),
                is_sanitary,
            },
            // ノーブラノーパン
            (_, true, true) => UnderwearStatus::NoBraNoPanty {
                reason: reasons.naked.clone(),
            },
        }
    }

    fn choose_unworn_reason(&self, rng: &mut SaltedRng<LogicalDay>, usage: UnwornReasonUsage) -> &str {
        let chosen_reason = self
            .unworn_reasons
            .iter()
            .filter(|r| r.usage.contains(&usage))
            .choose(rng);
        chosen_reason.map(|r| r.text.as_str()).unwrap_or_default()
    }

    fn generate_part(&self, rng: &mut SaltedRng<LogicalDay>) -> Option<UnderwearDesign> {
        let color = self.separate_colors.choose(rng)?;
        let design = self.separate_designs.choose(rng)?;
        Some(UnderwearDesign {
            color: color.clone(),
            pattern: design.clone(),
        })
    }
}
