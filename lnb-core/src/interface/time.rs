use time::OffsetDateTime;

/// 日時情報の抽象化。
pub trait DateTimeProvider {
    fn now(&self) -> OffsetDateTime;
}
