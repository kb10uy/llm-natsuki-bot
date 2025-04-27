use crate::application::error::ApplicationError;

use futures::TryFutureExt;
use redis::{AsyncCommands, Client, aio::MultiplexedConnection};

// const QUEUE_KEY: &str = "lnb_queue";
const JOB_TABLE_KEY: &str = "lnb_jobs";

#[derive(Debug, Clone)]
pub struct ReminderDb {
    connection: MultiplexedConnection,
}

impl ReminderDb {
    pub async fn connect(address: &str) -> Result<ReminderDb, ApplicationError> {
        let client = Client::open(address).map_err(ApplicationError::by_backend)?;
        let connection = client
            .get_multiplexed_async_connection()
            .map_err(ApplicationError::by_backend)
            .await?;

        Ok(ReminderDb { connection })
    }

    pub async fn count(&self) -> Result<usize, ApplicationError> {
        let mut conn = self.connection.clone();
        let count: usize = conn.hlen(JOB_TABLE_KEY).map_err(ApplicationError::by_backend).await?;
        Ok(count)
    }
}
