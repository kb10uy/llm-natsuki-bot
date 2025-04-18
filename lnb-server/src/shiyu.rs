mod function;
mod inner;
mod worker;

pub use function::ShiyuProvider;

use std::sync::Arc;

use futures::{FutureExt, future::BoxFuture};
use lnb_core::{
    error::ReminderError,
    interface::{
        reminder::{Remind, Remindable, Reminder},
        server::LnbServer,
    },
};
use serde::Deserialize;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct ReminderConfig {
    redis_address: String,
    max_seconds: i64,
    notification_virtual_text: String,
}

#[derive(Clone)]
pub struct Shiyu(Arc<inner::ShiyuInner>);

impl Shiyu {
    pub async fn new(config: &ReminderConfig) -> Result<Shiyu, ReminderError> {
        let inner = inner::ShiyuInner::new(config).await?;
        Ok(Shiyu(Arc::new(inner)))
    }

    pub async fn register_remindable(&self, remindable: impl Remindable) {
        self.0.register_remindable(remindable).await;
    }

    pub fn run(&self, server: impl LnbServer) -> BoxFuture<'static, Result<(), ReminderError>> {
        self.0.run(server)
    }
}

impl Reminder for Shiyu {
    fn register<'a>(
        &'a self,
        context: &'a str,
        remind: Remind,
        remind_at: OffsetDateTime,
    ) -> BoxFuture<'a, Result<Uuid, ReminderError>> {
        async move { self.0.register(context, remind, remind_at).await }.boxed()
    }

    fn remove(&self, id: Uuid) -> BoxFuture<'_, Result<(), ReminderError>> {
        async move { self.0.remove(id).await }.boxed()
    }
}
