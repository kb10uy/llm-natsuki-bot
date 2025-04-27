use crate::application::error::ApplicationError;

use futures::TryFutureExt;
use redis::{Client, aio::MultiplexedConnection};

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
}
