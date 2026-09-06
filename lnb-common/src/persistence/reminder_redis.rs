use crate::{config::reminder::ConfigReminder, persistence::PersistenceError};

use futures::TryFutureExt;
use redis::{AsyncCommands, Client, Script, Value, aio::MultiplexedConnection};
use serde::{Serialize, de::DeserializeOwned};
use time::UtcDateTime;
use tracing::{debug, trace};
use uuid::Uuid;

const JOB_TABLE_KEY: &str = "lnb_jobs";
const QUEUE_KEY: &str = "lnb_queue";
const PROCESSING_KEY: &str = "lnb_processing";

const CLAIM_JOBS_SCRIPT: &str = r#"
local expired = redis.call('ZRANGEBYSCORE', KEYS[2], '-inf', ARGV[1], 'LIMIT', 0, ARGV[3])
for _, id in ipairs(expired) do
    redis.call('ZREM', KEYS[2], id)
    redis.call('ZADD', KEYS[1], ARGV[1], id)
end

local jobs = redis.call('ZRANGEBYSCORE', KEYS[1], '-inf', ARGV[1], 'LIMIT', 0, ARGV[3])
for _, id in ipairs(jobs) do
    redis.call('ZREM', KEYS[1], id)
    redis.call('ZADD', KEYS[2], ARGV[2], id)
end
return jobs
"#;

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

        let id = Uuid::now_v7();
        let id_str = id.to_string();

        let job_bytes = serde_json::to_vec(job).map_err(PersistenceError::by_serialization)?;
        let score = datetime_score(execute_at);
        let _: () = redis::pipe()
            .atomic()
            .hset(JOB_TABLE_KEY, &id_str, job_bytes)
            .ignore()
            .zadd(QUEUE_KEY, &id_str, score)
            .ignore()
            .query_async(&mut conn)
            .map_err(PersistenceError::by_backend)
            .await?;

        Ok(id)
    }

    pub async fn remove_job(&self, id: Uuid) -> Result<(), PersistenceError> {
        let mut conn = self.connection.clone();

        let id_str = id.to_string();

        let _: () = redis::pipe()
            .atomic()
            .hdel(JOB_TABLE_KEY, &id_str)
            .ignore()
            .zrem(QUEUE_KEY, &id_str)
            .ignore()
            .zrem(PROCESSING_KEY, &id_str)
            .ignore()
            .query_async(&mut conn)
            .map_err(PersistenceError::by_backend)
            .await?;

        Ok(())
    }

    pub async fn claim_jobs_until<T>(
        &self,
        datetime_until: UtcDateTime,
        lease_until: UtcDateTime,
        limit: usize,
    ) -> Result<Vec<(Uuid, T)>, PersistenceError>
    where
        T: DeserializeOwned,
    {
        let mut conn = self.connection.clone();

        let job_ids: Vec<String> = Script::new(CLAIM_JOBS_SCRIPT)
            .key(QUEUE_KEY)
            .key(PROCESSING_KEY)
            .arg(datetime_score(datetime_until))
            .arg(datetime_score(lease_until))
            .arg(limit)
            .invoke_async(&mut conn)
            .map_err(PersistenceError::by_backend)
            .await?;
        trace!("claimed {} jobs", job_ids.len());

        let mut jobs = vec![];
        for job_id in job_ids {
            debug!("claimed {job_id}");

            let job_bytes: Option<Vec<u8>> = conn
                .hget(JOB_TABLE_KEY, &job_id)
                .map_err(PersistenceError::by_backend)
                .await?;
            let Some(job_bytes) = job_bytes else {
                let _: Value = conn
                    .zrem(PROCESSING_KEY, &job_id)
                    .map_err(PersistenceError::by_backend)
                    .await?;
                continue;
            };

            let job = serde_json::from_slice(&job_bytes).map_err(PersistenceError::by_serialization)?;
            let job_uuid = job_id.parse().map_err(PersistenceError::by_serialization)?;
            jobs.push((job_uuid, job));
        }
        Ok(jobs)
    }

    pub async fn acknowledge_job(&self, id: Uuid) -> Result<(), PersistenceError> {
        let mut conn = self.connection.clone();
        let id_str = id.to_string();
        let _: () = redis::pipe()
            .atomic()
            .hdel(JOB_TABLE_KEY, &id_str)
            .ignore()
            .zrem(PROCESSING_KEY, &id_str)
            .ignore()
            .query_async(&mut conn)
            .map_err(PersistenceError::by_backend)
            .await?;
        Ok(())
    }

    pub async fn retry_job(&self, id: Uuid, execute_at: UtcDateTime) -> Result<(), PersistenceError> {
        let mut conn = self.connection.clone();
        let id_str = id.to_string();
        let _: () = redis::pipe()
            .atomic()
            .zrem(PROCESSING_KEY, &id_str)
            .ignore()
            .zadd(QUEUE_KEY, &id_str, datetime_score(execute_at))
            .ignore()
            .query_async(&mut conn)
            .map_err(PersistenceError::by_backend)
            .await?;
        Ok(())
    }
}

fn datetime_score(datetime: UtcDateTime) -> f64 {
    (datetime.unix_timestamp_nanos() / 1_000_000) as f64 / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    use time::Duration;

    #[tokio::test]
    #[ignore = "requires a disposable Redis instance in LNB_TEST_REDIS_URL"]
    async fn claimed_job_is_retried_and_acknowledged() {
        let redis_address = std::env::var("LNB_TEST_REDIS_URL").expect("LNB_TEST_REDIS_URL must be set");
        let config = ConfigReminder {
            redis_address,
            max_seconds: 60,
            notification_virtual_text: String::new(),
        };
        let db = RedisReminderDb::connect(&config).await.unwrap();
        let mut connection = db.connection.clone();
        let _: () = redis::cmd("FLUSHDB").query_async(&mut connection).await.unwrap();

        let now = UtcDateTime::now();
        let id = db.enqueue_job(&"payload", now).await.unwrap();
        let claimed = db
            .claim_jobs_until::<String>(now + Duration::seconds(1), now + Duration::minutes(1), 10)
            .await
            .unwrap();
        assert_eq!(claimed, vec![(id, "payload".to_string())]);
        assert!(
            db.claim_jobs_until::<String>(now + Duration::seconds(1), now + Duration::minutes(1), 10)
                .await
                .unwrap()
                .is_empty()
        );

        db.retry_job(id, now).await.unwrap();
        let retried = db
            .claim_jobs_until::<String>(now + Duration::seconds(1), now + Duration::minutes(1), 10)
            .await
            .unwrap();
        assert_eq!(retried, vec![(id, "payload".to_string())]);

        db.acknowledge_job(id).await.unwrap();
        assert_eq!(db.count().await.unwrap(), 0);
    }
}
