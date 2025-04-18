use crate::{
    CONTEXT_KEY_PREFIX, MastodonLnbClientConfig,
    text::{sanitize_markdown_for_mastodon, sanitize_mention_html_from_mastodon},
};

use std::{collections::HashMap, iter::once, sync::Arc, time::Duration};

use futures::prelude::*;
use lnb_core::{
    APP_USER_AGENT, DebugOptionValue,
    error::ClientError,
    interface::{Context, reminder::RemindableContext, server::LnbServer},
    model::{
        conversation::{ConversationAttachment, ConversationId, ConversationUpdate, UserRole},
        message::{AssistantMessage, Message, UserMessage, UserMessageContent},
    },
};
use mastodon_async::{
    Mastodon, NewStatus, Visibility,
    entities::{AttachmentId, account::Account, event::Event, notification::Type as NotificationType, status::Status},
    prelude::MediaType,
};
use reqwest::{Client, header::HeaderMap};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use thiserror::Error as ThisError;
use tokio::{fs::File, io::AsyncWriteExt, spawn, time::sleep};
use tracing::{debug, error, info, warn};
use url::Url;

const RECONNECT_SLEEP: Duration = Duration::from_secs(120);

#[derive(Debug)]
pub struct MastodonLnbClientInner<S> {
    assistant: S,
    http_client: Client,
    mastodon: Mastodon,
    self_account: Account,
    sensitive_spoiler: String,
    max_length: usize,
}

impl<S: LnbServer> MastodonLnbClientInner<S> {
    pub async fn new(
        config: &MastodonLnbClientConfig,
        debug_options: &HashMap<String, DebugOptionValue>,
        assistant: S,
    ) -> Result<MastodonLnbClientInner<S>, ClientError> {
        // Mastodon クライアントと自己アカウント情報
        let http_client = reqwest::ClientBuilder::new()
            .user_agent(APP_USER_AGENT)
            .default_headers(default_headers(debug_options))
            .build()
            .map_err(ClientError::by_communication)?;
        let mastodon_data = mastodon_async::Data {
            base: config.server_url.clone().into(),
            token: config.token.clone().into(),
            ..Default::default()
        };
        let mastodon = Mastodon::new(http_client.clone(), mastodon_data);
        let self_account = mastodon.verify_credentials().map_err(ClientError::by_external).await?;

        Ok(MastodonLnbClientInner {
            assistant,
            http_client,
            mastodon,
            self_account,
            sensitive_spoiler: config.sensitive_spoiler.clone(),
            max_length: config.max_length,
        })
    }

    pub async fn execute(self: Arc<Self>) -> Result<(), ClientError> {
        loop {
            let notification_stream = self
                .mastodon
                .stream_notifications()
                .map_err(ClientError::by_communication)
                .await?;

            let disconnected_status = notification_stream
                .try_for_each(async |(e, _)| {
                    spawn(self.clone().process_event(e));
                    Ok(())
                })
                .map_err(ClientError::by_communication)
                .await;

            match disconnected_status {
                Ok(()) => {
                    warn!("user stream disconnected unexpectedly successfully");
                }
                Err(e) => {
                    error!("user stream disconnected: {e}");
                }
            }
            warn!("trying to reconnect user stream, waiting...");
            sleep(RECONNECT_SLEEP).await;
        }
    }

    async fn process_event(self: Arc<Self>, event: Event) {
        let processed = match event {
            // Event::Update(status) => self.process_status(status).await,
            Event::Notification(notification) => match notification.notification_type {
                NotificationType::Mention => match notification.status {
                    Some(status) => self.process_status(status).await,
                    None => Err(ClientError::External(MastodonClientError::InvalidMention.into())),
                },
                _ => Ok(()),
            },
            _ => Ok(()),
        };

        let Err(err) = processed else {
            return;
        };
        error!("mastodon event process reported error: {err}");
    }

    async fn process_status(&self, status: Status) -> Result<(), ClientError> {
        // フィルタリング(bot flag と自分には応答しない)
        if status.account.bot || status.account.id == self.self_account.id {
            return Ok(());
        }

        // Conversation の検索
        let (conversation_id, new_messages) = self.get_conversation(&status).await?;

        // Conversation の更新・呼出し
        let context = create_context(&status)?;
        let conversation_update = self
            .assistant
            .process_conversation(context, conversation_id, new_messages.clone(), UserRole::Normal)
            .await;

        // 返信処理
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
        let replied_status = self
            .send_reply(ReplyType::Status(status), assistant_message, attachments)
            .await?;
        info!(
            "夏稀[{}]: {:?} ({} attachment(s))",
            assistant_message.is_sensitive,
            assistant_message.text,
            attachments.len()
        );

        // Conversation/history の更新
        // let updated_conversation = conversation_update.finish();
        let new_history_id = format!("{CONTEXT_KEY_PREFIX}:{}", replied_status.id);
        self.assistant
            .save_conversation(recovered_update, &new_history_id)
            .await?;

        Ok(())
    }

