use crate::{
    CONTEXT_KEY_PREFIX,
    text::{sanitize_markdown_for_mastodon, sanitize_mention_html_from_mastodon},
};

use std::{collections::HashMap, iter::once, sync::Arc, time::Duration};

use futures::prelude::*;
use lnb_common::config::client::ConfigClientMastodon;
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
    entities::{
        AttachmentId, StatusId, account::Account, event::Event, notification::Type as NotificationType, status::Status,
    },
    prelude::MediaType,
};
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use thiserror::Error as ThisError;
use tokio::{fs::File, io::AsyncWriteExt, spawn, time::sleep};
use tracing::{debug, error, info, warn};

const RECONNECT_SLEEP: Duration = Duration::from_secs(120);

#[derive(Debug)]
pub struct MastodonLnbClientInner<S> {
    assistant: S,
    mastodon: Mastodon,
    self_account: Account,
    sensitive_spoiler: String,
    max_length: usize,
    remote_fetch_delay: Duration,
}

impl<S: LnbServer> MastodonLnbClientInner<S> {
    pub async fn new(
        config: &ConfigClientMastodon,
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
            mastodon,
            self_account,
            sensitive_spoiler: config.sensitive_spoiler.clone(),
            max_length: config.max_length,
            remote_fetch_delay: Duration::from_secs(config.remote_fetch_delay_seconds as u64),
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
        let Event::Notification(notification) = event else {
            return;
        };

        let processed = match notification.notification_type {
            NotificationType::Mention => match notification.status {
                Some(status) => self.process_status(status).await,
                None => Err(ClientError::External(MastodonClientError::InvalidMention.into())),
            },
            NotificationType::Follow => {
                info!("followed by {}", notification.account.acct);
                Ok(())
            }
            _ => Ok(()),
        };

        let Err(err) = processed else {
            return;
        };
        error!("mastodon event process reported error: {err}");
    }

    /// 投稿イベントを処理する。
    async fn process_status(&self, status: Status) -> Result<(), ClientError> {
        // フィルタリング(bot flag と自分には応答しない)
        if status.account.bot || status.account.id == self.self_account.id {
            return Ok(());
        }

        // Conversation の検索
        let (conversation_id, new_messages) = self.get_conversation(&status).await?;

        // Conversation の更新・呼出し
        let context = self.create_context(&status).await?;
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
        let new_history_id = format!("{CONTEXT_KEY_PREFIX}:{}", replied_status.id);
        self.assistant
            .save_conversation(recovered_update, &new_history_id)
            .await?;
        debug!("saved conversation {}", replied_status.id);

        Ok(())
    }

    /// `ConversationId` と新規追加されるべき `Message` のリストを取得する。
    /// * in_reply_to があり、会話を復元できる場合はその ID を返す。
    /// * in_reply_to がない場合は新しい `Conversation` の ID を返す。
    /// * in_reply_to があるが復元できない場合、新しい `Conversation` の ID と Mastodon 側の会話ツリーを返す。
    async fn get_conversation(&self, mentioned_status: &Status) -> Result<(ConversationId, Vec<Message>), ClientError> {
        match mentioned_status.in_reply_to_id.clone() {
            // リプライであることが確定している
            Some(in_reply_to_id) => {
                let context_key = format!("{CONTEXT_KEY_PREFIX}:{in_reply_to_id}");

                // 既知の会話
                if let Some(id) = self.assistant.restore_conversation(&context_key).await? {
                    debug!("restored conversation from id {in_reply_to_id}");
                    let user_message = self.transform_status(mentioned_status);
                    return Ok((id, vec![user_message]));
                }

                // 未知の会話
                debug!("unknown in_reply_to_id, creating new one: {in_reply_to_id}");
                let id = self.assistant.new_conversation().await?;
                let ancestors = self.get_ancestor(&mentioned_status.id).await;
                let current = self.transform_status(mentioned_status);
                Ok((id, ancestors.into_iter().chain(once(current)).collect()))
            }

            // リプライがあるかもしれない(到着直後では埋まっていないことがあるため)
            None => {
                let has_unknown_mention = mentioned_status
                    .mentions
                    .iter()
                    .any(|m| m.acct != self.self_account.acct);

                if has_unknown_mention {
                    // 他人に言及されているので待つ
                    debug!("waiting for fetching unknown ancestors of {}", mentioned_status.id);

                    // 意思表示としてふぁぼる
                    self.mastodon
                        .favourite(&mentioned_status.id)
                        .map_err(ClientError::by_external)
                        .await?;

                    sleep(self.remote_fetch_delay).await;

                    let id = self.assistant.new_conversation().await?;
                    let ancestors = self.get_ancestor(&mentioned_status.id).await;
                    let current = self.transform_status(mentioned_status);
                    Ok((id, ancestors.into_iter().chain(once(current)).collect()))
                } else {
                    // もはや新規会話とみなすしかない
                    debug!("creating new conversation");
                    let id = self.assistant.new_conversation().await?;
                    let user_message = self.transform_status(mentioned_status);
                    Ok((id, vec![user_message]))
                }
            }
        }
    }

