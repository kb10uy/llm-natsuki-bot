use crate::function::ConfigurableComplexFunction;

use futures::{FutureExt, future::BoxFuture};
use lnb_core::{
    error::FunctionError,
    interface::{
        Context,
        function::{FunctionDescriptor, FunctionResponse, complex::ComplexFunction},
    },
    model::{
        conversation::{IncompleteConversation, UserRole},
        message::MessageToolCalling,
        schema::DescribedSchema,
    },
};
use serde::Deserialize;
use serde_json::json;
use tracing::info;

#[derive(Debug)]
pub struct Reminder {
    max_seconds: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReminderConfig {
    max_seconds: usize,
}

impl ConfigurableComplexFunction for Reminder {
    const NAME: &'static str = stringify!(Reminder);

    type Configuration = ReminderConfig;

    async fn configure(config: &Self::Configuration) -> Result<Reminder, FunctionError> {
        Ok(Reminder {
            max_seconds: config.max_seconds,
        })
    }
}

impl ComplexFunction for Reminder {
    fn get_descriptor(&self) -> FunctionDescriptor {
        FunctionDescriptor {
            name: "reminder".to_string(),
            description: r#"
                ユーザーにリマインダー機能を提供します。
                - 何時間後や何日後など、確実に期間がわかる場合のみ remind_in にその秒数を指定し、それ以外の場合は remind_at に絶対形式で指定してください。
                - 現在時刻の情報が必要な場合は local_info で取得し、タイムゾーンは保持してください。
                - 会話の中でリマインダーのキャンセルを要求された場合、そのリマインダーの設定時のレスポンスに含まれる id を cancel に指定してください。
            "#
            .to_string(),
            parameters: DescribedSchema::object(
                "parameters",
                "引数",
                vec![
                    DescribedSchema::string(
                        "remind_at",
                        r#"
                            リマインドする絶対時刻(RFC3339形式)。ユーザーが明示的に時刻を指定しなかった場合は日付のみを指定してください。
                            相対時刻指定の場合は無視してください。
                        "#,
                    ).as_nullable(),
                    DescribedSchema::integer(
                        "remind_in",
                        "リマインドするまでの時間(合計秒数)。絶対時刻指定の場合は無視してください。",
                    ).as_nullable(),
                    DescribedSchema::string(
                        "cancel",
                        "ユーザーがキャンセルを要求したリマインドの id。新規設定時は無視してください。",
                    ).as_nullable(),
                    DescribedSchema::string(
                        "content",
                        "ユーザーがリマインドを希望した内容。情報の欠落を防ぐため、可能な限り原文のまま指定してください。キャンセルの要求時は空にしてください。",
                    ),
                ],
            ),
        }
    }

    fn call<'a>(
        &'a self,
        context: &'a Context,
        _incomplete: &'a IncompleteConversation,
        _user_role: &'a UserRole,
        tool_calling: &'a MessageToolCalling,
    ) -> BoxFuture<'a, Result<FunctionResponse, FunctionError>> {
        let parameters =
            match serde_json::from_value(tool_calling.arguments.clone()).map_err(FunctionError::by_serialization) {
                Ok(p) => p,
                Err(err) => return async { Err(FunctionError::Serialization(err.into())) }.boxed(),
            };
        async move { self.execute(context, parameters).await }.boxed()
    }
}

impl Reminder {
    async fn execute(
        &self,
        _context: &Context,
        parameters: ReminderParameters,
    ) -> Result<FunctionResponse, FunctionError> {
        info!("{parameters:?}");
        Ok(FunctionResponse {
            result: json!({
                "status": "accepted",
                "remind_at": "",
                "id": "hogehoge",
            }),
            /*
            result: json!({
                "error": "this feature is not implemented yet",
            }),
            */
            ..Default::default()
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
struct ReminderParameters {
    remind_at: Option<String>,
    remind_in: Option<usize>,
    cancel: Option<String>,
    content: String,
}
