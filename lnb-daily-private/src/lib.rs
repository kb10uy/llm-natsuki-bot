mod self_pleasure;
mod underwear;

use serde::{Deserialize, Serialize};
use time::{Date, Duration, PrimitiveDateTime, Time};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Configuration {
    /// 昼パートの開始時刻。
    pub daytime_start_at: Time,

    /// 朝着替えの時間。
    pub morning_preparation: Duration,

    /// 夜パートの開始時刻。
    pub night_start_at: Time,

    /// 風呂に入ってる時間。
    pub bathtime_duration: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum DayStep {
    /// 起きた後着替えるまで(昼パート)
    Morning,

    /// 日中(昼パート)
    Daytime,

    /// 風呂(夜パート)
    Bathtime,

    /// 寝支度完了(夜パート)
    Night,

    /// 寝てる(夜パート)
    Asleep,

    /// 中途覚醒(夜パート)
    MidAwake,
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

    /// 論理今日を取得する。
    pub fn logical_date(&self, datetime: PrimitiveDateTime) -> Date {
        let date = datetime.date();
        if datetime.time() < self.daytime_start_at {
            date.previous_day().expect("minimum date")
        } else {
            date
        }
    }

    /// 論理今日が進んだ割合を計算する。
    pub fn logical_day_progress(&self, datetime: PrimitiveDateTime) -> f64 {
        let logical_day_start = datetime.replace_date(self.logical_date(datetime));
        let progress_duration = datetime - logical_day_start;
        progress_duration.as_seconds_f64() / Duration::DAY.as_seconds_f64()
    }

    /// 論理日付の昼パートの開始日時を計算する。
    pub fn day_part_start(&self, logical_date: Date) -> PrimitiveDateTime {
        PrimitiveDateTime::new(logical_date, self.daytime_start_at)
    }

    /// 論理日付の夜パートの開始日時を計算する。
    pub fn night_part_start(&self, logical_date: Date) -> PrimitiveDateTime {
        if self.daytime_over_midnight() {
            PrimitiveDateTime::new(logical_date.next_day().expect("maximum date"), self.night_start_at)
        } else {
            PrimitiveDateTime::new(logical_date, self.night_start_at)
        }
    }

    /// `DayStep` を論理日時ベースで計算する。
    pub fn determine_day_step(&self, datetime: PrimitiveDateTime) -> DayStep {
        let logical_date = self.logical_date(datetime);
        let is_daytime = if self.daytime_over_midnight() {
            let night_range = self.night_start_at..self.daytime_start_at;
            !night_range.contains(&datetime.time())
        } else {
            let daytime_range = self.daytime_start_at..self.night_start_at;
            daytime_range.contains(&datetime.time())
        };
        let part_elapsed = if is_daytime {
            datetime - self.day_part_start(logical_date)
        } else {
            datetime - self.night_part_start(logical_date)
        };

        match is_daytime {
            true if part_elapsed < self.morning_preparation => DayStep::Morning,
            true => DayStep::Daytime,
            false if part_elapsed < self.bathtime_duration => DayStep::Bathtime,
            false => DayStep::Night,
        }
    }
}
