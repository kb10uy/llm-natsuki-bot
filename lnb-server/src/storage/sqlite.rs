use std::sync::Arc;

use futures::{FutureExt, TryFutureExt, future::BoxFuture};
use lnb_common::{config::storage::ConfigStorageSqlite, persistence::SqliteConversationDb};
use lnb_core::{
    error::StorageError,
    interface::storage::ConversationStorage,
    model::conversation::{Conversation, ConversationId},
};

#[derive(Debug, Clone)]
pub struct SqliteConversationStorage(Arc<SqliteConversationStorageInner>);

impl SqliteConversationStorage {
    pub async fn new(config: &ConfigStorageSqlite) -> Result<SqliteConversationStorage, StorageError> {
        let db = SqliteConversationDb::connect(config)
            .map_err(StorageError::by_backend)
            .await?;
        Ok(SqliteConversationStorage(Arc::new(SqliteConversationStorageInner {
            db,
        })))
    }
}

impl ConversationStorage for SqliteConversationStorage {
    fn description(&self) -> String {
        "SQLite".to_string()
    }

    fn fetch_content_by_id(&self, id: ConversationId) -> BoxFuture<'_, Result<Option<Conversation>, StorageError>> {
        async move { self.0.fetch_content_by_id(id).await }.boxed()
    }

    fn fetch_content_by_context_key<'a>(
        &'a self,
        context_key: &'a str,
    ) -> BoxFuture<'a, Result<Option<Conversation>, StorageError>> {
        async move { self.0.fetch_content_by_context_key(context_key).await }.boxed()
    }

    fn fetch_id_by_context_key<'a>(
        &'a self,
        context_key: &'a str,
    ) -> BoxFuture<'a, Result<Option<ConversationId>, StorageError>> {
        async move { self.0.fetch_id_by_context_key(context_key).await }.boxed()
    }

    fn upsert<'a>(
        &'a self,
        conversation: &'a Conversation,
        context_key: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), StorageError>> {
        async move { self.0.upsert(conversation, context_key).await }.boxed()
    }
}

#[derive(Debug)]
struct SqliteConversationStorageInner {
    db: SqliteConversationDb,
}

impl SqliteConversationStorageInner {
    async fn fetch_content_by_id(&self, id: ConversationId) -> Result<Option<Conversation>, StorageError> {
        self.db.fetch_by_id(id.0).map_err(StorageError::by_backend).await
    }

    async fn fetch_content_by_context_key<'a>(
        &'a self,
        context_key: &'a str,
    ) -> Result<Option<Conversation>, StorageError> {
        self.db
            .fetch_by_context_key(context_key)
            .map_err(StorageError::by_backend)
            .await
    }

    async fn fetch_id_by_context_key<'a>(
        &'a self,
        context_key: &'a str,
    ) -> Result<Option<ConversationId>, StorageError> {
        self.db
            .fetch_id_by_context_key(context_key)
            .map_ok(|r| r.map(ConversationId))
            .map_err(StorageError::by_backend)
            .await
    }

    async fn upsert<'a>(
        &'a self,
        conversation: &'a Conversation,
        context_key: Option<&'a str>,
    ) -> Result<(), StorageError> {
        self.db
            .upsert(conversation, context_key)
            .map_err(StorageError::by_backend)
            .await
    }
}
