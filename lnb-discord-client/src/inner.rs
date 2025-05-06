use std::sync::Arc;

use crate::text::{sanitize_discord_message, sanitize_markdown_for_discord};

use lnb_common::{config::client::ConfigClientDiscord, user_roles::UserRolesGroup};
use lnb_core::{
    error::ClientError,
    interface::{Context as LnbContext, server::LnbServer},
    model::{
        conversation::ConversationUpdate,
        message::{AssistantMessage, UserMessage, UserMessageContent},
    },
};
use tokio::{spawn, sync::RwLock};
use tracing::{info, warn};
use twilight_cache_inmemory::{DefaultInMemoryCache, ResourceType};
use twilight_gateway::{Event, EventTypeFlags, Intents, Shard, ShardId, StreamExt};
use twilight_http::Client;
use twilight_model::{
    gateway::payload::incoming::{MessageCreate, Ready},
    user::CurrentUser,
};

const CONTEXT_KEY_PREFIX: &str = "discord";

#[derive(Debug)]
pub struct DiscordLnbClientInner<S> {
    client: Client,
    roles_group: UserRolesGroup,
    bot_user: RwLock<Option<CurrentUser>>,
    max_length: usize,
    assistant: S,
}

impl<S: LnbServer> DiscordLnbClientInner<S> {
    pub async fn new(
        config: &ConfigClientDiscord,
        roles_group: UserRolesGroup,
        assistant: S,
    ) -> Result<DiscordLnbClientInner<S>, ClientError> {
        let client = Client::new(config.token.clone());
        let inner = DiscordLnbClientInner {
            client,
            roles_group,
            bot_user: RwLock::new(None),
            max_length: config.max_length,
            assistant,
        };
        Ok(inner)
    }

    pub async fn execute(self: Arc<Self>) -> Result<(), ClientError> {
        let mut shard = Shard::new(
            ShardId::ONE,
            self.client.token().expect("should be set").to_string(),
            Intents::GUILDS | Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT,
        );
        let cache = DefaultInMemoryCache::builder()
            .resource_types(ResourceType::MESSAGE)
            .build();

        while let Some(item) = shard.next_event(EventTypeFlags::all()).await {
            match item {
                Ok(event) => {
                    cache.update(&event);
                    let cloned_self = self.clone();
                    spawn(cloned_self.handle_event(event));
                }
                Err(err) => {
                    warn!("message error: {err}");
                }
            }
        }

        Ok(())
    }

    async fn handle_event(self: Arc<Self>, event: Event) -> Result<(), ClientError> {
        match event {
            Event::Ready(ready) => self.on_ready(*ready).await?,
            Event::MessageCreate(message_create) => self.on_message_create(*message_create).await?,
            _ => (),
        }
        Ok(())
    }

    async fn on_ready(&self, ready: Ready) -> Result<(), ClientError> {
        info!("Discord client got ready: [{}] {}", ready.user.id, ready.user.name);

        let mut bot_user = self.bot_user.write().await;
        *bot_user = Some(ready.user);
        Ok(())
    }

    async fn on_message_create(&self, message_create: MessageCreate) -> Result<(), ClientError> {
        let bot_user = self.bot_user.read().await;
        let Some(bot_user) = bot_user.as_ref() else {
            return Ok(());
        };

        // (自分含む) bot のメッセージと非メンションを除外
        if message_create.author.bot || message_create.mentions.iter().all(|m| m.id != bot_user.id) {
            return Ok(());
        }

        self.on_mentioned_message(message_create).await?;
        Ok(())
    }

    async fn on_mentioned_message(&self, message: MessageCreate) -> Result<(), ClientError> {
        // Conversation の検索
        let context_key = message.referenced_message.as_ref().map(|rm| rm.id.to_string());
        let conversation_id = match context_key {
            Some(context) => {
                info!("restoring conversation with last referenced message ID {context}");
                let context_key = format!("{CONTEXT_KEY_PREFIX}:{context}");
                match self.assistant.restore_conversation(&context_key).await? {
                    Some(c) => c,
                    None => {
                        info!("conversation has been lost, creating new one");
                        self.assistant.new_conversation().await?
                    }
                }
            }
            None => {
                info!("creating new conversation");
                self.assistant.new_conversation().await?
            }
        };

        // TODO: attachments
        let sanitized_content = sanitize_discord_message(&message.content);
        info!("[{}] {}: {}", message.id, message.author.id, sanitized_content);

        let contents = vec![UserMessageContent::Text(sanitized_content)];
        // contents.extend(images);

        // Conversation の更新・呼出し
        let new_messages = vec![
            UserMessage {
                contents,
                language: message.author.locale.clone(),
                ..Default::default()
            }
            .into(),
        ];
        let context = self.create_context(&message).await?;
        let conversation_update = self
            .assistant
            .process_conversation(context, conversation_id, new_messages.clone())
            .await;

        let recovered_update = match conversation_update {
            Ok(update) => update,
            Err(e) => {
                warn!("reporting conversation error: {e}",);
                ConversationUpdate::create_ephemeral(
                    conversation_id,
                    new_messages,
                    AssistantMessage {
                        text: e.to_string(),
                        skip_llm: true,
                        ..Default::default()
                    },
                )
            }
        };
        let assistant_message = recovered_update.assistant_response();
        let attachments = recovered_update.attachments();
        info!(
            "夏稀[{}]: {:?} ({} attachment(s))",
            assistant_message.is_sensitive,
            assistant_message.text,
            attachments.len()
        );
        // TODO: attachments

        // リプライ
        let mut sanitized_text = sanitize_markdown_for_discord(&assistant_message.text);
        if sanitized_text.chars().count() > self.max_length {
            sanitized_text = sanitized_text.chars().take(self.max_length).collect();
            sanitized_text.push_str("...(omitted)");
        }

        let replied_message = {
            let response = self
                .client
                .create_message(message.channel_id)
                .reply(message.id)
                .content(&sanitized_text)
                .await
                .map_err(ClientError::by_communication)?;
            response.model().await.map_err(ClientError::by_communication)?
        };

        // Conversation/history の更新
        let new_history_id = format!("{CONTEXT_KEY_PREFIX}:{}", replied_message.id);
        self.assistant
            .save_conversation(recovered_update, &new_history_id)
            .await?;

        Ok(())
    }

    async fn create_context(&self, message: &MessageCreate) -> Result<LnbContext, ClientError> {
        let identity = format!("{CONTEXT_KEY_PREFIX}:{}", message.author.id);

        let context = LnbContext::new_user(identity, self.roles_group.get(&message.author.id.to_string()).clone());
        Ok(context)
    }
}
