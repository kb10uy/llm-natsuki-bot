mod self_pleasure;
mod underwear;

use time::{Date, OffsetDateTime, Time};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Configuration {
    pub night_start_at: Time,
    pub daytime_start_at: Time,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DayStep {
    Daytime,
    Night,
}

impl Configuration {
    /// `Daytime` が日付を回るかどうか。
    /// ```text
    /// true:  |ddNnnnnnDddddddddddddddd|
    /// false: |nnnnnnnnDdddddddddddNnnn|
    /// ```
    fn daytime_over_midnight(&self) -> bool {
        self.night_start_at < self.daytime_start_at
    }

    /// 「論理今日」を取得する。
    pub fn logical_date(&self, datetime: OffsetDateTime) -> Date {
        let date = datetime.date();
        if datetime.time() < self.daytime_start_at {
            date.previous_day().expect("minimum date")
        } else {
            date
        }
    }

    pub fn determine_day_step(&self, datetime: OffsetDateTime) -> DayStep {
        if self.daytime_over_midnight() {
            let night_range = self.night_start_at..self.daytime_start_at;
            if night_range.contains(&datetime.time()) {
                DayStep::Night
            } else {
                DayStep::Daytime
            }
        } else {
            let daytime_range = self.daytime_start_at..self.night_start_at;
            if daytime_range.contains(&datetime.time()) {
                DayStep::Daytime
            } else {
                DayStep::Night
            }
        }
    }
}
