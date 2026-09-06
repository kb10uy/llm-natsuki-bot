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

    pub async fn insert(&self, conversation: &Conversation, context_key: Option<&str>) -> Result<(), PersistenceError> {
        let id = conversation.id().0;
        let blob = serde_json::to_vec(conversation).map_err(PersistenceError::by_serialization)?;

        sqlx::query(r#"INSERT INTO conversations (id, context_key, content) VALUES (?, ?, ?);"#)
            .bind(id)
            .bind(context_key)
            .bind(blob)
            .execute(&self.pool)
            .map_err(PersistenceError::by_backend)
            .await?;
        Ok(())
    }

    pub async fn update_if_current(
        &self,
        expected: &Conversation,
        updated: &Conversation,
        context_key: &str,
    ) -> Result<bool, PersistenceError> {
        debug_assert_eq!(expected.id(), updated.id());
        let expected_blob = serde_json::to_vec(expected).map_err(PersistenceError::by_serialization)?;
        let updated_blob = serde_json::to_vec(updated).map_err(PersistenceError::by_serialization)?;

        let result =
            sqlx::query(r#"UPDATE conversations SET context_key = ?, content = ? WHERE id = ? AND content = ?;"#)
                .bind(context_key)
                .bind(updated_blob)
                .bind(updated.id().0)
                .bind(expected_blob)
                .execute(&self.pool)
                .map_err(PersistenceError::by_backend)
                .await?;
        Ok(result.rows_affected() == 1)
    }
}

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
struct SqliteRowConversation {
    id: Uuid,
    context_key: Option<String>,
    content: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    use lnb_core::model::{
        conversation::IncompleteConversation,
        message::{AssistantMessage, Message, UserMessageContent},
    };

    async fn database() -> SqliteConversationDb {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            r#"
            CREATE TABLE conversations(
                id TEXT NOT NULL PRIMARY KEY,
                context_key TEXT NULL UNIQUE,
                content BLOB NOT NULL
            );
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();
        SqliteConversationDb { pool }
    }

    fn updated_from(original: &Conversation, text: &str) -> Conversation {
        let mut incomplete = IncompleteConversation::start(original.clone());
        incomplete.extend_messages([Message::new_user(
            [UserMessageContent::Text(text.to_string())],
            None,
            None,
            false,
        )]);
        incomplete
            .finish(AssistantMessage::default())
            .complete_conversation_with(original.clone())
    }

    #[tokio::test]
    async fn stale_conversation_update_is_rejected() {
        let db = database().await;
        let original = Conversation::new_now(Some(Message::new_system("system")));
        db.insert(&original, None).await.unwrap();

        let first = updated_from(&original, "first");
        let second = updated_from(&original, "second");
        assert!(db.update_if_current(&original, &first, "first").await.unwrap());
        assert!(!db.update_if_current(&original, &second, "second").await.unwrap());
    }
}
