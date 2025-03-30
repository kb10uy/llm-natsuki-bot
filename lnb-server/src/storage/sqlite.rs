use crate::config::AppConfigStorageSqlite;

use std::sync::Arc;

use futures::{FutureExt, TryFutureExt, future::BoxFuture};
use lnb_core::{error::StorageError, interface::storage::ConversationStorage, model::conversation::Conversation};
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
    fn find_by_id<'a>(&'a self, id: &'a Uuid) -> BoxFuture<'a, Result<Option<Conversation>, StorageError>> {
        async move { self.0.find_by_id(id).await }.boxed()
    }

    fn find_by_platform_context<'a>(
        &'a self,
        platform: &'a str,
        context: &'a str,
    ) -> BoxFuture<'a, Result<Option<Conversation>, StorageError>> {
        async move { self.0.find_by_platform_context(platform, context).await }.boxed()
    }

    fn upsert<'a>(
        &'a self,
        conversation: &'a Conversation,
        platform: &'a str,
        new_context: &'a str,
    ) -> BoxFuture<'a, Result<(), StorageError>> {
        async move { self.0.upsert(conversation, platform, new_context).await }.boxed()
    }
}

#[derive(Debug)]
struct SqliteConversationStorageInner {
    pool: SqlitePool,
}

impl SqliteConversationStorageInner {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<Conversation>, StorageError> {
        let row: Option<SqliteRowConversation> =
            sqlx::query_as(r#"SELECT id, conversation_blob FROM conversations WHERE id = ?"#)
                .bind(id)
                .fetch_optional(&self.pool)
                .map_err(StorageError::by_backend)
                .await?;

        let conversation = row
            .map(|r| rmp_serde::from_slice(&r.conversation_blob))
            .transpose()
            .map_err(StorageError::by_serialization)?;
        Ok(conversation)
    }

    async fn find_by_platform_context(
        &self,
        platform: &str,
        context: &str,
    ) -> Result<Option<Conversation>, StorageError> {
        let platform_context_row = sqlx::query_as(
            r#"SELECT conversation_id, platform, context FROM platform_contexts WHERE platform = ? AND context = ?"#,
        )
        .bind(platform)
        .bind(context)
        .fetch_optional(&self.pool)
        .map_err(StorageError::by_backend)
        .await?;
        let Some(SqliteRowPlatformContext { conversation_id, .. }) = platform_context_row else {
            return Ok(None);
        };

        self.find_by_id(&conversation_id).await
    }

    async fn upsert(&self, conversation: &Conversation, platform: &str, new_context: &str) -> Result<(), StorageError> {
        let blob = rmp_serde::to_vec(conversation).map_err(StorageError::by_serialization)?;

        sqlx::query(r#"INSERT INTO conversations (id, conversation_blob) VALUES (?, ?) ON CONFLICT DO UPDATE SET conversation_blob = excluded.conversation_blob;"#)
            .bind(conversation.id())
            .bind(blob)
            .execute(&self.pool)
            .map_err(StorageError::by_backend)
            .await?;
        sqlx::query(r#"INSERT INTO platform_contexts (conversation_id, platform, context) VALUES (?, ?, ?) ON CONFLICT DO UPDATE SET context = excluded.context;"#)
            .bind(conversation.id())
            .bind(platform)
            .bind(new_context)
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
    conversation_blob: Vec<u8>,
}

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
struct SqliteRowPlatformContext {
    conversation_id: Uuid,
    platform: String,
    context: String,
}
