use crate::application::error::ApplicationError;

#[derive(Debug, Clone)]
pub struct ConversationDb {}

impl ConversationDb {
    pub fn connect() -> Result<ConversationDb, ApplicationError> {
        todo!();
    }
}