    async fn get_conversation(&self, status: &Status) -> Result<(ConversationId, Vec<Message>), ClientError> {
        let Some(status_id) = status.in_reply_to_id.clone() else {
            info!("creating new conversation");
            let id = self.assistant.new_conversation().await?;
            let user_message = self.transform_status(status);
            return Ok((id, vec![user_message]));
        };

        match self.assistant.restore_conversation(status_id.as_ref()).await? {
            Some(id) => {
                info!("conversation {status_id} restored");
                let user_message = self.transform_status(status);
                Ok((id, vec![user_message]))
            }
            None => {
                info!("unknown conversation detected; fetching in_reply_to status {status_id}");
                let id = self.assistant.new_conversation().await?;
                let new_messages = {
                    let previous = match self.mastodon.get_status(&status_id).await {
                        Ok(previous) => Some(self.transform_status(&previous)),
                        Err(e) => {
                            warn!("failed to fetch previous status: {e}");
                            None
                        }
                    };
                    let current = self.transform_status(status);
                    previous.into_iter().chain(once(current)).collect()
                };
                Ok((id, new_messages))
            }
        }
    }

    fn transform_status(&self, status: &Status) -> Message {
        let sanitized_mention_text = sanitize_mention_html_from_mastodon(&status.content);
        let language = status.language.and_then(|l| l.to_639_1()).map(|l| l.to_string());

        if status.account.id == self.self_account.id {
            AssistantMessage {
                text: sanitized_mention_text,
                language,
                ..Default::default()
            }
            .into()
        } else {
            let contents = {
                let images: Vec<_> = status
                    .media_attachments
                    .iter()
                    .filter(|a| matches!(a.media_type, MediaType::Image | MediaType::Gifv))
                    .map(|atch| UserMessageContent::ImageUrl(atch.preview_url.clone()))
                    .collect();
                info!(
                    "[{}] {}: {:?} ({} image(s))",
                    status.id,
                    status.account.acct,
                    sanitized_mention_text,
                    images.len()
                );
                let mut contents = vec![UserMessageContent::Text(sanitized_mention_text)];
                contents.extend(images);
                contents
            };
            UserMessage {
                contents,
                language,
                ..Default::default()
            }
            .into()
        }
    }

    async fn send_reply(
        &self,
        reply_type: ReplyType,
        assistant_message: &AssistantMessage,
        attachments: &[ConversationAttachment],
    ) -> Result<Status, ClientError> {
        // 添付メディア
        let mut attachment_ids = vec![];
        for attachment in attachments {
            match attachment {
                ConversationAttachment::Image { url, description } => {
                    let image_id = self.upload_image(url, description.as_deref()).await?;
                    attachment_ids.push(image_id);
                }
            }
        }

        // リプライ構築
        // 公開範囲は最大 unlisted でリプライ元に合わせる
        // CW はリプライ元があったらそのまま、ないときは要そぎぎなら付与
        let mut sanitized_text = sanitize_markdown_for_mastodon(&assistant_message.text);
        if sanitized_text.chars().count() > self.max_length {
            sanitized_text = sanitized_text.chars().take(self.max_length).collect();
            sanitized_text.push_str("...(omitted)");
        }
        let reply_text = format!("@{} {sanitized_text}", reply_type.acct());
        let reply_spoiler = reply_type
            .present_spoiler()
            .or(assistant_message
                .is_sensitive
                .then_some(self.sensitive_spoiler.as_str()))
            .map(|s| s.to_string());
        let reply_status = NewStatus {
            status: Some(reply_text),
            visibility: Some(reply_type.visilibity()),
            in_reply_to_id: reply_type.in_reply_to_id(),
            spoiler_text: reply_spoiler,
            media_ids: Some(attachment_ids),
            ..Default::default()
        };

        let replied_status = self
            .mastodon
            .new_status(reply_status)
            .map_err(ClientError::by_external)
            .await?;
        Ok(replied_status)
    }

