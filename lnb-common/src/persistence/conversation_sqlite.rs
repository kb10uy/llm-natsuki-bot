use crate::{config::storage::ConfigStorageSqlite, persistence::PersistenceError};

use futures::TryFutureExt;
use lnb_core::model::conversation::Conversation;
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SqliteConversationDb {
    pool: SqlitePool,
}

impl SqliteConversationDb {
    pub async fn connect(config: &ConfigStorageSqlite) -> Result<SqliteConversationDb, PersistenceError> {
        let pool = SqlitePool::connect(&config.filepath.to_string_lossy())
            .map_err(PersistenceError::by_backend)
            .await?;
        Ok(SqliteConversationDb { pool })
    }

    pub async fn count(&self) -> Result<usize, PersistenceError> {
        let count: (u64,) = sqlx::query_as(r#"SELECT COUNT(*) FROM conversations;"#)
            .fetch_one(&self.pool)
            .map_err(PersistenceError::by_backend)
            .await?;
        Ok(count.0 as usize)
    }

    pub async fn fetch_by_id(&self, id: Uuid) -> Result<Option<Conversation>, PersistenceError> {
        let row: Option<SqliteRowConversation> =
            sqlx::query_as(r#"SELECT id, context_key, content FROM conversations WHERE id = ?;"#)
                .bind(id)
                .fetch_optional(&self.pool)
                .map_err(PersistenceError::by_backend)
                .await?;
        let conversation = row
            .map(|r| serde_json::from_slice(&r.content))
            .transpose()
            .map_err(PersistenceError::by_serialization)?;
        Ok(conversation)
    }

    pub async fn fetch_by_context_key(&self, context_key: &str) -> Result<Option<Conversation>, PersistenceError> {
        let row: Option<SqliteRowConversation> =
            sqlx::query_as(r#"SELECT id, context_key, content FROM conversations WHERE context_key = ?;"#)
                .bind(context_key)
                .fetch_optional(&self.pool)
                .map_err(PersistenceError::by_backend)
                .await?;
        let conversation = row
            .map(|r| serde_json::from_slice(&r.content))
            .transpose()
            .map_err(PersistenceError::by_serialization)?;
        Ok(conversation)
    }

    pub async fn fetch_id_by_context_key(&self, context_key: &str) -> Result<Option<Uuid>, PersistenceError> {
        let row: Option<(Uuid,)> = sqlx::query_as(r#"SELECT id FROM conversations WHERE context_key = ?;"#)
            .bind(context_key)
            .fetch_optional(&self.pool)
            .map_err(PersistenceError::by_backend)
            .await?;

        Ok(row.map(|r| r.0))
    }

    pub async fn latest_ids(&self, count: usize, max_id: Option<Uuid>) -> Result<Vec<Uuid>, PersistenceError> {
        let max = max_id.unwrap_or(Uuid::max());
        let rows: Vec<(Uuid,)> =
            sqlx::query_as(r#"SELECT id FROM conversations WHERE id < ? ORDER BY id DESC LIMIT ?;"#)
                .bind(max)
                .bind(count as i64)
                .fetch_all(&self.pool)
                .map_err(PersistenceError::by_backend)
                .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    pub async fn earliest_ids(&self, count: usize, min_id: Option<Uuid>) -> Result<Vec<Uuid>, PersistenceError> {
        let min = min_id.unwrap_or(Uuid::max());
        let rows: Vec<(Uuid,)> =
            sqlx::query_as(r#"SELECT id FROM conversations WHERE id > ? ORDER BY id ASC LIMIT ?;"#)
                .bind(min)
                .bind(count as i64)
                .fetch_all(&self.pool)
                .map_err(PersistenceError::by_backend)
                .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    pub async fn upsert(&self, conversation: &Conversation, context_key: Option<&str>) -> Result<(), PersistenceError> {
        let id = conversation.id().0;
        let blob = serde_json::to_vec(conversation).map_err(PersistenceError::by_serialization)?;

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
        .map_err(PersistenceError::by_backend)
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
