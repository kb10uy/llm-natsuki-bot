use time::{Date, Duration, PrimitiveDateTime, Time};

/// スケジュール決定のベースになる論理日付情報。
#[derive(Debug, Clone)]
pub struct LogicalDateTime {
    /// ローカル日時。
    pub local_now: PrimitiveDateTime,

    /// 論理日付のユリウス通日。
    pub logical_julian_day: usize,

    /// 論理日付。
    pub logical_date: Date,

    /// 時刻。
    pub time: Time,

    /// 論理日の経過時間。
    pub day_elapsed: Duration,

    /// 論理日の進行度。範囲は `0.0..1.0`。
    pub day_progress: f64,

    /// 長周期カウント。
    pub long_term_cycles: usize,

    /// 長周期内での経過日数。
    pub long_term_days: usize,
}

impl LogicalDateTime {
    /// ローカル日時から論理日付や長周期情報を算出する。
    pub fn calculate(local_now: PrimitiveDateTime, day_start: Time, long_span: usize) -> LogicalDateTime {
        let logical_date = if local_now.time() < day_start {
            local_now.date().previous_day().expect("minimum date")
        } else {
            local_now.date()
        };

        let day_elapsed = local_now - PrimitiveDateTime::new(logical_date, day_start);
        let day_progress = day_elapsed.as_seconds_f64() / Duration::DAY.as_seconds_f64();

        let logical_julian_day = logical_date.to_julian_day() as usize;
        let long_term_cycles = logical_julian_day.div_euclid(long_span);
        let long_term_days = logical_julian_day.rem_euclid(long_span);

        LogicalDateTime {
            local_now,
            logical_julian_day,
            logical_date,
            time: local_now.time(),
            day_elapsed,
            day_progress,
            long_term_cycles,
            long_term_days,
        }
    }
}
