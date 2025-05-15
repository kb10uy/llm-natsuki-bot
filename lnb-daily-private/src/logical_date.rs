use time::{Date, Duration, PrimitiveDateTime, Time, Weekday};

#[derive(Debug, Clone)]
pub struct LogicalDateTime {
    original: PrimitiveDateTime,
    logical_date: Date,
    logical_julian_day: usize,
    day_progress: f64,
    long_term_cycles: usize,
    long_term_days: usize,
}

impl LogicalDateTime {
    pub fn calculate(original: PrimitiveDateTime, day_start: Time, long_span: usize) -> LogicalDateTime {
        let logical_date = if original.time() < day_start {
            original.date().previous_day().expect("minimum date")
        } else {
            original.date()
        };

        let day_progress = {
            let logical_day_start = PrimitiveDateTime::new(logical_date, day_start);
            let progress_duration = original - logical_day_start;
            progress_duration.as_seconds_f64() / Duration::DAY.as_seconds_f64()
        };

        let logical_julian_day = logical_date.to_julian_day() as usize;
        let long_term_cycles = logical_julian_day.div_euclid(long_span);
        let long_term_days = logical_julian_day.rem_euclid(long_span);

        LogicalDateTime {
            original,
            logical_date,
            logical_julian_day,
            day_progress,
            long_term_cycles,
            long_term_days,
        }
    }

    pub fn local_now(&self) -> PrimitiveDateTime {
        self.original
    }

    pub fn logical_date(&self) -> Date {
        self.logical_date
    }

    pub fn time(&self) -> Time {
        self.original.time()
    }

    pub fn logical_julian_day(&self) -> usize {
        self.logical_julian_day
    }

    pub fn day_progress(&self) -> f64 {
        self.day_progress
    }

    pub fn long_term_cycles(&self) -> usize {
        self.long_term_cycles
    }

    pub fn long_term_days(&self) -> usize {
        self.long_term_days
    }

    pub fn weekday(&self) -> Weekday {
        self.logical_date.weekday()
    }
}
