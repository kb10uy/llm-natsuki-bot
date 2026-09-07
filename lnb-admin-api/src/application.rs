use lnb_persistence_sqlite::SqliteConversationDb;
use lnb_reminder_redis::RedisReminderDb;

#[derive(Debug, Clone)]
pub struct Application {
    pub conversation: SqliteConversationDb,
    pub reminder: RedisReminderDb,
}
