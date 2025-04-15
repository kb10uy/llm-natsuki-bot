use crate::error::WorkerError;

use std::{convert::Infallible, time::Duration};

use futures::{FutureExt, future::BoxFuture};
use redis::{AsyncCommands, Client, Value, aio::MultiplexedConnection};
use serde::{Serialize, de::DeserializeOwned};
use time::OffsetDateTime;
use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
    time::sleep,
};
use tracing::{error, info};

const QUEUE_KEY: &str = "lnb_queue";
const DISCONNECTION_RETRY_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct Worker {
    connection: MultiplexedConnection,
    polling_interval: Duration,
}

impl Worker {
    pub async fn connect(address: &str) -> Result<Worker, WorkerError> {
        let client = Client::open(address)?;
        let connection = client.get_multiplexed_async_connection().await?;

        Ok(Worker {
            connection,
            polling_interval: Duration::from_secs(5),
        })
    }

    pub async fn enqueue<T>(&self, job: &T, execute_at: OffsetDateTime) -> Result<(), WorkerError>
    where
        T: Serialize,
    {
        let mut conn = self.connection.clone();
        let score = (execute_at.unix_timestamp_nanos() / 1_000_000_000) as f64;
        let job_bytes = rmp_serde::to_vec(job).map_err(WorkerError::by_serialization)?;
        let _: Value = conn.zadd(QUEUE_KEY, job_bytes, score).await?;
        Ok(())
    }

    pub fn run<T>(&self) -> (BoxFuture<'static, Result<(), WorkerError>>, UnboundedReceiver<T>)
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

    async fn run_connection<T>(&mut self, send: UnboundedSender<T>) -> Result<Infallible, WorkerError>
    where
        T: Send + Sync + DeserializeOwned,
    {
        info!("connection established");
        loop {
            let now_unixtime = (OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000_000) as f64;
            let target_job_count: isize = self
                .connection
                .zcount(QUEUE_KEY, f64::NEG_INFINITY, now_unixtime)
                .await?;
            let jobs: Vec<Vec<u8>> = self.connection.zpopmin(QUEUE_KEY, target_job_count).await?;
            for job_bytes in jobs {
                let job: T = rmp_serde::from_slice(&job_bytes).map_err(WorkerError::by_serialization)?;
                send.send(job).map_err(|_| WorkerError::CannotPushAnymore)?;
            }

            sleep(self.polling_interval).await;
        }
    }
}
