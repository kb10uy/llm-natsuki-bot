use crate::llm::{convert_json_schema, openai::OpenaiModelConfig};

use std::sync::Arc;

use futures::{FutureExt, TryFutureExt, future::BoxFuture};
use lnb_core::{
    APP_USER_AGENT,
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
use reqwest::{Client, ClientBuilder, header::HeaderMap};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// OpenAI Responses API を利用したバックエンド。
#[derive(Debug, Clone)]
pub struct ResponsesBackend(Arc<ResponsesBackendInner>);

impl ResponsesBackend {
    pub async fn new(config: OpenaiModelConfig) -> Result<ResponsesBackend, LlmError> {
        let client = {
            let mut headers = HeaderMap::new();
            headers.insert(
                "Authorization",
                format!("Bearer {}", config.token).parse().expect("should pass header"),
            );

            ClientBuilder::new()
                .user_agent(APP_USER_AGENT)
                .default_headers(headers)
                .build()
                .map_err(LlmError::by_communication)?
        };

        Ok(ResponsesBackend(Arc::new(ResponsesBackendInner {
            client,
            api_root: config.endpoint.clone(),
            model: config.model.clone(),
            max_token: config.max_token,
        })))
    }
}

impl Llm for ResponsesBackend {
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
#[allow(dead_code)]
struct ResponsesBackendInner {
    client: Client,
    api_root: String,
    model: String,
    max_token: usize,
}

impl ResponsesBackendInner {
    async fn send_conversation(
        &self,
        conversation: &IncompleteConversation,
        function_descriptors: &[&FunctionDescriptor],
    ) -> Result<LlmUpdate, LlmError> {
        let input: Vec<_> = {
            let unflat: Result<Vec<_>, _> = conversation.llm_sending_messages().map(transform_message).collect();
            unflat?.into_iter().flatten().collect()
        };

        // web search を入れる
        let mut tools = transform_tools(function_descriptors);
        tools.push(json!({"type": "web_search_preview"}));

        let request = json!({
            "model": self.model,
            "input": input,
            "tools": tools,
            "store": false,
        });
        let response_value = self.call_api("/responses", &request).await?;
        let output_objects = response_value["output"].as_array().ok_or(LlmError::NoChoice)?;
        transform_choice(output_objects)
    }

    async fn call_api<T: Serialize>(&self, endpoint: &str, body: &T) -> Result<Value, LlmError> {
        let api_url = format!("{}{endpoint}", self.api_root);
        let response = self
            .client
            .post(api_url)
            .json(body)
            .send()
            .map_err(LlmError::by_communication)
            .await?;

        let response_status = response.status();
        let response_body = response.json().map_err(LlmError::by_format).await?;
        if response_status.is_success() {
            Ok(response_body)
        } else {
            let message = match response_body.pointer("/error/message") {
                Some(Value::String(s)) => s.as_str(),
                _ => "unknown error",
            };
            Err(LlmError::Backend(message.to_string().into()))
        }
    }
}

fn transform_tools(descriptors: &[&FunctionDescriptor]) -> Vec<Value> {
    descriptors
        .iter()
        .map(|d| {
            json!({
                "type": "function",
                "name": d.name,
                "description": d.description,
                "parameters": convert_json_schema(&d.parameters),
            })
        })
        .collect()
}

fn transform_message(message: &Message) -> Result<Vec<Value>, LlmError> {
    let message = match message {
        Message::System(system_message) => vec![json!({
            "role": "developer",
            "content": system_message.0,
        })],
        Message::User(user_message) => {
            let contents: Vec<_> = user_message
                .contents
                .iter()
                .map(|umc| match umc {
                    UserMessageContent::Text(text) => json!({
                        "type": "input_text",
                        "text": text,
                    }),
                    UserMessageContent::ImageUrl(url) => json!({
                        "type": "input_image",
                        "image_url": url.to_string(),
                    }),
                })
                .collect();
            vec![json!({
                "role": "user",
                "content": contents,
            })]
        }
        Message::Assistant(assistant_message) => vec![json!({
            "role": "assistant",
            "content": assistant_message.text,
        })],
        Message::FunctionCalls(function_calls_message) => {
            let tool_calls: Result<Vec<_>, _> = function_calls_message
                .0
                .iter()
                .map(|c| {
                    let arguments_str = serde_json::to_string(&c.arguments).map_err(LlmError::by_format)?;
                    Ok(json!({
                        "type": "function_call",
                        "call_id": c.id,
                        "name": c.name,
                        "arguments": arguments_str,
                    }))
                })
                .collect();
            tool_calls?
        }
        Message::FunctionResponse(function_response_message) => {
            let result_str = serde_json::to_string(&function_response_message.result).map_err(LlmError::by_format)?;
            vec![json!({
                "type": "function_call_output",
                "call_id": function_response_message.id,
                "output": result_str,
            })]
        }
    };
    Ok(message)
}

fn transform_choice(outputs: &[Value]) -> Result<LlmUpdate, LlmError> {
    let outputs = {
        let objects: Result<Vec<ResponsesOutput>, _> =
            outputs.iter().map(|v| serde_json::from_value(v.clone())).collect();
        objects.map_err(LlmError::by_format)?
    };

    let mut messages = vec![];
    let mut calls = vec![];
    for output in outputs {
        match output {
            ResponsesOutput::Message(message) => messages.push(message),
            ResponsesOutput::FunctionCall(call) => calls.push(call),
            ResponsesOutput::Unknown => (),
        }
    }

    // function call が 1 つでもあればそれに寄せる
    if !calls.is_empty() {
        let converted_calls: Result<_, _> = calls
            .into_iter()
            .map(|c| {
                let arguments = serde_json::from_str(&c.arguments).map_err(LlmError::by_format)?;
                Ok(MessageToolCalling {
                    id: c.call_id,
                    name: c.name,
                    arguments,
                })
            })
            .collect();
        return Ok(LlmUpdate::ToolCalling(converted_calls?));
    }

    if messages.is_empty() || messages[0].content.is_empty() {
        return Err(LlmError::NoChoice);
    }
    let message = messages.into_iter().next().expect("should have an item");
    let first_content = message.content.into_iter().next().expect("should have an item");
    match first_content {
        ResponsesMessageContent::OutputText { text } => {
            let response = LlmAssistantResponse {
                text,
                language: None,
                sensitive: None,
            };
            match message.status.as_str() {
                "completed" => Ok(LlmUpdate::Finished(response)),
                "incomplete" | "in_progress" => Ok(LlmUpdate::Finished(response)),
                otherwise => {
                    let message = format!("unknown message reason: {otherwise}");
                    Err(LlmError::Backend(message.into()))
                }
            }
        }
        ResponsesMessageContent::Refusal { .. } => Ok(LlmUpdate::Filtered),
        ResponsesMessageContent::Unknown => Err(LlmError::NoChoice),
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
enum ResponsesOutput {
    Message(ResponsesMessage),
    FunctionCall(ResponsesFunctionCall),

    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize)]
struct ResponsesMessage {
    status: String,

    // FXXK: なんでこれ array なん？そっちで繋げろよ
    content: Vec<ResponsesMessageContent>,
}

#[derive(Debug, Clone, Deserialize)]
struct ResponsesFunctionCall {
    call_id: String,
    name: String,
    arguments: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
enum ResponsesMessageContent {
    OutputText {
        text: String,
    },
    Refusal {},

    #[serde(other)]
    Unknown,
}
