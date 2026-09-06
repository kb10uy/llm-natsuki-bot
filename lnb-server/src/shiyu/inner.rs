use crate::shiyu::worker::{ClaimedJob, Worker};

use std::{collections::HashMap, sync::Arc};

use futures::{FutureExt, TryFutureExt, future::BoxFuture, select};
use lnb_common::config::reminder::ConfigReminder;
use lnb_core::{
    error::ReminderError,
    interface::{
        MessageContext,
        reminder::{Remind, Remindable},
        server::LnbServer,
    },
    model::message::{UserMessage, UserMessageContent},
};
use serde::{Deserialize, Serialize};
use time::UtcDateTime;
use tokio::{
    spawn,
    sync::{RwLock, Semaphore, mpsc::Receiver},
};
use tracing::{info, warn};
use uuid::Uuid;

const DELIVERY_RETRY_DELAY: time::Duration = time::Duration::seconds(30);
const MAX_CONCURRENT_DELIVERIES: usize = 16;

pub struct ShiyuInner {
    worker: Worker,
    remindables: Arc<RwLock<HashMap<String, Arc<dyn Remindable>>>>,
    notification_virtual_text: String,
}

struct ShiyuDispatcher {
    receiver: Receiver<ClaimedJob<ShiyuJob>>,
    worker: Worker,
    server: Arc<dyn LnbServer>,
    remindables: Arc<RwLock<HashMap<String, Arc<dyn Remindable>>>>,
    notification_virtual_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ShiyuJob {
    context: String,
    remind: Remind,
}

impl ShiyuInner {
    pub async fn new(config: &ConfigReminder) -> Result<ShiyuInner, ReminderError> {
        let worker = Worker::connect(config).await?;

        Ok(ShiyuInner {
            worker,
            remindables: Arc::new(RwLock::new(HashMap::new())),
            notification_virtual_text: config.notification_virtual_text.clone(),
        })
    }

    pub async fn register_remindable(&self, remindable: impl Remindable) {
        let mut locked = self.remindables.write().await;
        let remindable = Arc::new(remindable);
        let context = remindable.get_context();
        locked.insert(context, remindable);
    }

    pub fn run(&self, server: impl LnbServer) -> BoxFuture<'static, Result<(), ReminderError>> {
        // worker
        let (worker_task, receiver) = self.worker.run::<ShiyuJob>();

        // dispatcher
        let dispatcher = ShiyuDispatcher {
            server: Arc::new(server),
            remindables: self.remindables.clone(),
            receiver,
            worker: self.worker.clone(),
            notification_virtual_text: self.notification_virtual_text.clone(),
        };
        let dispatcher_task = dispatcher.run();

        async move {
            select! {
                wr = worker_task.fuse() => wr,
                dr = dispatcher_task.fuse() => dr,
            }
        }
        .boxed()
    }

    pub async fn register(&self, context: &str, remind: Remind, remind_at: UtcDateTime) -> Result<Uuid, ReminderError> {
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
}

impl ShiyuDispatcher {
    async fn run(mut self) -> Result<(), ReminderError> {
        let virtual_text: Arc<str> = self.notification_virtual_text.into();
        let delivery_permits = Arc::new(Semaphore::new(MAX_CONCURRENT_DELIVERIES));

        while let Some(claimed) = self.receiver.recv().await {
            let permit = delivery_permits
                .clone()
                .acquire_owned()
                .await
                .expect("delivery semaphore should remain open");
            let id = claimed.id;
            let job = claimed.payload;
            info!(
                "sending reminder {id}: ({} / {}) {}",
                job.context, job.remind.requester, job.remind.content,
            );
            let remindable = {
                let locked = self.remindables.read().await;
                let Some(remindable) = locked.get(&job.context) else {
                    warn!("unknown context: {}", job.context);
                    self.worker.retry(id, UtcDateTime::now() + DELIVERY_RETRY_DELAY).await?;
                    continue;
                };
                remindable.clone()
            };

            let server = self.server.clone();
            let worker = self.worker.clone();
            let virtual_text = virtual_text.clone();
            spawn(async move {
                let _permit = permit;
                match ShiyuDispatcher::send_remind(server, remindable, job.remind, virtual_text).await {
                    Ok(()) => {
                        if let Err(err) = worker.acknowledge(id).await {
                            warn!("failed to acknowledge reminder {id}: {err}");
                        }
                    }
                    Err(err) => {
                        warn!("reminder {id} delivery failed: {err}");
                        if let Err(retry_err) = worker.retry(id, UtcDateTime::now() + DELIVERY_RETRY_DELAY).await {
                            warn!("failed to schedule reminder {id} retry: {retry_err}");
                        }
                    }
                }
            });
        }
        Ok(())
    }

    async fn send_remind(
        server: Arc<dyn LnbServer>,
        remindable: Arc<dyn Remindable>,
        remind: Remind,
        virtual_text: Arc<str>,
    ) -> Result<(), ReminderError> {
        let conversation_id = server.new_conversation().map_err(ReminderError::by_internal).await?;
        let text = format!("{}\n{}", virtual_text, remind.content);
        let user_message = UserMessage {
            contents: vec![UserMessageContent::Text(text)],
            ..Default::default()
        };
        let update = server
            .process_conversation(MessageContext::new_system(), conversation_id, vec![user_message.into()])
            .map_err(ReminderError::by_internal)
            .await?;
        remindable
            .remind(remind.requester, update)
            .map_err(ReminderError::by_internal)
            .await?;
        Ok(())
    }
}