    /// 親会話ツリーを取得して Message にする。
    async fn get_ancestor(&self, base_id: &StatusId) -> Vec<Message> {
        match self.mastodon.get_context(base_id).await {
            Ok(mstdn_context) => {
                debug!("restoring {} statuses from context", mstdn_context.ancestors.len());
                mstdn_context
                    .ancestors
                    .into_iter()
                    .map(|s| self.transform_status(&s))
                    .collect()
            }
            Err(e) => {
                warn!("failed to fetch context: {e}");
                vec![]
            }
        }
    }

    /// Mastodon の `Status` を `Message` 形式に変換する。
    /// 会話復元時などに bot 自身のアカウントの投稿だった場合は `AssistantMessage` になる。
    fn transform_status(&self, status: &Status) -> Message {
        let mut sanitized_mention_text = sanitize_mention_html_from_mastodon(&status.content);
        let language = status.language.and_then(|l| l.to_639_1()).map(|l| l.to_string());

        if status.account.id == self.self_account.id {
            AssistantMessage {
                text: sanitized_mention_text,
                language,
                ..Default::default()
            }
            .into()
        } else {
            info!(
                "[{}] {}: {:?} ({} attachment(s))",
                status.id,
                status.account.acct,
                sanitized_mention_text,
                status.media_attachments.len(),
            );
            let contents = {
                let image_urls: Vec<_> = status
                    .media_attachments
                    .iter()
                    .filter(|a| matches!(a.media_type, MediaType::Image | MediaType::Gifv))
                    .map(|atch| atch.preview_url.clone())
                    .collect();
                let images: Vec<_> = image_urls
                    .iter()
                    .map(|url| UserMessageContent::ImageUrl(url.clone()))
                    .collect();

                if !image_urls.is_empty() {
                    sanitized_mention_text.push_str("\n--------\n");
                    sanitized_mention_text.push_str("Original Media Images:\n");
                    for image_url in &image_urls {
                        sanitized_mention_text.push_str(image_url.as_str());
                        sanitized_mention_text.push('\n');
                    }
                }
                once(UserMessageContent::Text(sanitized_mention_text))
                    .chain(images)
                    .collect()
            };

            UserMessage {
                contents,
                language,
                ..Default::default()
            }
            .into()
        }
    }

    /// `assistant_message` に対応する内容のリプライを送信する。
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
                ConversationAttachment::Image { bytes, description } => {
                    let image_id = self.upload_image(bytes, description.as_deref()).await?;
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

    /// 画像をアップロードする(非同期アップロード)。
    async fn upload_image(&self, image_data: &[u8], description: Option<&str>) -> Result<AttachmentId, ClientError> {
        let mime_type = infer::get(image_data).map(|ft| ft.mime_type());

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
                .write_all(image_data)
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

    async fn create_context(&self, status: &Status) -> Result<Context, ClientError> {
        let identity = format!("{CONTEXT_KEY_PREFIX}:{}", status.account.acct);
        let remindable = RemindableContext {
            context: CONTEXT_KEY_PREFIX.to_string(),
            requester: serde_json::to_string(&RemindRequester {
                acct: status.account.acct.clone(),
                visibility: status.visibility,
            })
            .map_err(ClientError::by_external)?,
        };

        let mut context = Context::new_user(identity, UserRole::Normal);
        context.set(remindable).map_err(ClientError::by_external)?;
        Ok(context)
    }
}

/// `Remindable` に付与する requester。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RemindRequester {
    acct: String,
    visibility: Visibility,
}

/// bot から投げるメンション投稿の形式。
#[derive(Debug)]
enum ReplyType {
    /// 投稿に対するリプライ。
    Status(Status),

    /// リマインド(親投稿なし)。
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
