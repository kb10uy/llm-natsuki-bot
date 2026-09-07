use crate::{
    DailyPrivateError,
    datetime::{LogicalDay, LogicalMoment},
    day_routine::{DayRoutine, DayStep},
    masturbation::{MasturbationConfiguration, MasturbationPlan, MasturbationStatus},
    menstruation::{MenstruationConfiguration, MenstruationCycles, MenstruationPlan, MenstruationStatus},
    rng::SaltedRng,
    schedule::{HolidayEvent, ScheduleConfiguration},
    temperature::{TemperatureConfiguration, TemperaturePlan},
    underwear::{UnderwearConfiguration, UnderwearPlan, UnderwearStatus},
};

use time::{PrimitiveDateTime, Time};

#[derive(Debug, Clone)]
pub struct DailyPrivateConfiguration {
    pub rng_salt: String,
    pub long_term_days: usize,
    pub daytime_start: Time,
    pub day_routine: DayRoutine,
    pub schedule: ScheduleConfiguration,
    pub menstruation: MenstruationConfiguration,
    pub temperature: TemperatureConfiguration,
    pub masturbation: MasturbationConfiguration,
    pub underwear: UnderwearConfiguration,
}

/// 論理日だけで決まる、その日ぶんの確定した予定。
#[derive(Debug, Clone)]
pub struct DayPlan {
    pub event: Option<HolidayEvent>,
    pub menstruation_cycles: MenstruationCycles,
    pub menstruation: MenstruationPlan,
    pub temperature: TemperaturePlan,
    pub masturbation: MasturbationPlan,
    pub underwear: UnderwearPlan,
}

/// [`DayPlan`] を特定の時刻で観測した結果。
#[derive(Debug, Clone)]
pub struct DayObservation {
    pub day_step: DayStep,
    pub menstruation: MenstruationStatus,
    pub basal_body_temperature: f64,
    pub masturbation: MasturbationStatus,
    pub underwear: UnderwearStatus,
}

impl DailyPrivateConfiguration {
    pub fn logical_moment(&self, local_now: PrimitiveDateTime) -> LogicalMoment {
        LogicalMoment::calculate(local_now, self.daytime_start, self.long_term_days)
    }

    /// 計画フェーズ。乱数を引くのはこの関数の内側だけで、入力は [`LogicalDay`] しかない。
    /// したがって乱数の消費列は論理日だけの関数になり、日より細かい要素では変化しない。
    pub fn plan_day(&self, day: &LogicalDay) -> Result<DayPlan, DailyPrivateError> {
        let mut long_term_rng = SaltedRng::new(&self.rng_salt, &day.long_term);
        let mut daily_rng = SaltedRng::new(&self.rng_salt, day);

        let event = self.schedule.plan(&mut daily_rng, day).cloned();
        let menstruation_cycles = self.menstruation.plan_cycles(&mut long_term_rng, &day.long_term)?;
        let menstruation = self
            .menstruation
            .plan(&mut daily_rng, &menstruation_cycles, day, event.as_ref());
        let temperature = self.temperature.plan(&mut daily_rng);
        let masturbation = self.masturbation.plan(&mut daily_rng, day, menstruation.bleeding_days);
        let underwear = self.underwear.plan(&mut daily_rng);

        Ok(DayPlan {
            event,
            menstruation_cycles,
            menstruation,
            temperature,
            masturbation,
            underwear,
        })
    }

    /// 観測フェーズ。時刻を含む [`LogicalMoment`] を使うが、乱数生成器がスコープに存在しない。
    pub fn observe(&self, plan: &DayPlan, moment: &LogicalMoment) -> DayObservation {
        let day_step = self.day_routine.observe(moment);
        let menstruation = self.menstruation.observe(&plan.menstruation, moment.day_progress);
        let basal_body_temperature = self.temperature.observe(&plan.temperature, menstruation.phase);
        let (masturbation, current_play) = self.masturbation.observe(&plan.masturbation, moment.day_progress);
        let underwear =
            self.underwear
                .observe(&plan.underwear, day_step, menstruation.absorbent.as_ref(), current_play);

        DayObservation {
            day_step,
            menstruation,
            basal_body_temperature,
            masturbation,
            underwear,
        }
    }
}
