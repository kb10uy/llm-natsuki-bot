use crate::{error::ReminderError, model::conversation::ConversationUpdate};

use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use time::UtcDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Remind {
    pub requester: String,
    pub content: String,
}

pub trait Reminder: Send + Sync + 'static {
    fn register<'a>(
        &'a self,
        context: &'a str,
        remind: Remind,
        remind_at: UtcDateTime,
    ) -> BoxFuture<'a, Result<Uuid, ReminderError>>;

    fn remove(&self, id: Uuid) -> BoxFuture<'_, Result<(), ReminderError>>;
}

/// Context で Reminder に送信できることを示す。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemindableContext {
    pub context: String,
    pub requester: String,
}

/// Reminder を送信可能なクライアントが実装する。
pub trait Remindable: Send + Sync + 'static {
    fn get_context(&self) -> String;
    fn remind(
        &self,
        requester: String,
        remind_conversation: ConversationUpdate,
    ) -> BoxFuture<'_, Result<(), ReminderError>>;
}
