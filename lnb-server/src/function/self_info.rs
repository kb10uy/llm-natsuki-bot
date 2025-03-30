use futures::{FutureExt, future::BoxFuture};
use lnb_core::{
    error::FunctionError,
    interface::function::simple::{SimpleFunction, SimpleFunctionDescriptor, SimpleFunctionResponse},
    model::schema::DescribedSchema,
};
use serde_json::{Value, json};

#[derive(Debug)]
pub struct SelfInfo {}

impl SimpleFunction for SelfInfo {
    fn get_descriptor(&self) -> SimpleFunctionDescriptor {
        SimpleFunctionDescriptor {
            name: "self_info".to_string(),
            description: r#"
                この bot 自身に関する以下の情報を提供する。
                - バージョン
                - Git コミットハッシュ
                - bot のバイナリがビルドされた日時
            "#
            .to_string(),
            parameters: DescribedSchema::object("parameters", "引数", vec![]),
        }
    }

    fn call<'a>(&'a self, _id: &str, _params: Value) -> BoxFuture<'a, Result<SimpleFunctionResponse, FunctionError>> {
        async { self.get_info() }.boxed()
    }
}

impl SelfInfo {
    pub fn new() -> SelfInfo {
        SelfInfo {}
    }

    fn get_info(&self) -> Result<SimpleFunctionResponse, FunctionError> {
        Ok(SimpleFunctionResponse {
            result: json!({
                "bot_version": env!("CARGO_PKG_VERSION"),
                "bot_commit": env!("GIT_COMMIT_HASH"),
                "bot_binary_built_at": env!("BUILT_AT_DATETIME"),
            }),
            ..Default::default()
        })
    }
}
