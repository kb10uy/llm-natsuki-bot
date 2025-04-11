use futures::{FutureExt, future::BoxFuture};
use lnb_core::{
    RFC3339_NUMOFFSET,
    error::FunctionError,
    interface::function::{FunctionDescriptor, FunctionResponse, simple::SimpleFunction},
    model::schema::DescribedSchema,
};
use serde_json::{Value, json};
use time::OffsetDateTime;

#[derive(Debug)]
pub struct LocalInfo {
    started_at: OffsetDateTime,
}

impl SimpleFunction for LocalInfo {
    fn get_descriptor(&self) -> FunctionDescriptor {
        FunctionDescriptor {
            name: "local_info".to_string(),
            description: r#"
                この bot が動作している環境に関する以下の情報を提供する。
                - 現在時刻
                - bot が動作を開始した日時
            "#
            .to_string(),
            parameters: DescribedSchema::object("parameters", "引数", vec![]),
        }
    }

    fn call<'a>(&'a self, _id: &str, _params: Value) -> BoxFuture<'a, Result<FunctionResponse, FunctionError>> {
        async { self.get_info() }.boxed()
    }
}

impl LocalInfo {
    pub fn new() -> Result<LocalInfo, FunctionError> {
        Ok(LocalInfo {
            started_at: OffsetDateTime::now_local().map_err(FunctionError::by_external)?,
        })
    }

    fn get_info(&self) -> Result<FunctionResponse, FunctionError> {
        let now = OffsetDateTime::now_local().map_err(FunctionError::by_external)?;
        Ok(FunctionResponse {
            result: json!({
                "time_now": now.format(RFC3339_NUMOFFSET).map_err(FunctionError::by_serialization)?,
                "bot_started_at": self.started_at.format(RFC3339_NUMOFFSET).map_err(FunctionError::by_serialization)?,
            }),
            ..Default::default()
        })
    }
}
