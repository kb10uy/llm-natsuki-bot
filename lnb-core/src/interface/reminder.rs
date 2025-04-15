use crate::error::ReminderError;

use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
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
        remind_at: OffsetDateTime,
    ) -> BoxFuture<'a, Result<Uuid, ReminderError>>;

    fn remove(&self, id: Uuid) -> BoxFuture<'_, Result<(), ReminderError>>;
}

/// Context で Reminder に送信できることを示す。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Remindable {
    pub context: String,
    pub requester: String,
}
