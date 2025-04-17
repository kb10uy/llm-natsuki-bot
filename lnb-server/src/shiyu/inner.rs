use std::{collections::HashMap, sync::Arc};

use crate::shiyu::{ReminderConfig, worker::Worker};

use futures::{FutureExt, future::BoxFuture, select};
use lnb_core::{
    error::ReminderError,
    interface::{
        reminder::{Remind, Remindable},
        server::LnbServer,
    },
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tokio::sync::{RwLock, mpsc::UnboundedReceiver};
use tracing::{info, warn};
use uuid::Uuid;

pub struct ShiyuInner {
    worker: Worker,
    remindables: Arc<RwLock<HashMap<String, Arc<dyn Remindable>>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ShiyuJob {
    context: String,
    remind: Remind,
}

impl ShiyuInner {
    pub async fn new(config: &ReminderConfig) -> Result<ShiyuInner, ReminderError> {
        let worker = Worker::connect(&config.redis_address).await?;

        Ok(ShiyuInner {
            worker,
            remindables: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn register_remindable(&self, remindable: impl Remindable) {
        let mut locked = self.remindables.write().await;
        let remindable = Arc::new(remindable);
        let context = remindable.get_context();
        locked.insert(context, remindable);
    }

    pub fn run(&self, server: impl LnbServer) -> BoxFuture<'static, Result<(), ReminderError>> {
        let (worker_task, receiver) = self.worker.run::<ShiyuJob>();
        let dispatcher_task = ShiyuInner::run_dispatcher(server, self.remindables.clone(), receiver);
        async move {
            select! {
                wr = worker_task.fuse() => wr,
                dr = dispatcher_task.fuse() => dr,
            }
        }
        .boxed()
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

    pub async fn remove(&self, id: Uuid) -> Result<(), ReminderError> {
        self.worker.remove(id).await?;
        Ok(())
    }

    async fn run_dispatcher(
        server: impl LnbServer,
        remindables: Arc<RwLock<HashMap<String, Arc<dyn Remindable>>>>,
        mut receiver: UnboundedReceiver<ShiyuJob>,
    ) -> Result<(), ReminderError> {
        while let Some(job) = receiver.recv().await {
            info!("sending reminder: [{}] {:?}", job.context, job.remind);
            let remindable = {
                let locked = remindables.read().await;
                let Some(remindable) = locked.get(&job.context) else {
                    warn!("unknown context: {}", job.context);
                    continue;
                };
                remindable.clone()
            };

            // remindable.remind(job.context, remind_conversation);
        }
        Ok(())
    }
}
