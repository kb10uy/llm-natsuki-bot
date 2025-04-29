use futures::{FutureExt, TryFutureExt, future::BoxFuture};
use lnb_common::config::reminder::ConfigReminder;
use lnb_core::{
    error::FunctionError,
    interface::{
        Context,
        function::{Function, FunctionDescriptor, FunctionResponse},
        reminder::{Remind, RemindableContext, Reminder},
    },
    model::{
        conversation::{IncompleteConversation, UserRole},
        message::MessageToolCalling,
        schema::DescribedSchema,
    },
};
use serde::{Deserialize, Serialize};
use time::{
    Date, Duration, OffsetDateTime,
    format_description::{BorrowedFormatItem, well_known::Rfc3339},
    macros::format_description,
};
use tracing::{info, warn};

const DATE_FORMAT: &[BorrowedFormatItem<'static>] = format_description!("[year]-[month]-[day]");

pub struct ShiyuProvider {
    reminder: Box<dyn Reminder>,
    max_seconds: i64,
}

impl Function for ShiyuProvider {
    fn get_descriptor(&self) -> FunctionDescriptor {
        FunctionDescriptor {
            name: "shiyu_provider".to_string(),
            description: r#"
                ユーザーにリマインダー機能を提供します。
                - 先に local_info で現在時刻の情報を取得し、ユーザーが希望した時刻になるように remind_at に指定してください。その際、タイムゾーンは保持してください。
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
                    DescribedSchema::string(
                        "cancel",
                        "ユーザーがキャンセルを要求したリマインドの id。新規設定時は無視してください。",
                    ).as_nullable(),
                    DescribedSchema::string(
                        "content",
                        "ユーザーがリマインドを希望した内容。キャンセルの要求時は空にしてください。",
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
        tool_calling: MessageToolCalling,
    ) -> BoxFuture<'a, Result<FunctionResponse, FunctionError>> {
        let parameters = match serde_json::from_value(tool_calling.arguments).map_err(FunctionError::by_serialization) {
            Ok(p) => p,
            Err(err) => return async { Err(FunctionError::Serialization(err.into())) }.boxed(),
        };
        async move { self.execute(context, parameters).await }.boxed()
    }
}

impl ShiyuProvider {
    pub async fn new(config: &ConfigReminder, reminder: impl Reminder) -> Result<ShiyuProvider, FunctionError> {
        Ok(ShiyuProvider {
            reminder: Box::new(reminder),
            max_seconds: config.max_seconds,
        })
    }

    async fn execute(
        &self,
        context: &Context,
        parameters: ReminderParameters,
    ) -> Result<FunctionResponse, FunctionError> {
        let Some(remindable) = context.get::<RemindableContext>() else {
            return self.error(ReminderResponse::UnsupportedPlatform).await;
        };

        if let Some(cancel_id) = parameters.cancel {
            return self.cancel(cancel_id).await;
        }

        let now = OffsetDateTime::now_local().map_err(FunctionError::by_external)?;
        let complete_remind_at = if let Some(remind_at) = parameters.remind_at {
            if let Ok(full_datetime) = OffsetDateTime::parse(&remind_at, &Rfc3339) {
                full_datetime
            } else if let Ok(date) = Date::parse(&remind_at, DATE_FORMAT) {
                now.replace_date(date)
            } else {
                return self.error(ReminderResponse::InvalidRequest).await;
            }
        } else {
            return self.error(ReminderResponse::InvalidRequest).await;
        };

        if complete_remind_at - now > Duration::seconds(self.max_seconds) {
            return self.error(ReminderResponse::DueLimitExceeded).await;
        }

        self.register(remindable, complete_remind_at, parameters.content).await
    }

    async fn register(
        &self,
        remindable: &RemindableContext,
        remind_at: OffsetDateTime,
        content: String,
    ) -> Result<FunctionResponse, FunctionError> {
        let remind = Remind {
            requester: remindable.requester.clone(),
            content: content.clone(),
        };
        let id = self
            .reminder
            .register(&remindable.context, remind, remind_at.to_utc())
            .map_err(FunctionError::by_external)
            .await?;

        info!(
            "reminder registered: [{id}] ({} / {}): {content} @ {remind_at}",
            remindable.context, remindable.requester
        );
        Ok(FunctionResponse {
            result: serde_json::to_value(ReminderResponse::Registered {
                id: id.to_string(),
                remind_at: remind_at.format(&Rfc3339).map_err(FunctionError::by_serialization)?,
            })
            .map_err(FunctionError::by_serialization)?,
            ..Default::default()
        })
    }

    async fn cancel(&self, cancel_id: String) -> Result<FunctionResponse, FunctionError> {
        info!("reminder cancelled: {cancel_id}");
        Ok(FunctionResponse {
            result: serde_json::to_value(ReminderResponse::Cancelled { id: cancel_id })
                .map_err(FunctionError::by_serialization)?,
            ..Default::default()
        })
    }

    async fn error(&self, response: ReminderResponse) -> Result<FunctionResponse, FunctionError> {
        warn!("reminder error: {response:?}");
        Ok(FunctionResponse {
            result: serde_json::to_value(response).map_err(FunctionError::by_serialization)?,
            ..Default::default()
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
struct ReminderParameters {
    remind_at: Option<String>,
    cancel: Option<String>,
    content: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case", tag = "status", content = "data")]
enum ReminderResponse {
    Registered { id: String, remind_at: String },
    Cancelled { id: String },
    DueLimitExceeded,
    InvalidRequest,
    UnsupportedPlatform,
}
