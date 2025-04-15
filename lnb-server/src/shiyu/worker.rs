use std::{convert::Infallible, time::Duration};

use futures::{FutureExt, TryFutureExt, future::BoxFuture};
use lnb_core::error::ReminderError;
use redis::{AsyncCommands, Client, Value, aio::MultiplexedConnection};
use serde::{Serialize, de::DeserializeOwned};
use time::OffsetDateTime;
use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
    time::sleep,
};
use tracing::{error, info};
use uuid::Uuid;

const QUEUE_KEY: &str = "lnb_queue";
const JOB_TABLE_KEY: &str = "lnb_jobs";
const DISCONNECTION_RETRY_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct Worker {
    connection: MultiplexedConnection,
    polling_interval: Duration,
}

impl Worker {
    pub async fn connect(address: &str) -> Result<Worker, ReminderError> {
        let client = Client::open(address).map_err(ReminderError::by_internal)?;
        let connection = client
            .get_multiplexed_async_connection()
            .map_err(ReminderError::by_internal)
            .await?;

        Ok(Worker {
            connection,
            polling_interval: Duration::from_secs(5),
        })
    }

    pub async fn enqueue<T>(&self, job: &T, execute_at: OffsetDateTime) -> Result<Uuid, ReminderError>
    where
        T: Serialize,
    {
        let mut conn = self.connection.clone();

        let id = Uuid::new_v4();
        let id_str = id.to_string();

        // ジョブ本体を登録
        let job_bytes = rmp_serde::to_vec(job).map_err(ReminderError::by_serialization)?;
        let _: Value = conn
            .hset(JOB_TABLE_KEY, &id_str, job_bytes)
            .map_err(ReminderError::by_internal)
            .await?;

        // キューに時刻とのマッピングを登録
        let score = (execute_at.unix_timestamp_nanos() / 1_000_000_000) as f64;
        let _: Value = conn
            .zadd(QUEUE_KEY, &id_str, score)
            .map_err(ReminderError::by_internal)
            .await?;

        Ok(id)
    }

    pub fn run<T>(&self) -> (BoxFuture<'static, Result<(), ReminderError>>, UnboundedReceiver<T>)
    where
        T: 'static + Send + Sync + DeserializeOwned,
    {
        let (sender, receiver) = unbounded_channel();
        let mut cloned_self = self.clone();
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

    async fn run_connection<T>(&mut self, send: UnboundedSender<T>) -> Result<Infallible, ReminderError>
    where
        T: Send + Sync + DeserializeOwned,
    {
        info!("connection established");
        loop {
            let now_unixtime = (OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000_000) as f64;
            let target_job_count: isize = self
                .connection
                .zcount(QUEUE_KEY, f64::NEG_INFINITY, now_unixtime)
                .map_err(ReminderError::by_internal)
                .await?;
            let jobs: Vec<Vec<u8>> = self
                .connection
                .zpopmin(QUEUE_KEY, target_job_count)
                .map_err(ReminderError::by_internal)
                .await?;
            for job_bytes in jobs {
                let job: T = rmp_serde::from_slice(&job_bytes).map_err(ReminderError::by_serialization)?;
                send.send(job).map_err(|_| ReminderError::CannotPushAnymore)?;
            }

            sleep(self.polling_interval).await;
        }
    }
}
