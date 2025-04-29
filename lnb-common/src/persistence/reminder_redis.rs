use crate::{config::reminder::ConfigReminder, persistence::PersistenceError};

use futures::TryFutureExt;
use redis::{AsyncCommands, Client, Value, aio::MultiplexedConnection};
use serde::{Serialize, de::DeserializeOwned};
use time::UtcDateTime;
use tracing::{debug, trace};
use uuid::Uuid;

const JOB_TABLE_KEY: &str = "lnb_jobs";
const QUEUE_KEY: &str = "lnb_queue";

#[derive(Debug, Clone)]
pub struct RedisReminderDb {
    connection: MultiplexedConnection,
}

impl RedisReminderDb {
    pub async fn connect(config: &ConfigReminder) -> Result<RedisReminderDb, PersistenceError> {
        let client = Client::open(config.redis_address.as_str()).map_err(PersistenceError::by_backend)?;
        let connection = client
            .get_multiplexed_async_connection()
            .map_err(PersistenceError::by_backend)
            .await?;

        Ok(RedisReminderDb { connection })
    }

    pub async fn count(&self) -> Result<usize, PersistenceError> {
        let mut conn = self.connection.clone();
        let count: usize = conn.hlen(JOB_TABLE_KEY).map_err(PersistenceError::by_backend).await?;
        Ok(count)
    }

    pub async fn enqueue_job<T>(&self, job: &T, execute_at: UtcDateTime) -> Result<Uuid, PersistenceError>
    where
        T: Serialize,
    {
        let mut conn = self.connection.clone();

        let id = Uuid::new_v4();
        let id_str = id.to_string();

        // ジョブ本体を登録
        let job_bytes = serde_json::to_vec(job).map_err(PersistenceError::by_serialization)?;
        let _: Value = conn
            .hset(JOB_TABLE_KEY, &id_str, job_bytes)
            .map_err(PersistenceError::by_backend)
            .await?;

        // キューに時刻(unixtime)とのマッピングを登録
        let score = (execute_at.unix_timestamp_nanos() / 1_000_000) as f64 / 1000.0;
        let _: Value = conn
            .zadd(QUEUE_KEY, &id_str, score)
            .map_err(PersistenceError::by_backend)
            .await?;

        Ok(id)
    }

    pub async fn remove_job(&self, id: Uuid) -> Result<(), PersistenceError> {
        let mut conn = self.connection.clone();

        let id_str = id.to_string();

        // ジョブ本体を削除
        let _: Value = conn
            .hdel(JOB_TABLE_KEY, &id_str)
            .map_err(PersistenceError::by_backend)
            .await?;

        // キューから削除
        let _: Value = conn
            .zrem(QUEUE_KEY, &id_str)
            .map_err(PersistenceError::by_backend)
            .await?;

        Ok(())
    }

    pub async fn pull_jobs_until<T>(&self, datetime_until: UtcDateTime) -> Result<Vec<(Uuid, T)>, PersistenceError>
    where
        T: DeserializeOwned,
    {
        let mut conn = self.connection.clone();

        let score = (datetime_until.unix_timestamp_nanos() / 1_000_000) as f64 / 1000.0;
        let target_job_count: isize = conn
            .zcount(QUEUE_KEY, f64::NEG_INFINITY, score)
            .map_err(PersistenceError::by_backend)
            .await?;
        trace!("pulling {target_job_count} jobs");

        let job_ids: Vec<(String, f64)> = conn
            .zpopmin(QUEUE_KEY, target_job_count)
            .map_err(PersistenceError::by_backend)
            .await?;
        let mut jobs = vec![];
        for (job_id, _target_time) in job_ids {
            debug!("pulling {job_id}");

            // MEMO: Valkey にも HGETDEL が実装されたらそっちを使う
            let job_bytes: Vec<u8> = conn
                .hget(JOB_TABLE_KEY, &job_id)
                .map_err(PersistenceError::by_backend)
                .await?;
            let _: Value = conn
                .hdel(JOB_TABLE_KEY, &job_id)
                .map_err(PersistenceError::by_backend)
                .await?;

            let job = serde_json::from_slice(&job_bytes).map_err(PersistenceError::by_serialization)?;
            let job_uuid = job_id.parse().map_err(PersistenceError::by_serialization)?;
            jobs.push((job_uuid, job));
        }
        Ok(jobs)
    }
}
