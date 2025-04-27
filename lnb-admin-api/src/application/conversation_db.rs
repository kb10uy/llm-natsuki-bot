use crate::{application::error::ApplicationError, config::ConfigStorageSqlite};

use futures::TryFutureExt;
use lnb_core::model::conversation::Conversation;
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

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

    pub async fn count(&self) -> Result<usize, ApplicationError> {
        let count: (u64,) = sqlx::query_as(r#"SELECT COUNT(*) FROM conversations;"#)
            .fetch_one(&self.pool)
            .map_err(ApplicationError::by_backend)
            .await?;
        Ok(count.0 as usize)
    }

    pub async fn show(&self, id: Uuid) -> Result<Conversation, ApplicationError> {
        let row: SqliteRowConversation =
            sqlx::query_as(r#"SELECT id, context_key, content FROM conversations WHERE id = ?;"#)
                .bind(id)
                .fetch_one(&self.pool)
                .map_err(ApplicationError::by_backend)
                .await?;
        let conversation = serde_json::from_slice(&row.content).map_err(ApplicationError::by_serialization)?;
        Ok(conversation)
    }

    pub async fn latest_ids(&self, count: usize, max_id: Option<Uuid>) -> Result<Vec<Uuid>, ApplicationError> {
        let max = max_id.unwrap_or(Uuid::max());
        let rows: Vec<(Uuid,)> =
            sqlx::query_as(r#"SELECT id FROM conversations WHERE id < ? ORDER BY id DESC LIMIT ?;"#)
                .bind(max)
                .bind(count as i64)
                .fetch_all(&self.pool)
                .map_err(ApplicationError::by_backend)
                .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    pub async fn earliest_ids(&self, count: usize, min_id: Option<Uuid>) -> Result<Vec<Uuid>, ApplicationError> {
        let min = min_id.unwrap_or(Uuid::max());
        let rows: Vec<(Uuid,)> =
            sqlx::query_as(r#"SELECT id FROM conversations WHERE id > ? ORDER BY id ASC LIMIT ?;"#)
                .bind(min)
                .bind(count as i64)
                .fetch_all(&self.pool)
                .map_err(ApplicationError::by_backend)
                .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }
}

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
struct SqliteRowConversation {
    id: Uuid,
    context_key: Option<String>,
    content: Vec<u8>,
}
