use crate::bang_command::BangCommandResponse;

use lnb_core::{
    error::LlmError,
    interface::{Context, interception::InterceptionStatus},
    model::{
        conversation::{ConversationModel, UserRole},
        message::AssistantMessage,
    },
};

pub fn ping(_ctx: &Context, _rest_text: &str, _role: &UserRole) -> Result<BangCommandResponse, LlmError> {
    Ok(BangCommandResponse {
        status: InterceptionStatus::Complete(AssistantMessage {
            text: "pong".to_string(),
            ..Default::default()
        }),
        ..Default::default()
    })
}

pub fn change(_ctx: &Context, rest_text: &str, _role: &UserRole) -> Result<BangCommandResponse, LlmError> {
    if rest_text.is_empty() || rest_text == "default" {
        Ok(BangCommandResponse {
            status: InterceptionStatus::Complete(AssistantMessage {
                text: "model restored to default".to_string(),
                skip_llm: true,
                ..Default::default()
            }),
            model_override: Some(ConversationModel::Default),
        })
    } else {
        Ok(BangCommandResponse {
            status: InterceptionStatus::Complete(AssistantMessage {
                text: format!("model changed to {rest_text}"),
                skip_llm: true,
                ..Default::default()
            }),
            model_override: Some(ConversationModel::Specified(rest_text.to_string())),
        })
    }
}
