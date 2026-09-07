use std::{convert::Infallible, time::Duration};

use futures::{FutureExt, TryFutureExt, future::BoxFuture};
use lnb_core::error::ReminderError;
use lnb_reminder_redis::{ConfigReminder, RedisReminderDb};
use serde::{Serialize, de::DeserializeOwned};
use time::UtcDateTime;
use tokio::{
    sync::mpsc::{Receiver, Sender, channel},
    time::sleep,
};
use tracing::{debug, error, info};
use uuid::Uuid;

const DISCONNECTION_RETRY_INTERVAL: Duration = Duration::from_secs(5);
const CLAIM_LEASE: time::Duration = time::Duration::minutes(30);
const CLAIM_BATCH_SIZE: usize = 64;
const CHANNEL_CAPACITY: usize = 64;

#[derive(Debug)]
pub(super) struct ClaimedJob<T> {
    pub id: Uuid,
    pub payload: T,
}

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

    pub async fn acknowledge(&self, id: Uuid) -> Result<(), ReminderError> {
        self.db.acknowledge_job(id).map_err(ReminderError::by_internal).await
    }

    pub async fn retry(&self, id: Uuid, execute_at: UtcDateTime) -> Result<(), ReminderError> {
        self.db
            .retry_job(id, execute_at)
            .map_err(ReminderError::by_internal)
            .await
    }

    pub fn run<T>(&self) -> (BoxFuture<'static, Result<(), ReminderError>>, Receiver<ClaimedJob<T>>)
    where
        T: 'static + Send + Sync + DeserializeOwned,
    {
        let (sender, receiver) = channel(CHANNEL_CAPACITY);
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

    async fn run_connection<T>(&self, send: Sender<ClaimedJob<T>>) -> Result<Infallible, ReminderError>
    where
        T: Send + Sync + DeserializeOwned,
    {
        info!("connection established");
        loop {
            let now = UtcDateTime::now();
            let target_jobs = self
                .db
                .claim_jobs_until::<T>(now, now + CLAIM_LEASE, CLAIM_BATCH_SIZE)
                .map_err(ReminderError::by_internal)
                .await?;
            for (id, job) in target_jobs {
                debug!("sending {id}");
                if send.send(ClaimedJob { id, payload: job }).await.is_err() {
                    self.retry(id, UtcDateTime::now()).await?;
                    return Err(ReminderError::CannotPushAnymore);
                }
            }

            sleep(self.polling_interval).await;
        }
    }
}
