mod function;
mod inner;
mod worker;

pub use function::ShiyuProvider;
use uuid::Uuid;

use crate::natsuki::Natsuki;

use std::sync::Arc;

use futures::{FutureExt, future::BoxFuture};
use lnb_core::{
    error::ReminderError,
    interface::reminder::{Remind, Reminder},
};
use serde::Deserialize;
use time::OffsetDateTime;

#[derive(Debug, Clone, Deserialize)]
pub struct ReminderConfig {
    redis_address: String,
    max_seconds: i64,
    notification_virtual_text: String,
}

#[derive(Clone)]
pub struct Shiyu(Arc<inner::ShiyuInner>);

impl Shiyu {
    pub async fn new(config: &ReminderConfig, natsuki: Natsuki) -> Result<Shiyu, ReminderError> {
        let inner = inner::ShiyuInner::new(config, natsuki).await?;
        Ok(Shiyu(Arc::new(inner)))
    }

    pub fn run(&self) -> BoxFuture<'static, Result<(), ReminderError>> {
        self.0.run()
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
