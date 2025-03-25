pub mod cli;
pub mod error;
pub mod mastodon;

use crate::platform::error::Error;

use futures::future::BoxFuture;

pub trait ConversationPlatform {
    /// このプラットフォームに対して処理を開始する。
    /// 基本的には返される Future は半永久的に処理が続くが、`execute()` 自身は複数回呼ばれる可能性を考慮しなければならない。
    fn execute(&self) -> BoxFuture<'static, Result<(), Error>>;
}
