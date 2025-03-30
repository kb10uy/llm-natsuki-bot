use crate::model::message::{AssistantMessage, Message, UserMessage};

use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConversationId(pub Uuid);

impl ConversationId {
    pub fn new_now() -> ConversationId {
        ConversationId(Uuid::now_v7())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    id: ConversationId,
    messages: Vec<Message>,
}

impl Conversation {
    pub fn new_now(system: Option<Message>) -> Conversation {
        Conversation {
            id: ConversationId::new_now(),
            messages: system.into_iter().collect(),
        }
    }

    pub fn id(&self) -> ConversationId {
        self.id
    }

    pub fn push_messages(mut self, pushed_messages: impl IntoIterator<Item = Message>) -> Conversation {
        self.messages.extend(pushed_messages);
        self
    }
}

#[derive(Debug, Clone)]
pub struct IncompleteConversation {
    base: Conversation,
    pushed_messages: Vec<Message>,
}

impl IncompleteConversation {
    pub fn start(conversation: Conversation, user_message: UserMessage) -> IncompleteConversation {
        let pushed_messages = vec![user_message.into()];
        IncompleteConversation {
            base: conversation,
            pushed_messages,
        }
    }

    pub fn messages_with_pushed(&self) -> impl Iterator<Item = &Message> {
        self.base.messages.iter().chain(self.pushed_messages.iter())
    }

    pub fn extend_message(&mut self, messages: impl IntoIterator<Item = Message>) {
        self.pushed_messages.extend(messages);
    }

    pub fn finish(
        self,
        assistant_response: AssistantMessage,
        assistant_attachments: Vec<ConversationAttachment>,
    ) -> ConversationUpdate {
        ConversationUpdate {
            base_conversation_id: self.base.id,
            intermediate_messages: self.pushed_messages,
            assistant_response,
            assistant_attachments,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConversationUpdate {
    base_conversation_id: ConversationId,
    intermediate_messages: Vec<Message>,
    assistant_response: AssistantMessage,
    assistant_attachments: Vec<ConversationAttachment>,
}

impl ConversationUpdate {
    pub fn id(&self) -> ConversationId {
        self.base_conversation_id
    }

    pub fn assistant_response(&self) -> &AssistantMessage {
        &self.assistant_response
    }

    pub fn attachments(&self) -> &[ConversationAttachment] {
        &self.assistant_attachments
    }

    pub fn into_completing_messages(self) -> Vec<Message> {
        let mut messages = self.intermediate_messages;
        messages.push(self.assistant_response.into());
        messages
    }
}

#[derive(Debug, Clone)]
pub enum ConversationAttachment {
    Image { url: Url, description: Option<String> },
}
