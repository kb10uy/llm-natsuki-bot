mod memory;
mod sqlite;

use self::{memory::MemoryConversationStorage, sqlite::SqliteConversationStorage};
use crate::config::{AppConfigStorage, AppConfigStorageBackend};

use lnb_core::{error::StorageError, interface::storage::ConversationStorage};

pub async fn create_storage(config: &AppConfigStorage) -> Result<Box<dyn ConversationStorage + 'static>, StorageError> {
    let boxed_storage: Box<dyn ConversationStorage> = match config.backend {
        AppConfigStorageBackend::Memory => Box::new(MemoryConversationStorage::new()),
        AppConfigStorageBackend::Sqlite => Box::new(SqliteConversationStorage::new(&config.sqlite).await?),
    };
    Ok(boxed_storage)
}
