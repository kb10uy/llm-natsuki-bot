use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

/// `Conversation` 中の単一メッセージ。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    System(SystemMessage),
    User(UserMessage),
    FunctionCalls(FunctionCallsMessage),
    FunctionResponse(FunctionResponseMessage),
    Assistant(AssistantMessage),
}

impl Message {
    pub fn new_system(text: impl Into<String>) -> Message {
        Message::System(SystemMessage(text.into()))
    }

    pub fn new_user(
        contents: impl IntoIterator<Item = UserMessageContent>,
        name: Option<String>,
        language: Option<String>,
        skip_llm: bool,
    ) -> Message {
        Message::User(UserMessage {
            contents: contents.into_iter().collect(),
            name,
            language,
            skip_llm,
        })
    }

    pub fn new_function_calls(calls: impl IntoIterator<Item = MessageToolCalling>) -> Message {
        Message::FunctionCalls(FunctionCallsMessage(calls.into_iter().collect()))
    }

    pub fn new_function_response(id: impl Into<String>, name: impl Into<String>, result: impl Into<Value>) -> Message {
        Message::FunctionResponse(FunctionResponseMessage {
            id: id.into(),
            name: name.into(),
            result: result.into(),
        })
    }

    pub fn new_assistant(
        text: impl Into<String>,
        is_sensitive: bool,
        language: Option<String>,
        skip_llm: bool,
    ) -> Message {
        Message::Assistant(AssistantMessage {
            text: text.into(),
            is_sensitive,
            language,
            skip_llm,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct UserMessage {
    pub contents: Vec<UserMessageContent>,
    pub name: Option<String>,
    pub language: Option<String>,
    pub skip_llm: bool,
}

impl From<UserMessage> for Message {
    fn from(value: UserMessage) -> Message {
        Message::User(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UserMessageContent {
    Text(String),
    ImageUrl(Url),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct SystemMessage(pub String);

impl From<SystemMessage> for Message {
    fn from(value: SystemMessage) -> Message {
        Message::System(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct FunctionCallsMessage(pub Vec<MessageToolCalling>);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct MessageToolCalling {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

impl From<FunctionCallsMessage> for Message {
    fn from(value: FunctionCallsMessage) -> Message {
        Message::FunctionCalls(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct FunctionResponseMessage {
    pub id: String,
    pub name: String,
    pub result: Value,
}

impl From<FunctionResponseMessage> for Message {
    fn from(value: FunctionResponseMessage) -> Message {
        Message::FunctionResponse(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub text: String,
    pub is_sensitive: bool,
    pub language: Option<String>,
    pub skip_llm: bool,
}

impl From<AssistantMessage> for Message {
    fn from(value: AssistantMessage) -> Message {
        Message::Assistant(value)
    }
}
