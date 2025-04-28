use crate::{config::reminder::ConfigReminder, persistence::PersistenceError};

use futures::TryFutureExt;
use redis::{AsyncCommands, Client, aio::MultiplexedConnection};

const JOB_TABLE_KEY: &str = "lnb_jobs";

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
}
