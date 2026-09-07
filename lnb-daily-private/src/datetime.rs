use crate::rng::RngGranularity;

use time::{Date, Duration, PrimitiveDateTime, Time};

/// 長周期を特定する情報。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LongTermCycle {
    /// 長周期カウント。
    pub index: usize,

    /// 長周期の日数。
    pub span_days: usize,
}

/// 論理日を特定する情報。日より細かい情報を含まない。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LogicalDay {
    /// 属する長周期。
    pub long_term: LongTermCycle,

    /// 長周期内での経過日数。
    pub long_term_days: usize,

    /// 論理日付のユリウス通日。
    pub julian_day: usize,

    /// 論理日付。
    pub date: Date,
}

/// 論理日時。日より細かい情報を含む。
#[derive(Debug, Clone)]
pub struct LogicalMoment {
    /// 属する論理日。
    pub day: LogicalDay,

    /// ローカル日時。
    pub local_now: PrimitiveDateTime,

    /// 論理日の経過時間。
    pub day_elapsed: Duration,

    /// 論理日の進行度。範囲は `0.0..1.0`。
    pub day_progress: f64,
}

impl LogicalMoment {
    /// ローカル日時から論理日付や長周期情報を算出する。
    pub fn calculate(local_now: PrimitiveDateTime, day_start: Time, long_span: usize) -> LogicalMoment {
        let logical_date = if local_now.time() < day_start {
            local_now.date().previous_day().expect("minimum date")
        } else {
            local_now.date()
        };

        let day_elapsed = local_now - PrimitiveDateTime::new(logical_date, day_start);
        let day_progress = day_elapsed.as_seconds_f64() / Duration::DAY.as_seconds_f64();

        let julian_day = logical_date.to_julian_day() as usize;
        let day = LogicalDay {
            long_term: LongTermCycle {
                index: julian_day.div_euclid(long_span),
                span_days: long_span,
            },
            long_term_days: julian_day.rem_euclid(long_span),
            julian_day,
            date: logical_date,
        };

        LogicalMoment {
            day,
            local_now,
            day_elapsed,
            day_progress,
        }
    }
}

impl RngGranularity for LongTermCycle {
    const NAME: &'static str = "long_term_cycle";

    fn seed_source(&self) -> impl AsRef<[u8]> {
        self.index.to_le_bytes()
    }
}

impl RngGranularity for LogicalDay {
    const NAME: &'static str = "logical_day";

    fn seed_source(&self) -> impl AsRef<[u8]> {
        self.julian_day.to_le_bytes()
    }
}
