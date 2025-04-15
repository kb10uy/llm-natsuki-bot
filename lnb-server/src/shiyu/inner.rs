use crate::shiyu::{ReminderConfig, worker::Worker};

use lnb_core::{error::ReminderError, interface::reminder::Remind};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub struct ShiyuInner {
    worker: Worker,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ShiyuJob {
    context: String,
    remind: Remind,
}

impl ShiyuInner {
    pub async fn new(config: &ReminderConfig) -> Result<ShiyuInner, ReminderError> {
        let worker = Worker::connect(&config.redis_address).await?;

        Ok(ShiyuInner { worker })
    }

    pub async fn run(&self) -> Result<(), ReminderError> {
        Ok(())
    }

    pub async fn register(
        &self,
        context: &str,
        remind: Remind,
        remind_at: OffsetDateTime,
    ) -> Result<(), ReminderError> {
        let job = ShiyuJob {
            context: context.to_string(),
            remind,
        };
        self.worker.enqueue(&job, remind_at).await?;
        Ok(())
    }
}
