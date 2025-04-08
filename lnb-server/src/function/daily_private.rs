use crate::function::{ConfigurableSimpleFunction, extract_time_from_toml};

use futures::{FutureExt, future::BoxFuture};
use lnb_core::{
    error::FunctionError,
    interface::function::simple::{SimpleFunction, SimpleFunctionDescriptor, SimpleFunctionResponse},
    model::schema::DescribedSchema,
};
use lnb_daily_private::{
    day_routine::{DayRoutineConfiguration, DayStep},
    masturbation::{MasturbationConfiguration, MasturbationStatus},
    underwear::{UnderwearConfiguration, UnderwearStatus},
};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use time::{Duration, OffsetDateTime, PrimitiveDateTime, format_description::well_known::Rfc3339};
use toml::value::Datetime as TomlDateTime;
use tracing::info;

// TOML は Time だけを書けるが toml::Time は from str しかできないので TomlDateTime で拾う
#[derive(Debug, Clone, Deserialize)]
pub struct DailyPrivateConfig {
    daily_rng_salt: String,
    morning_start: TomlDateTime,
    morning_preparation_minutes: usize,
    night_start: TomlDateTime,
    bathtime_minutes: usize,
    underwear: UnderwearConfiguration,
    masturbation: MasturbationConfiguration,
}

#[derive(Debug, Clone, Serialize)]
struct DailyPrivateInfo {
    asked_at: String,
    current_status: DayStep,
    underwear_status: UnderwearStatus,
    masturbation_status: MasturbationStatus,
    menstruation_cycle: usize,
}

#[derive(Debug)]
pub struct DailyPrivate {
    rng_salt: String,
    day_routine: DayRoutineConfiguration,
    underwear: UnderwearConfiguration,
    masturbation: MasturbationConfiguration,
}

impl ConfigurableSimpleFunction for DailyPrivate {
    const NAME: &'static str = stringify!(DailyPrivate);

    type Configuration = DailyPrivateConfig;

    async fn configure(config: &DailyPrivateConfig) -> Result<Self, FunctionError> {
        Ok(DailyPrivate {
            rng_salt: config.daily_rng_salt.clone(),
            day_routine: DayRoutineConfiguration {
                daytime_start_at: extract_time_from_toml(config.morning_start)?,
                morning_preparation: Duration::minutes(config.morning_preparation_minutes as i64),
                night_start_at: extract_time_from_toml(config.night_start)?,
                bathtime_duration: Duration::minutes(config.bathtime_minutes as i64),
            },
            underwear: config.underwear.clone(),
            masturbation: config.masturbation.clone(),
        })
    }
}

impl SimpleFunction for DailyPrivate {
    fn get_descriptor(&self) -> SimpleFunctionDescriptor {
        SimpleFunctionDescriptor {
            name: "daily_private".to_string(),
            description: r#"
                この bot 自身のその日のプライベートな事情を取得します。
                以下のいずれかの項目についてユーザーから訊かれた場合、この function のレスポンスをもとに適切に回答してください。
                訊かれていない項目については答えなくてかまいません。
                - 下着の色
                - 生理周期
                - その日のオナニーの回数
                - 今の行動状態
            "#
            .to_string(),
            parameters: DescribedSchema::object("parameters", "引数", vec![]),
        }
    }

    fn call<'a>(&'a self, _id: &str, _params: Value) -> BoxFuture<'a, Result<SimpleFunctionResponse, FunctionError>> {
        async move { self.get_daily_info().await }.boxed()
    }
}

impl DailyPrivate {
    async fn get_daily_info(&self) -> Result<SimpleFunctionResponse, FunctionError> {
        let now = OffsetDateTime::now_local().map_err(FunctionError::by_external)?;
        let local_now = PrimitiveDateTime::new(now.date(), now.time());
        let logical_date = self.day_routine.logical_date(local_now);
        let day_progress = self.day_routine.logical_day_progress(local_now);
        let day_step = self.day_routine.determine_day_step(local_now);
        info!(
            "logical date: {logical_date}, day progress: {:.2}%, step: {day_step:?}",
            day_progress * 100.0
        );

        let mut daily_rng = {
            let mut hasher = Sha256::new();
            hasher.update(&self.rng_salt);
            hasher.update(logical_date.ordinal().to_le_bytes());
            StdRng::from_seed(hasher.finalize().into())
        };

        let underwear_status = self.underwear.generate_status(&mut daily_rng, day_step, None);
        info!("underwear status: {underwear_status:?}");
        let masturbation_ranges = self.masturbation.get_playing_ranges(&mut daily_rng);
        let masturbation_status = self.masturbation.construct_status(&masturbation_ranges, day_progress);
        info!(
            "masturbation: playing {}, {} completed, {} planned total today",
            masturbation_status.playing_now,
            masturbation_status.completed_count,
            masturbation_ranges.len()
        );

        let info = DailyPrivateInfo {
            asked_at: now.format(&Rfc3339).map_err(FunctionError::by_serialization)?,
            current_status: day_step,
            underwear_status,
            masturbation_status,
            menstruation_cycle: 1,
        };
        Ok(SimpleFunctionResponse {
            result: serde_json::to_value(&info).map_err(FunctionError::by_serialization)?,
            attachments: vec![],
        })
    }
}
