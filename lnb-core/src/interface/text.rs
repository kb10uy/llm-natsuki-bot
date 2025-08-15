use std::fmt::Debug;

/// 可変テキストの抽象化。
pub trait TextProvider: Debug + Send + Sync {
    /// 生成時に渡す追加データ。
    type Data: Send + Sync;

    fn generate(&self, data: Self::Data) -> String;
}
