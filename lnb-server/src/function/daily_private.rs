use crate::function::ConfigurableFunction;

use futures::{FutureExt, future::BoxFuture};
use lnb_common::{config::tools::ConfigToolsDailyPrivate, debug::debug_option_parsed};
use lnb_core::{
    error::FunctionError,
    interface::{
        Context,
        function::{Function, FunctionDescriptor, FunctionResponse},
    },
    model::{conversation::IncompleteConversation, message::MessageToolCalling, schema::DescribedSchema},
};
use lnb_daily_private::{
    datetime::LogicalDateTime,
    day_routine::{DayRoutine, DayStep},
    masturbation::{MasturbationConfiguration, MasturbationStatus},
    menstruation::{MenstruationConfiguration, MenstruationStatus},
    schedule::ScheduleConfiguration,
    temperature::TemperatureConfiguration,
    underwear::{UnderwearConfiguration, UnderwearStatus},
};
use lnb_rate_limiter::RateLimiter;
use rand::prelude::*;
use serde::Serialize;
use sha2::{Digest, Sha256};
use time::{
    Duration, OffsetDateTime, PrimitiveDateTime, Time,
    format_description::{BorrowedFormatItem, well_known::Rfc3339},
    macros::format_description,
};
use tracing::{info, warn};

const TIME_FORMAT: &[BorrowedFormatItem<'static>] = format_description!("[hour]:[minute]:[second]");

#[derive(Debug, Clone, Serialize)]
struct DailyPrivateInfo {
    asked_at: String,
    current_status: DayStep,
    menstruation_status: MenstruationStatus,
    basal_body_temperature: String,
    masturbation_status: MasturbationStatus,
    underwear_status: UnderwearStatus,
}

#[derive(Debug)]
pub struct DailyPrivate {
    rng_salt: String,
    long_term_days: usize,
    daytime_start: Time,
    day_routine: DayRoutine,
    schedule: ScheduleConfiguration,
    menstruation: MenstruationConfiguration,
    temperature: TemperatureConfiguration,
    masturbation: MasturbationConfiguration,
    underwear: UnderwearConfiguration,

    debug_offset: Duration,
}

impl ConfigurableFunction for DailyPrivate {
    const NAME: &'static str = stringify!(DailyPrivate);

    type Configuration = ConfigToolsDailyPrivate;

    async fn configure(config: &ConfigToolsDailyPrivate, _: Option<RateLimiter>) -> Result<Self, FunctionError> {
        let daytime_start =
            Time::parse(&config.day_routine.morning_start, TIME_FORMAT).map_err(FunctionError::by_serialization)?;
        let day_routine = DayRoutine::new(
            Duration::minutes(config.day_routine.daytime_minutes as i64),
            Duration::minutes(config.day_routine.morning_preparation_minutes as i64),
            Duration::minutes(config.day_routine.bathtime_minutes as i64),
        );
        let debug_offset = {
            let offset_days = debug_option_parsed("daily_private_offset")
                .map_err(FunctionError::by_serialization)?
                .unwrap_or(0);
            if offset_days != 0 {
                warn!("day offset: {offset_days}");
            }
            Duration::days(offset_days)
        };
        Ok(DailyPrivate {
            rng_salt: config.daily_rng_salt.clone(),
            long_term_days: config.day_routine.long_term_days as usize,
            daytime_start,
            day_routine,
            schedule: config.schedule.clone(),
            underwear: config.underwear.clone(),
            masturbation: config.masturbation.clone(),
            menstruation: config.menstruation.clone(),
            temperature: config.temperature.clone(),

            debug_offset,
        })
    }
}

impl Function for DailyPrivate {
    fn get_descriptor(&self) -> FunctionDescriptor {
        FunctionDescriptor {
            name: "daily_private".to_string(),
            description: r#"
                この bot 自身のその日のプライベートな事情を取得します。
                以下のいずれかの項目についてユーザーから訊かれた場合、この function のレスポンスをもとに適切に回答してください。
                訊かれていない項目については答えなくてかまいません。
                - 今の行動状態
                - 生理周期
                - 基礎体温
                - その日のオナニーの回数
                - 下着の色
            "#
            .to_string(),
            parameters: DescribedSchema::object("parameters", "引数", vec![]),
        }
    }

    fn call<'a>(
        &'a self,
        _context: &'a Context,
        _incomplete: &'a IncompleteConversation,
        _tool_calling: MessageToolCalling,
    ) -> BoxFuture<'a, Result<FunctionResponse, FunctionError>> {
        async move { self.get_daily_info().await }.boxed()
    }
}

impl DailyPrivate {
    async fn get_daily_info(&self) -> Result<FunctionResponse, FunctionError> {
        let now = OffsetDateTime::now_local().map_err(FunctionError::by_external)? + self.debug_offset;
        let logical_datetime = LogicalDateTime::calculate(
            PrimitiveDateTime::new(now.date(), now.time()),
            self.daytime_start,
            self.long_term_days,
        );
        let day_step = self.day_routine.calculate_day_step(&logical_datetime);
        info!("logical: {logical_datetime:?}, step: {day_step:?}");

        let mut daily_rng = self.make_salted_rng(logical_datetime.logical_julian_day.to_le_bytes());
        let mut long_term_rng = self.make_salted_rng(logical_datetime.long_term_cycles.to_le_bytes());

        // スケジュール
        let event = self
            .schedule
            .choose_event(&mut daily_rng, logical_datetime.logical_date);
        info!("event: {event:?}");

        // 生理周期
        let menstruation_cycles = self
            .menstruation
            .calculate_cycles(&mut long_term_rng, self.long_term_days as u64)
            .map_err(FunctionError::by_external)?;
        let menstruation_status =
            self.menstruation
                .construct_status(&mut daily_rng, &menstruation_cycles, &logical_datetime, event);
        info!("menstruation: {menstruation_status:?}");
        info!("menstruation cycles: {menstruation_cycles:?}");

        // 基礎体温
        let basal_body_temperature = self.temperature.calculate(&mut daily_rng, menstruation_status.phase);
        info!("basal body temperature: {basal_body_temperature:.02}℃");

        // オナニー
        let masturbation_ranges = self.masturbation.calculate_daily_playing_ranges(
            &mut daily_rng,
            menstruation_status.bleeding_days,
            &logical_datetime,
        );
        let (masturbation_status, current_play) = self
            .masturbation
            .construct_status_progress(&masturbation_ranges, logical_datetime.day_progress);
        info!(
            "masturbation: {} completed (current play: {current_play:?})",
            masturbation_status.completed_count
        );
        info!("masturbation planned: {masturbation_ranges:?}");

        // 下着
        let underwear_status =
            self.underwear
                .generate_status(&mut daily_rng, day_step, &menstruation_status.absorbent, current_play);
        info!("underwear status: {underwear_status:?}");

        let info = DailyPrivateInfo {
            asked_at: now.format(&Rfc3339).map_err(FunctionError::by_serialization)?,
            current_status: day_step,
            menstruation_status,
            basal_body_temperature: format!("{basal_body_temperature:.02}"),
            masturbation_status,
            underwear_status,
        };
        Ok(FunctionResponse {
            result: serde_json::to_value(&info).map_err(FunctionError::by_serialization)?,
            attachments: vec![],
        })
    }

    fn make_salted_rng(&self, seed_bytes: impl AsRef<[u8]>) -> StdRng {
        let mut hasher = Sha256::new();
        hasher.update(&self.rng_salt);
        hasher.update(seed_bytes);
        StdRng::from_seed(hasher.finalize().into())
    }
}
