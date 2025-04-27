use crate::{application::error::ApplicationError, config::ConfigStorageSqlite};

use futures::TryFutureExt;
use sqlx::SqlitePool;

#[derive(Debug, Clone)]
pub struct ConversationDb {
    pool: SqlitePool,
}

impl ConversationDb {
    pub async fn connect(config: &ConfigStorageSqlite) -> Result<ConversationDb, ApplicationError> {
        let pool = SqlitePool::connect(&config.filepath.to_string_lossy())
            .map_err(ApplicationError::by_backend)
            .await?;
        Ok(ConversationDb { pool })
    }
}
