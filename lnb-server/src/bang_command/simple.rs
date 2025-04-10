use crate::bang_command::interception::BangCommandResponse;

use lnb_core::{
    error::LlmError,
    interface::Context,
    model::{conversation::UserRole, message::AssistantMessage},
};

pub fn ping(_ctx: &Context, _rest_text: &str, _role: &UserRole) -> Result<BangCommandResponse, LlmError> {
    Ok(BangCommandResponse {
        message: AssistantMessage {
            text: "pong".to_string(),
            ..Default::default()
        },
        ..Default::default()
    })
}
