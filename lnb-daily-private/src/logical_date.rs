use time::{Date, Duration, PrimitiveDateTime, Time};

#[derive(Debug, Clone)]
pub struct LogicalDateTime {
    pub local_now: PrimitiveDateTime,
    pub logical_julian_day: usize,
    pub logical_date: Date,
    pub time: Time,
    pub day_progress: f64,
    pub long_term_cycles: usize,
    pub long_term_days: usize,
}

impl LogicalDateTime {
    pub fn calculate(local_now: PrimitiveDateTime, day_start: Time, long_span: usize) -> LogicalDateTime {
        let logical_date = if local_now.time() < day_start {
            local_now.date().previous_day().expect("minimum date")
        } else {
            local_now.date()
        };

        let day_progress = {
            let logical_day_start = PrimitiveDateTime::new(logical_date, day_start);
            let progress_duration = local_now - logical_day_start;
            progress_duration.as_seconds_f64() / Duration::DAY.as_seconds_f64()
        };

        let logical_julian_day = logical_date.to_julian_day() as usize;
        let long_term_cycles = logical_julian_day.div_euclid(long_span);
        let long_term_days = logical_julian_day.rem_euclid(long_span);

        LogicalDateTime {
            local_now,
            logical_julian_day,
            logical_date,
            time: local_now.time(),
            day_progress,
            long_term_cycles,
            long_term_days,
        }
    }
}
