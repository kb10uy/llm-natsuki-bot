mod function;
mod inner;
mod worker;

pub use function::ShiyuProvider;
use time::OffsetDateTime;

use std::sync::Arc;

use lnb_core::{
    error::ReminderError,
    interface::reminder::{Remind, Reminder},
};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ReminderConfig {
    redis_address: String,
    max_seconds: i64,
    notification_virtual_text: String,
}

#[derive(Debug, Clone)]
pub struct Shiyu(Arc<inner::ShiyuInner>);

impl Shiyu {
    pub async fn new(config: &ReminderConfig) -> Result<Shiyu, ReminderError> {
        let inner = inner::ShiyuInner::new(config).await?;
        Ok(Shiyu(Arc::new(inner)))
    }

    pub async fn run(&self) {}
}

impl Reminder for Shiyu {
    fn register(&self, remind: Remind, remind_at: OffsetDateTime) -> Result<(), ReminderError> {
        todo!()
    }
}
