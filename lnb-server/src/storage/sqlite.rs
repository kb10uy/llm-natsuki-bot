use crate::config::AppConfigStorageSqlite;

use std::sync::Arc;

use futures::{FutureExt, TryFutureExt, future::BoxFuture};
use lnb_core::{
    error::StorageError,
    interface::storage::ConversationStorage,
    model::conversation::{Conversation, ConversationId},
};
use sqlx::{SqlitePool, prelude::FromRow};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SqliteConversationStorage(Arc<SqliteConversationStorageInner>);

impl SqliteConversationStorage {
    pub async fn new(config: &AppConfigStorageSqlite) -> Result<SqliteConversationStorage, StorageError> {
        let pool = SqlitePool::connect(&config.filepath.to_string_lossy())
            .map_err(StorageError::by_backend)
            .await?;
        Ok(SqliteConversationStorage(Arc::new(SqliteConversationStorageInner {
            pool,
        })))
    }
}

impl ConversationStorage for SqliteConversationStorage {
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
    pool: SqlitePool,
}

impl SqliteConversationStorageInner {
    async fn fetch_content_by_id(&self, id: ConversationId) -> Result<Option<Conversation>, StorageError> {
        let row: Option<SqliteRowConversation> =
            sqlx::query_as(r#"SELECT id, context_key, content FROM conversations WHERE id = ?;"#)
                .bind(id.0)
                .fetch_optional(&self.pool)
                .map_err(StorageError::by_backend)
                .await?;

        let conversation = row
            .map(|r| rmp_serde::from_slice(&r.content))
            .transpose()
            .map_err(StorageError::by_serialization)?;
        Ok(conversation)
    }

    async fn fetch_content_by_context_key<'a>(
        &'a self,
        context_key: &'a str,
    ) -> Result<Option<Conversation>, StorageError> {
        let row: Option<SqliteRowConversation> =
            sqlx::query_as(r#"SELECT id, context_key, content FROM conversations WHERE context_key = ?;"#)
                .bind(context_key)
                .fetch_optional(&self.pool)
                .map_err(StorageError::by_backend)
                .await?;

        let conversation = row
            .map(|r| rmp_serde::from_slice(&r.content))
            .transpose()
            .map_err(StorageError::by_serialization)?;
        Ok(conversation)
    }

    async fn fetch_id_by_context_key<'a>(
        &'a self,
        context_key: &'a str,
    ) -> Result<Option<ConversationId>, StorageError> {
        let row: Option<(Uuid,)> = sqlx::query_as(r#"SELECT id FROM conversations WHERE context_key = ?;"#)
            .bind(context_key)
            .fetch_optional(&self.pool)
            .map_err(StorageError::by_backend)
            .await?;

        Ok(row.map(|r| ConversationId(r.0)))
    }

    async fn upsert<'a>(
        &'a self,
        conversation: &'a Conversation,
        context_key: Option<&'a str>,
    ) -> Result<(), StorageError> {
        let id = conversation.id().0;
        let blob = rmp_serde::to_vec(conversation).map_err(StorageError::by_serialization)?;

        sqlx::query(
            r#"
            INSERT INTO conversations (id, context_key, content) VALUES (?, ?, ?)
            ON CONFLICT DO UPDATE SET content = excluded.content, context_key = excluded.context_key;
        "#,
        )
        .bind(id)
        .bind(context_key)
        .bind(blob)
        .execute(&self.pool)
        .map_err(StorageError::by_backend)
        .await?;
        Ok(())
    }
}

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
struct SqliteRowConversation {
    id: Uuid,
    context_key: Option<String>,
    content: Vec<u8>,
}
