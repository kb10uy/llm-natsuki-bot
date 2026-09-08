use crate::function::ConfigurableFunction;

use crate::config::tools::ConfigToolsDailyPrivate;
use futures::{FutureExt, future::BoxFuture};
use lnb_core::{
    context::Context,
    error::FunctionError,
    interface::{
        MessageContext,
        function::{Function, FunctionDescriptor, FunctionResponse},
    },
    model::{conversation::IncompleteConversation, message::MessageToolCalling, schema::DescribedSchema},
};
use lnb_daily_private::{
    day_routine::{DayRoutine, DayStep},
    masturbation::MasturbationStatus,
    menstruation::MenstruationStatus,
    plan::DailyPrivateConfiguration,
    underwear::UnderwearStatus,
};
use lnb_rate_limiter::RateLimiter;
use serde::Serialize;
use time::{
    Duration, OffsetDateTime, PrimitiveDateTime, Time,
    format_description::{BorrowedFormatItem, well_known::Rfc3339},
    macros::format_description,
};
use tracing::info;

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
    configuration: DailyPrivateConfiguration,
    descriptor: FunctionDescriptor,
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
        Ok(DailyPrivate {
            configuration: DailyPrivateConfiguration {
                rng_salt: config.daily_rng_salt.clone(),
                long_term_days: config.day_routine.long_term_days as usize,
                daytime_start,
                day_routine,
                schedule: config.schedule.clone(),
                menstruation: config.menstruation.clone(),
                temperature: config.temperature.clone(),
                masturbation: config.masturbation.clone(),
                underwear: config.underwear.clone(),
            },
            descriptor: FunctionDescriptor {
                name: "daily_private".to_string(),
                description: config.prompt.description.clone(),
                parameters: DescribedSchema::object("parameters", "引数", vec![]),
            },
        })
    }
}

impl Function for DailyPrivate {
    fn get_descriptor(&self) -> FunctionDescriptor {
        self.descriptor.clone()
    }

    fn call<'a>(
        &'a self,
        ctx: &'a Context,
        _message_ctx: &'a MessageContext,
        _incomplete: &'a IncompleteConversation,
        _tool_calling: MessageToolCalling,
    ) -> BoxFuture<'a, Result<FunctionResponse, FunctionError>> {
        async move { self.get_daily_info(ctx.datetime_provider.now()).await }.boxed()
    }
}

impl DailyPrivate {
    async fn get_daily_info(&self, now: OffsetDateTime) -> Result<FunctionResponse, FunctionError> {
        let moment = self
            .configuration
            .logical_moment(PrimitiveDateTime::new(now.date(), now.time()));
        let plan = self
            .configuration
            .plan_day(&moment.day)
            .map_err(FunctionError::by_external)?;
        let observation = self.configuration.observe(&plan, &moment);

        info!("logical: {moment:?}, step: {:?}", observation.day_step);
        info!("event: {:?}", plan.event);
        info!("menstruation: {:?}", observation.menstruation);
        info!("menstruation cycles: {:?}", plan.menstruation_cycles.ranges());
        info!("basal body temperature: {:.02}℃", observation.basal_body_temperature);
        info!(
            "masturbation: {} completed (playing now: {})",
            observation.masturbation.completed_count, observation.masturbation.playing_now
        );
        info!("masturbation planned: {:?}", plan.masturbation.ranges());
        info!("underwear status: {:?}", observation.underwear);

        let info = DailyPrivateInfo {
            asked_at: now.format(&Rfc3339).map_err(FunctionError::by_serialization)?,
            current_status: observation.day_step,
            menstruation_status: observation.menstruation,
            basal_body_temperature: format!("{:.02}", observation.basal_body_temperature),
            masturbation_status: observation.masturbation,
            underwear_status: observation.underwear,
        };
        Ok(FunctionResponse {
            result: serde_json::to_value(&info).map_err(FunctionError::by_serialization)?,
            attachments: vec![],
        })
    }
}
