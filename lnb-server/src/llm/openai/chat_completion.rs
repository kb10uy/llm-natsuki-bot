use crate::llm::{
    convert_json_schema,
    openai::{OpenaiModelConfig, RESPONSE_JSON_SCHEMA, create_openai_client},
};

use std::sync::Arc;

use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{
        ChatChoice, ChatCompletionMessageToolCall, ChatCompletionRequestAssistantMessage, ChatCompletionRequestMessage,
        ChatCompletionRequestMessageContentPartImage, ChatCompletionRequestMessageContentPartText,
        ChatCompletionRequestToolMessage, ChatCompletionRequestToolMessageContent, ChatCompletionRequestUserMessage,
        ChatCompletionRequestUserMessageContent, ChatCompletionRequestUserMessageContentPart, ChatCompletionTool,
        ChatCompletionToolType, CreateChatCompletionRequest, FinishReason, FunctionCall, FunctionObject, ImageUrl,
        ResponseFormat,
    },
};
use futures::{FutureExt, TryFutureExt, future::BoxFuture};
use lnb_core::{
    error::LlmError,
    interface::{
        function::FunctionDescriptor,
        llm::{Llm, LlmAssistantResponse, LlmUpdate},
    },
    model::{
        conversation::IncompleteConversation,
        message::{Message, MessageToolCalling, UserMessageContent},
    },
};
use tracing::warn;

/// OpenAI Chat Completion API を利用したバックエンド。
#[derive(Debug, Clone)]
pub struct ChatCompletionBackend(Arc<ChatCompletionBackendInner>);

impl ChatCompletionBackend {
    pub async fn new(config: OpenaiModelConfig) -> Result<ChatCompletionBackend, LlmError> {
        let client = create_openai_client(&config.token, &config.endpoint).await?;
        Ok(ChatCompletionBackend(Arc::new(ChatCompletionBackendInner {
            client,
            model: config.model,
            enable_tool: config.tool,
            structured: config.structured,
            max_token: config.max_token,
        })))
    }
}

impl Llm for ChatCompletionBackend {
    fn send_conversation<'a>(
        &'a self,
        conversation: &'a IncompleteConversation,
        function_descriptors: &'a [&'a FunctionDescriptor],
    ) -> BoxFuture<'a, Result<LlmUpdate, LlmError>> {
        let cloned = self.0.clone();
        async move { cloned.send_conversation(conversation, function_descriptors).await }.boxed()
    }
}

#[derive(Debug)]
struct ChatCompletionBackendInner {
    client: Client<OpenAIConfig>,
    model: String,
    enable_tool: bool,
    structured: bool,
    max_token: usize,
}

impl ChatCompletionBackendInner {
    async fn send_conversation(
        &self,
        conversation: &IncompleteConversation,
        function_descriptors: &[&FunctionDescriptor],
    ) -> Result<LlmUpdate, LlmError> {
        let messages: Result<_, _> = conversation.llm_sending_messages().map(transform_message).collect();
        if self.structured {
            self.send_conversation_structured(messages?, function_descriptors).await
        } else {
            self.send_conversation_normal(messages?, function_descriptors).await
        }
    }

    async fn send_conversation_normal(
        &self,
        messages: Vec<ChatCompletionRequestMessage>,
        function_descriptors: &[&FunctionDescriptor],
    ) -> Result<LlmUpdate, LlmError> {
        let request = CreateChatCompletionRequest {
            model: self.model.clone(),
            messages,
            tools: self.enable_tool.then(|| transform_tools(function_descriptors)),
            max_completion_tokens: Some(self.max_token as u32),
            ..Default::default()
        };

        let openai_response = self.client.chat().create(request).map_err(LlmError::by_backend).await?;
        let Some(first_choice) = openai_response.choices.into_iter().next() else {
            return Err(LlmError::NoChoice);
        };

        transform_choice(first_choice)
    }

    async fn send_conversation_structured(
        &self,
        messages: Vec<ChatCompletionRequestMessage>,
        function_descriptors: &[&FunctionDescriptor],
    ) -> Result<LlmUpdate, LlmError> {
        let request = CreateChatCompletionRequest {
            model: self.model.clone(),
            messages,
            tools: self.enable_tool.then(|| transform_tools(function_descriptors)),
            response_format: Some(ResponseFormat::JsonSchema {
                json_schema: RESPONSE_JSON_SCHEMA.clone(),
            }),
            max_completion_tokens: Some(self.max_token as u32),
            ..Default::default()
        };

        let openai_response = self.client.chat().create(request).map_err(LlmError::by_backend).await?;
        let Some(first_choice) = openai_response.choices.into_iter().next() else {
            return Err(LlmError::NoChoice);
        };

        transform_choice(first_choice)
    }
}