    async fn upload_image(&self, url: &Url, description: Option<&str>) -> Result<AttachmentId, ClientError> {
        // ダウンロード
        let response = self
            .http_client
            .get(url.to_string())
            .send()
            .map_err(ClientError::by_external)
            .await?;
        let image_data = response.bytes().map_err(ClientError::by_external).await?;
        let mime_type = infer::get(&image_data).map(|ft| ft.mime_type());

        // tempfile に書き出し
        let tempfile = match mime_type {
            Some("image/jpeg") => NamedTempFile::with_suffix(".jpg").map_err(ClientError::by_external)?,
            Some("image/png") => NamedTempFile::with_suffix(".png").map_err(ClientError::by_external)?,
            Some("image/gif") => NamedTempFile::with_suffix(".gif").map_err(ClientError::by_external)?,
            Some(otherwise) => {
                return Err(ClientError::External(
                    MastodonClientError::UnsupportedImageType(otherwise.to_string()).into(),
                ));
            }
            None => {
                return Err(ClientError::External(
                    MastodonClientError::UnsupportedImageType("(unknown)".into()).into(),
                ));
            }
        };
        debug!("writing temporary image at {:?}", tempfile.path());
        // tokio File にするので分解する
        let restored_tempfile = {
            let (temp_file, temp_path) = tempfile.into_parts();
            let mut async_file = File::from_std(temp_file);
            async_file
                .write_all(&image_data)
                .await
                .map_err(ClientError::by_external)?;
            let restored_file = async_file.into_std().await;
            NamedTempFile::from_parts(restored_file, temp_path)
        };
        // アップロード
        let uploaded_attachment = self
            .mastodon
            .media(restored_tempfile.path(), description.map(|d| d.to_string()))
            .map_err(ClientError::by_external)
            .await?;
        // ここまで生き残らせる
        drop(restored_tempfile);

        Ok(uploaded_attachment.id)
    }

    pub async fn remind(&self, requester: String, update: ConversationUpdate) -> Result<(), ClientError> {
        let remind_requester = serde_json::from_str(&requester).map_err(ClientError::by_external)?;
        let assistant_message = update.assistant_response();
        let attachments = update.attachments();
        let replied_status = self
            .send_reply(ReplyType::Remind(remind_requester), assistant_message, attachments)
            .await?;
        info!(
            "夏稀[{}]: {:?} ({} attachment(s))",
            assistant_message.is_sensitive,
            assistant_message.text,
            attachments.len()
        );

        // Conversation/history の更新
        let new_history_id = format!("{CONTEXT_KEY_PREFIX}:{}", replied_status.id);
        self.assistant.save_conversation(update, &new_history_id).await?;

        Ok(())
    }
}

fn create_context(status: &Status) -> Result<Context, ClientError> {
    let mut context = Context::default();
    context.set(RemindableContext {
        context: CONTEXT_KEY_PREFIX.to_string(),
        requester: serde_json::to_string(&RemindRequester {
            acct: status.account.acct.clone(),
            visibility: status.visibility,
        })
        .map_err(ClientError::by_external)?,
    });
    Ok(context)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RemindRequester {
    acct: String,
    visibility: Visibility,
}

#[derive(Debug)]
enum ReplyType {
    Status(Status),
    Remind(RemindRequester),
}

impl ReplyType {
    pub fn in_reply_to_id(&self) -> Option<String> {
        match self {
            ReplyType::Status(status) => Some(status.id.to_string()),
            ReplyType::Remind(_) => None,
        }
    }

    pub fn acct(&self) -> &str {
        match self {
            ReplyType::Status(status) => &status.account.acct,
            ReplyType::Remind(requester) => &requester.acct,
        }
    }

    pub fn visilibity(&self) -> Visibility {
        let original_visibility = match self {
            ReplyType::Status(status) => status.visibility,
            ReplyType::Remind(requester) => requester.visibility,
        };
        match original_visibility {
            Visibility::Public => Visibility::Unlisted,
            otherwise => otherwise,
        }
    }

    pub fn present_spoiler(&self) -> Option<&str> {
        match self {
            ReplyType::Status(status) if !status.spoiler_text[..].trim().is_empty() => Some(&status.spoiler_text),
            _ => None,
        }
    }
}

#[derive(Debug, ThisError)]
pub enum MastodonClientError {
    #[error("invalid mention object")]
    InvalidMention,

    #[error("unsupported image type: {0}")]
    UnsupportedImageType(String),
}

fn default_headers(debug_options: &HashMap<String, DebugOptionValue>) -> HeaderMap {
    let mut headers = HeaderMap::new();

    if let Some(DebugOptionValue::Specified(secs)) = debug_options.get("mastodon_disconnect") {
        warn!("force disconnection enabled; duration is {secs}");
        headers.append("X-Disconnect-After", secs.parse().expect("must parse"));
    }

    headers
}
