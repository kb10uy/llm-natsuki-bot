mod function;

pub use function::Reminder;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ReminderConfig {
    max_seconds: i64,
    notification_virtual_text: String,
}