fn transform_tools(descriptors: &[&FunctionDescriptor]) -> Vec<ChatCompletionTool> {
    descriptors
        .iter()
        .map(|d| ChatCompletionTool {
            function: FunctionObject {
                name: d.name.clone(),
                description: Some(d.description.clone()),
                parameters: Some(convert_json_schema(&d.parameters)),
                strict: Some(true),
            },
            ..Default::default()
        })
        .collect()
}

fn transform_choice(choice: ChatChoice) -> Result<LlmUpdate, LlmError> {
    // Reason 関係なく tool_calls が埋まることがあるので先頭で判定する
    let tool_calls = choice.message.tool_calls.clone().unwrap_or_default();
    if !tool_calls.is_empty() {
        let converted_calls: Result<_, _> = tool_calls
            .into_iter()
            .map(|c| {
                let arguments = serde_json::from_str(&c.function.arguments).map_err(LlmError::by_format)?;
                Ok(MessageToolCalling {
                    id: c.id,
                    name: c.function.name,
                    arguments,
                })
            })
            .collect();
        return Ok(LlmUpdate::ToolCalling(converted_calls?));
    }

    match choice.finish_reason {
        // stop
        Some(FinishReason::Stop) => {
            let Some(text) = choice.message.content else {
                warn!("no content value detected; actual value: {choice:?}");
                return Err(LlmError::ExpectationMismatch("no content value".to_string()));
            };
            Ok(LlmUpdate::Finished(LlmAssistantResponse {
                text,
                language: None,
                sensitive: None,
            }))
        }

        // max_length
        Some(FinishReason::Length) => {
            let Some(text) = choice.message.content else {
                warn!("no content value detected; actual value: {choice:?}");
                return Err(LlmError::ExpectationMismatch("no content value".to_string()));
            };
            Ok(LlmUpdate::LengthCut(LlmAssistantResponse {
                text,
                language: None,
                sensitive: None,
            }))
        }

        // tool_calls
        // 先頭で抜けてるので来ないはずだけど一応
        Some(FinishReason::ToolCalls) => Ok(LlmUpdate::ToolCalling(vec![])),

        // content_filter
        Some(FinishReason::ContentFilter) => Ok(LlmUpdate::Filtered),

        // other invalid values
        Some(FinishReason::FunctionCall) => {
            Err(LlmError::ExpectationMismatch("function call not expected".to_string()))
        }
        None => Err(LlmError::NoChoice),
    }
}

fn transform_message(message: &Message) -> Result<ChatCompletionRequestMessage, LlmError> {
    let message = match message {
        Message::System(system_message) => ChatCompletionRequestMessage::System(system_message.0.clone().into()),
        Message::User(user_message) => {
            let contents =
                user_message
                    .contents
                    .iter()
                    .map(|umc| match umc {
                        UserMessageContent::Text(text) => ChatCompletionRequestUserMessageContentPart::Text(
                            ChatCompletionRequestMessageContentPartText { text: text.to_string() },
                        ),
                        UserMessageContent::ImageUrl(url) => ChatCompletionRequestUserMessageContentPart::ImageUrl(
                            ChatCompletionRequestMessageContentPartImage {
                                image_url: ImageUrl {
                                    url: url.to_string(),
                                    ..Default::default()
                                },
                            },
                        ),
                    })
                    .collect();
            ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                content: ChatCompletionRequestUserMessageContent::Array(contents),
                name: user_message.name.clone(),
            })
        }
        Message::Assistant(assistant_message) => {
            ChatCompletionRequestMessage::Assistant(assistant_message.text.clone().into())
        }
        Message::FunctionCalls(function_calls_message) => {
            let tool_calls: Result<_, _> = function_calls_message
                .0
                .iter()
                .map(|c| {
                    serde_json::to_string(&c.arguments).map(|args| ChatCompletionMessageToolCall {
                        id: c.id.clone(),
                        function: FunctionCall {
                            name: c.name.clone(),
                            arguments: args,
                        },
                        r#type: ChatCompletionToolType::Function,
                    })
                })
                .collect();
            ChatCompletionRequestMessage::Assistant(ChatCompletionRequestAssistantMessage {
                tool_calls: Some(tool_calls.map_err(LlmError::by_format)?),
                ..Default::default()
            })
        }
        Message::FunctionResponse(function_response_message) => {
            let json = serde_json::to_string(&function_response_message.result).map_err(LlmError::by_format)?;
            ChatCompletionRequestMessage::Tool(ChatCompletionRequestToolMessage {
                tool_call_id: function_response_message.id.clone(),
                content: ChatCompletionRequestToolMessageContent::Text(json),
            })
        }
    };
    Ok(message)
}
