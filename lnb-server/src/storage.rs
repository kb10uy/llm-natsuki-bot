mod memory;
mod sqlite;

use self::{memory::MemoryConversationStorage, sqlite::SqliteConversationStorage};
use crate::config::{AppConfigStorage, AppConfigStorageBackend};

use lnb_core::{error::StorageError, interface::storage::BoxConversationStorage};

pub async fn initialize_storage(config: &AppConfigStorage) -> Result<BoxConversationStorage, StorageError> {
    match config.backend {
        AppConfigStorageBackend::Memory => Ok(Box::new(MemoryConversationStorage::new())),
        AppConfigStorageBackend::Sqlite => Ok(Box::new(SqliteConversationStorage::new(&config.sqlite).await?)),
    }
}
