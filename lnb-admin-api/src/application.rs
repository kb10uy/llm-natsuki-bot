use lnb_common::persistence::{RedisReminderDb, SqliteConversationDb};

#[derive(Debug, Clone)]
pub struct Application {
    pub conversation: SqliteConversationDb,
    pub reminder: RedisReminderDb,
}
