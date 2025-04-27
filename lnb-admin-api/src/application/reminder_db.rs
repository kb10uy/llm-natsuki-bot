use crate::application::error::ApplicationError;

#[derive(Debug, Clone)]
pub struct ReminderDb {}

impl ReminderDb {
    pub fn connect() -> Result<ReminderDb, ApplicationError> {
        todo!();
    }
}
