mod function;
mod inner;
mod worker;

pub use function::ShiyuProvider;
use lnb_common::config::reminder::ConfigReminder;

use std::sync::Arc;

use futures::{FutureExt, future::BoxFuture};
use lnb_core::{
    error::ReminderError,
    interface::{
        reminder::{Remind, Remindable, Reminder},
        server::LnbServer,
    },
};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Clone)]
pub struct Shiyu(Arc<inner::ShiyuInner>);

impl Shiyu {
    pub async fn new(config: &ConfigReminder) -> Result<Shiyu, ReminderError> {
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
