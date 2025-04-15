use crate::shiyu::{ReminderConfig, worker::Worker};

use futures::future::BoxFuture;
use lnb_core::{
    error::ReminderError,
    interface::{reminder::Remind, server::LnbServer},
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

pub struct ShiyuInner {
    server: Box<dyn LnbServer>,
    worker: Worker,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ShiyuJob {
    context: String,
    remind: Remind,
}

impl ShiyuInner {
    pub async fn new(config: &ReminderConfig, server: impl LnbServer) -> Result<ShiyuInner, ReminderError> {
        let server = Box::new(server);
        let worker = Worker::connect(&config.redis_address).await?;

        Ok(ShiyuInner { server, worker })
    }

    pub fn run(&self) -> BoxFuture<'static, Result<(), ReminderError>> {
        let (task, receiver) = self.worker.run::<ShiyuJob>();
        task
    }

    pub async fn register(
        &self,
        context: &str,
        remind: Remind,
        remind_at: OffsetDateTime,
    ) -> Result<Uuid, ReminderError> {
        let job = ShiyuJob {
            context: context.to_string(),
            remind,
        };
        let id = self.worker.enqueue(&job, remind_at).await?;
        Ok(id)
    }
}
