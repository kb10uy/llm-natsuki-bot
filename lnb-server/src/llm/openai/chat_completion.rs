use crate::{
    config::AppConfigLlmOpenai,
    llm::{
        convert_json_schema,
        openai::{RESPONSE_JSON_SCHEMA, create_openai_client},
    },
};

use std::sync::Arc;

use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{
        ChatCompletionMessageToolCall, ChatCompletionRequestAssistantMessage, ChatCompletionRequestMessage,
        ChatCompletionRequestMessageContentPartImage, ChatCompletionRequestMessageContentPartText,
        ChatCompletionRequestToolMessage, ChatCompletionRequestToolMessageContent, ChatCompletionRequestUserMessage,
        ChatCompletionRequestUserMessageContent, ChatCompletionRequestUserMessageContentPart, ChatCompletionTool,
        ChatCompletionToolType, CreateChatCompletionRequest, FunctionCall, FunctionObject, ImageUrl, ResponseFormat,
    },
};
use futures::{FutureExt, TryFutureExt, future::BoxFuture};
use lnb_core::{
    error::LlmError,
    interface::{
        function::simple::SimpleFunctionDescriptor,
        llm::{Llm, LlmAssistantResponse, LlmUpdate},
    },
    model::{
        conversation::IncompleteConversation,
        message::{Message, MessageFunctionCall, UserMessageContent},
    },
};
use tokio::sync::Mutex;

/// OpenAI Chat Completion API を利用したバックエンド。
#[derive(Debug, Clone)]
pub struct ChatCompletionBackend(Arc<ChatCompletionBackendInner>);

impl ChatCompletionBackend {
    pub async fn new(config: &AppConfigLlmOpenai) -> Result<ChatCompletionBackend, LlmError> {
        let client = create_openai_client(config).await?;
        let model = config.model.clone();

        Ok(ChatCompletionBackend(Arc::new(ChatCompletionBackendInner {
            client,
            tools: Mutex::new(vec![]),
            model,
            max_token: config.max_token,
            structured_mode: config.use_structured_output,
        })))
    }
}

impl Llm for ChatCompletionBackend {
    fn add_simple_function(&self, descriptor: SimpleFunctionDescriptor) -> BoxFuture<'_, ()> {
        async { self.0.add_simple_function(descriptor).await }.boxed()
    }

    fn send_conversation<'a>(
        &'a self,
        conversation: &'a IncompleteConversation,
    ) -> BoxFuture<'a, Result<LlmUpdate, LlmError>> {
        let cloned = self.0.clone();
        async move { cloned.send_conversation(conversation).await }.boxed()
    }
}

#[derive(Debug)]
struct ChatCompletionBackendInner {
    client: Client<OpenAIConfig>,
    tools: Mutex<Vec<ChatCompletionTool>>,
    model: String,
    max_token: usize,
    structured_mode: bool,
}

impl ChatCompletionBackendInner {
    async fn add_simple_function(&self, descriptor: SimpleFunctionDescriptor) {
        let tool = ChatCompletionTool {
            function: FunctionObject {
                name: descriptor.name,
                description: Some(descriptor.description),
                parameters: Some(convert_json_schema(&descriptor.parameters)),
                strict: Some(true),
            },
            ..Default::default()
        };

        let mut locked = self.tools.lock().await;
        locked.push(tool);
    }

    async fn send_conversation(&self, conversation: &IncompleteConversation) -> Result<LlmUpdate, LlmError> {
        let messages: Result<_, _> = conversation.latest_messages.iter().map(transform_message).collect();
        if self.structured_mode {
            self.send_conversation_structured(messages?).await
        } else {
            self.send_conversation_normal(messages?).await
        }
    }

    async fn send_conversation_normal(
        &self,
        messages: Vec<ChatCompletionRequestMessage>,
    ) -> Result<LlmUpdate, LlmError> {
        let request = CreateChatCompletionRequest {
            messages,
            tools: Some(self.tools.lock().await.clone()),
            model: self.model.clone(),
            max_completion_tokens: Some(self.max_token as u32),
            ..Default::default()
        };

        let openai_response = self.client.chat().create(request).map_err(LlmError::by_backend).await?;
        let Some(first_choice) = openai_response.choices.into_iter().next() else {
            return Err(LlmError::NoChoice);
        };

        let tool_callings = match first_choice.message.tool_calls {
            Some(calls) => {
                let converted_calls: Result<Vec<_>, _> = calls
                    .into_iter()
                    .map(|c| {
                        serde_json::from_str(&c.function.arguments).map(|args| MessageFunctionCall {
                            id: c.id,
                            name: c.function.name,
                            arguments: args,
                        })
                    })
                    .collect();
                Some(converted_calls.map_err(LlmError::by_format)?)
            }
            None => None,
        };

        let update = LlmUpdate {
            response: first_choice.message.content.map(|text| LlmAssistantResponse {
                text,
                language: None,
                sensitive: None,
            }),
            tool_callings,
        };
        Ok(update)
    }

    async fn send_conversation_structured(
        &self,
        messages: Vec<ChatCompletionRequestMessage>,
    ) -> Result<LlmUpdate, LlmError> {
        let request = CreateChatCompletionRequest {
            messages,
            tools: Some(self.tools.lock().await.clone()),
            model: self.model.clone(),
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

        let tool_callings = match first_choice.message.tool_calls {
            Some(calls) => {
                let converted_calls: Result<Vec<_>, _> = calls
                    .into_iter()
                    .map(|c| {
                        serde_json::from_str(&c.function.arguments).map(|args| MessageFunctionCall {
                            id: c.id,
                            name: c.function.name,
                            arguments: args,
                        })
                    })
                    .collect();
                Some(converted_calls.map_err(LlmError::by_format)?)
            }
            None => None,
        };

        let response = first_choice
            .message
            .content
            .map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(|e| LlmError::ResponseFormat(e.into()))?;

        let update = LlmUpdate {
            response,
            tool_callings,
        };
        Ok(update)
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
