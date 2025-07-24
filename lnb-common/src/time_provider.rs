use lnb_core::interface::time::DateTimeProvider;
use time::{Duration, OffsetDateTime};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BotDateTimeProvider {
    offset: Duration,
}

impl BotDateTimeProvider {
    pub fn new() -> BotDateTimeProvider {
        BotDateTimeProvider { offset: Duration::ZERO }
    }

    pub fn set_offset(&mut self, offset: Duration) {
        self.offset = offset;
    }
}

impl Default for BotDateTimeProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl DateTimeProvider for BotDateTimeProvider {
    fn now(&self) -> OffsetDateTime {
        let raw_now = OffsetDateTime::now_local().expect("offset must be determinate");
        raw_now + self.offset
    }
}
