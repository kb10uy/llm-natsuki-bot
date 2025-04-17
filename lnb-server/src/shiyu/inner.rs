use std::{collections::HashMap, sync::Arc};

use crate::shiyu::{ReminderConfig, worker::Worker};

use futures::{FutureExt, TryFutureExt, future::BoxFuture, select};
use lnb_core::{
    error::ReminderError,
    interface::{
        Context,
        reminder::{Remind, Remindable},
        server::LnbServer,
    },
    model::{
        conversation::UserRole,
        message::{UserMessage, UserMessageContent},
    },
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tokio::{
    spawn,
    sync::{RwLock, mpsc::UnboundedReceiver},
};
use tracing::{info, warn};
use uuid::Uuid;

pub struct ShiyuInner {
    worker: Worker,
    remindables: Arc<RwLock<HashMap<String, Arc<dyn Remindable>>>>,
    notification_virtual_text: String,
}

struct ShiyuDispatcher {
    receiver: UnboundedReceiver<ShiyuJob>,
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
    pub async fn new(config: &ReminderConfig) -> Result<ShiyuInner, ReminderError> {
        let worker = Worker::connect(&config.redis_address).await?;

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
}

impl ShiyuDispatcher {
    async fn run(mut self) -> Result<(), ReminderError> {
        let virtual_text: Arc<str> = self.notification_virtual_text.into();

        while let Some(job) = self.receiver.recv().await {
            info!(
                "sending reminder: ({} / {}) {}",
                job.context, job.remind.requester, job.remind.content
            );
            let remindable = {
                let locked = self.remindables.read().await;
                let Some(remindable) = locked.get(&job.context) else {
                    warn!("unknown context: {}", job.context);
                    continue;
                };
                remindable.clone()
            };

            spawn(ShiyuDispatcher::send_remind(
                self.server.clone(),
                remindable,
                job.remind,
                virtual_text.clone(),
            ));
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
            .process_conversation(Context::default(), conversation_id, user_message, UserRole::Normal)
            .map_err(ReminderError::by_internal)
            .await?;
        remindable
            .remind(remind.requester, update)
            .map_err(ReminderError::by_internal)
            .await?;
        Ok(())
    }
}
