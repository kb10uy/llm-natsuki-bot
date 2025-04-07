use futures::{FutureExt, future::BoxFuture};
use lnb_core::{
    error::FunctionError,
    interface::function::simple::{SimpleFunction, SimpleFunctionDescriptor, SimpleFunctionResponse},
    model::schema::DescribedSchema,
};
use serde::Serialize;
use serde_json::Value;
use time::OffsetDateTime;

#[derive(Debug)]
pub struct DailyPrivate {}

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

#[derive(Debug, Clone, Serialize)]
pub struct ClothesDesign {
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
