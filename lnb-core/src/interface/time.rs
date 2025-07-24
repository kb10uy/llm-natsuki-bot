use std::fmt::Debug;

use time::OffsetDateTime;

/// 日時情報の抽象化。
pub trait DateTimeProvider: Debug + Send + Sync {
    fn now(&self) -> OffsetDateTime;
}
