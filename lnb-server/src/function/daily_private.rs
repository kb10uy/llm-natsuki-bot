use crate::function::{ConfigurableSimpleFunction, extract_time_from_toml};

use futures::{FutureExt, future::BoxFuture};
use lnb_core::{
    error::FunctionError,
    interface::function::simple::{SimpleFunction, SimpleFunctionDescriptor, SimpleFunctionResponse},
    model::schema::DescribedSchema,
};
use lnb_daily_private::{Configuration, DayStep};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{Duration, OffsetDateTime, PrimitiveDateTime};
use toml::value::Datetime as TomlDateTime;

// TOML は Time だけを書けるが toml::Time は from str しかできないので TomlDateTime で拾う
#[derive(Debug, Clone, Deserialize)]
pub struct DailyPrivateConfig {
    morning_start: TomlDateTime,
    morning_preparation_minutes: usize,
    night_start: TomlDateTime,
    bathtime_minutes: usize,
}

#[derive(Debug, Clone, Serialize)]
struct ClothesDesign {
    color: String,
    pattern: String,
}

#[derive(Debug, Clone, Serialize)]
struct DailyPrivateInfo {
    asked_at: OffsetDateTime,
    current_status: DayStep,
    bra_design: ClothesDesign,
    panty_design: ClothesDesign,
    menstruation_cycle: usize,
    self_pleasure_count: usize,
}

#[derive(Debug)]
pub struct DailyPrivate {
    configuration: Configuration,
}

impl ConfigurableSimpleFunction for DailyPrivate {
    const NAME: &'static str = stringify!(DailyPrivate);

    type Configuration = DailyPrivateConfig;

    async fn configure(config: DailyPrivateConfig) -> Result<Self, FunctionError> {
        Ok(DailyPrivate {
            configuration: Configuration {
                daytime_start_at: extract_time_from_toml(config.morning_start)?,
                morning_preparation: Duration::minutes(config.morning_preparation_minutes as i64),
                night_start_at: extract_time_from_toml(config.night_start)?,
                bathtime_duration: Duration::minutes(config.bathtime_minutes as i64),
            },
        })
    }
}

impl SimpleFunction for DailyPrivate {
    fn get_descriptor(&self) -> SimpleFunctionDescriptor {
        SimpleFunctionDescriptor {
            name: "daily_private".to_string(),
            description: r#"
                この bot 自身のその日のプライベートな事情を取得します。
                これらについてユーザーから訊かれた場合、この function のレスポンスをもとに適切に回答してください。
                - 下着の色
                - 生理周期
                - その日のセルフプレジャーの回数
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
        let logical_date = self.configuration.logical_date(local_now);
        let day_step = self.configuration.determine_day_step(local_now);

        let info = DailyPrivateInfo {
            asked_at: now,
            current_status: day_step,
            bra_design: ClothesDesign {
                color: "青".to_string(),
                pattern: "しましま".to_string(),
            },
            panty_design: ClothesDesign {
                color: "オレンジ".to_string(),
                pattern: "しましま".to_string(),
            },
            menstruation_cycle: 1,
            self_pleasure_count: 2,
        };
        Ok(SimpleFunctionResponse {
            result: serde_json::to_value(&info).map_err(FunctionError::by_serialization)?,
            attachments: vec![],
        })
    }
}
