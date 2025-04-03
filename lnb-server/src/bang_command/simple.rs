use lnb_core::{
    error::LlmError,
    interface::Context,
    model::{conversation::UserRole, message::AssistantMessage},
};

pub fn ping(_ctx: &Context, _rest_text: &str, _role: &UserRole) -> Result<AssistantMessage, LlmError> {
    Ok(AssistantMessage {
        text: "pong".to_string(),
        ..Default::default()
    })
}
