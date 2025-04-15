use crate::shiyu::{ReminderConfig, worker::Worker};

use lnb_core::error::ReminderError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ShiyuInner {
    worker: Worker,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ShiyuJob {
    context: String,
    text: String,
}

impl ShiyuInner {
    pub async fn new(config: &ReminderConfig) -> Result<ShiyuInner, ReminderError> {
        let worker = Worker::connect(&config.redis_address).await?;

        Ok(ShiyuInner { worker })
    }

    pub async fn run(&self) -> Result<(), ReminderError> {
        let (task, receiver) = self.worker.run();
        Ok(())
    }
}
