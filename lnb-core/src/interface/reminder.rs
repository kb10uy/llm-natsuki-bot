use crate::error::ReminderError;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Remind {
    pub requester: String,
    pub content: String,
}

pub trait Reminder {
    fn register(&self, remind: Remind, remind_at: OffsetDateTime) -> Result<(), ReminderError>;
}
