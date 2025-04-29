mod memory;
mod sqlite;

use lnb_common::config::storage::{ConfigStorage, ConfigStorageBackend};
use lnb_core::{error::StorageError, interface::storage::BoxConversationStorage};

pub async fn initialize_storage(config: &ConfigStorage) -> Result<BoxConversationStorage, StorageError> {
    match config.backend {
        ConfigStorageBackend::Memory => Ok(Box::new(memory::MemoryConversationStorage::new())),
        ConfigStorageBackend::Sqlite => Ok(Box::new(sqlite::SqliteConversationStorage::new(&config.sqlite).await?)),
    }
}
