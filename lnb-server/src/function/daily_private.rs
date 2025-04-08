use crate::function::ConfigurableSimpleFunction;

use futures::{FutureExt, future::BoxFuture};
use lnb_core::{
    error::FunctionError,
    interface::function::simple::{SimpleFunction, SimpleFunctionDescriptor, SimpleFunctionResponse},
    model::schema::DescribedSchema,
};
use lnb_daily_private::Configuration;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{OffsetDateTime, Time};
use toml::value::Time as TomlTime;

#[derive(Debug, Clone, Deserialize)]
pub struct DailyPrivateConfig {
    night_start: TomlTime,
    morning_start: TomlTime,
}

#[derive(Debug, Clone, Serialize)]
struct ClothesDesign {
    color: String,
    pattern: String,
}

#[derive(Debug, Clone, Serialize)]
struct DailyPrivateInfo {
    asked_at: OffsetDateTime,
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
        let night_start_at = Time::from_hms_nano(
            config.night_start.hour,
            config.night_start.minute,
            config.night_start.second,
            config.night_start.nanosecond,
        )
        .map_err(FunctionError::by_serialization)?;
        let daytime_start_at = Time::from_hms_nano(
            config.morning_start.hour,
            config.morning_start.minute,
            config.morning_start.second,
            config.morning_start.nanosecond,
        )
        .map_err(FunctionError::by_serialization)?;

        Ok(DailyPrivate {
            configuration: Configuration {
                night_start_at,
                daytime_start_at,
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
                これには下着の色や月経、その日のセルフプレジャーの回数が含まれます。
                これらについてユーザーから訊かれた場合、LLM による生成ではなくこの function のレスポンスをもとに回答してください。
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
        let logical_date = self.configuration.logical_date(now);
        let day_step = self.configuration.determine_day_step(now);

        let info = DailyPrivateInfo {
            asked_at: now,
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
