use std::{convert::Infallible, time::Duration};

use futures::{FutureExt, TryFutureExt, future::BoxFuture};
use lnb_common::{config::reminder::ConfigReminder, persistence::RedisReminderDb};
use lnb_core::error::ReminderError;
use serde::{Serialize, de::DeserializeOwned};
use time::UtcDateTime;
use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
    time::sleep,
};
use tracing::{debug, error, info};
use uuid::Uuid;

const DISCONNECTION_RETRY_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct Worker {
    db: RedisReminderDb,
    polling_interval: Duration,
}

impl Worker {
    pub async fn connect(config: &ConfigReminder) -> Result<Worker, ReminderError> {
        let db = RedisReminderDb::connect(config)
            .map_err(ReminderError::by_internal)
            .await?;

        Ok(Worker {
            db,
            polling_interval: Duration::from_secs(5),
        })
    }

    pub async fn enqueue<T>(&self, job: &T, execute_at: UtcDateTime) -> Result<Uuid, ReminderError>
    where
        T: Serialize,
    {
        self.db
            .enqueue_job(job, execute_at)
            .map_err(ReminderError::by_internal)
            .await
    }

    pub async fn remove(&self, id: Uuid) -> Result<(), ReminderError> {
        self.db.remove_job(id).map_err(ReminderError::by_internal).await
    }

    pub fn run<T>(&self) -> (BoxFuture<'static, Result<(), ReminderError>>, UnboundedReceiver<T>)
    where
        T: 'static + Send + Sync + DeserializeOwned,
    {
        let (sender, receiver) = unbounded_channel();
        let cloned_self = self.clone();
        let running_future = async move {
            loop {
                let Err(err) = cloned_self.run_connection(sender.clone()).await;
                error!("worker failed on error: {err}");
                sleep(DISCONNECTION_RETRY_INTERVAL).await;
            }
        }
        .boxed();

        (running_future, receiver)
    }

    async fn run_connection<T>(&self, send: UnboundedSender<T>) -> Result<Infallible, ReminderError>
    where
        T: Send + Sync + DeserializeOwned,
    {
        info!("connection established");
        loop {
            let target_jobs = self
                .db
                .pull_jobs_until::<T>(UtcDateTime::now())
                .map_err(ReminderError::by_internal)
                .await?;
            for (id, job) in target_jobs {
                debug!("sending {id}");
                send.send(job).map_err(|_| ReminderError::CannotPushAnymore)?;
            }

            sleep(self.polling_interval).await;
        }
    }
}
