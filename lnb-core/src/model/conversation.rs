use std::collections::BTreeSet;

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
    attachments: Vec<ConversationAttachment>,
}

impl IncompleteConversation {
    pub fn start(conversation: Conversation, user_message: UserMessage) -> IncompleteConversation {
        let pushed_messages = vec![user_message.into()];
        IncompleteConversation {
            base: conversation,
            pushed_messages,
            attachments: vec![],
        }
    }

    pub fn llm_sending_messages(&self) -> impl Iterator<Item = &Message> {
        self.base
            .messages
            .iter()
            .chain(self.pushed_messages.iter())
            .filter(|m| match m {
                Message::User(um) => !um.skip_llm,
                Message::Assistant(am) => !am.skip_llm,
                _ => true,
            })
    }

    pub fn extend_messages(&mut self, messages: impl IntoIterator<Item = Message>) {
        self.pushed_messages.extend(messages);
    }

    pub fn extend_attachments(&mut self, attachments: impl IntoIterator<Item = ConversationAttachment>) {
        self.attachments.extend(attachments);
    }

    /// 元の `Conversation` のうち最後にある `UserMessage` を取得する。
    pub fn last_user(&self) -> Option<&UserMessage> {
        let Some(Message::User(last_user)) = &self.pushed_messages.last() else {
            return None;
        };
        Some(last_user)
    }

    /// 最後の `AssistantMessage` に指定された `AssistantMessage` の内容を追加する。
    /// 最後が `AssistantMessage` でなければ受け取ったものをそのまま追加する。
    pub fn push_assistant(&mut self, appending_message: AssistantMessage) {
        let Some(Message::Assistant(last_assistant)) = self.pushed_messages.last_mut() else {
            self.pushed_messages.push(appending_message.into());
            return;
        };
        last_assistant.text.push_str(&appending_message.text);
        last_assistant.is_sensitive |= appending_message.is_sensitive;
        last_assistant.skip_llm &= appending_message.skip_llm;
        if let Some(updated_language) = appending_message.language {
            last_assistant.language = Some(updated_language);
        }
    }

    /// 最後の `AssistantMessage` に指定された `AssistantMessage` の内容を追加してそれを合計の `AssistantMessage` とする。
    /// 最後が `AssistantMessage` でなければ受け取ったものをそのまま適用する。
    pub fn finish(mut self, finished_response: AssistantMessage) -> ConversationUpdate {
        let assistant_response = match self.pushed_messages.pop() {
            // Cut ありで完了
            Some(Message::Assistant(mut last_assistant)) => {
                last_assistant.text.push_str(&finished_response.text);
                last_assistant.is_sensitive |= finished_response.is_sensitive;
                last_assistant.skip_llm &= finished_response.skip_llm;
                if let Some(updated_language) = finished_response.language {
                    last_assistant.language = Some(updated_language);
                }
                last_assistant
            }

            // Cut なしで完了
            Some(otherwise) => {
                self.pushed_messages.push(otherwise);
                finished_response
            }

            // 要素なし(普通 `User` が入ると思うけど)
            None => finished_response,
        };

        ConversationUpdate {
            base_conversation_id: self.base.id,
            intermediate_messages: self.pushed_messages,
            assistant_response,
            attachments: self.attachments,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConversationUpdate {
    base_conversation_id: ConversationId,
    intermediate_messages: Vec<Message>,
    assistant_response: AssistantMessage,
    attachments: Vec<ConversationAttachment>,
}

impl ConversationUpdate {
    pub fn id(&self) -> ConversationId {
        self.base_conversation_id
    }

    pub fn assistant_response(&self) -> &AssistantMessage {
        &self.assistant_response
    }

    pub fn attachments(&self) -> &[ConversationAttachment] {
        &self.attachments
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserRole {
    Privileged,
    Scoped(BTreeSet<String>),
    Normal,
}

impl UserRole {
    pub fn scoped_with(scopes: impl IntoIterator<Item = impl Into<String>>) -> UserRole {
        UserRole::Scoped(scopes.into_iter().map(|s| s.into()).collect())
    }
}
