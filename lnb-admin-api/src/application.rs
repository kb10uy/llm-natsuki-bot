mod conversation_db;
mod error;
mod reminder_db;

pub use conversation_db::ConversationDb;
pub use reminder_db::ReminderDb;

#[derive(Debug, Clone)]
pub struct Application {
    pub conversation: ConversationDb,
    pub reminder: ReminderDb,
}
