use time::{Duration, UtcDateTime};

#[derive(Debug, Clone)]
pub struct Bucket {
    previous_start: UtcDateTime,
    duration: Duration,
    previous_count: usize,
    current_count: usize,
}

impl Bucket {
    pub fn new_from_now(now: UtcDateTime, duration: Duration) -> Bucket {
        let previous_start = now - duration;
        Bucket {
            previous_start,
            duration,
            previous_count: 0,
            current_count: 0,
        }
    }

    pub fn try_increment(&mut self, now: UtcDateTime, limit: usize) -> bool {
        self.ensure_rotated(now);
        let called_in_window = self.weighted_count(now);

        if called_in_window < limit as f64 {
            self.current_count += 1;
            true
        } else {
            false
        }
    }

    fn weighted_count(&self, now: UtcDateTime) -> f64 {
        let current_start = self.previous_start + self.duration;
        let current_covering_duration = now - current_start;
        let current_covering_ratio = current_covering_duration / self.duration;

        (1.0 - current_covering_ratio) * self.previous_count as f64 + self.current_count as f64
    }

    fn ensure_rotated(&mut self, now: UtcDateTime) {
        let current_end = self.previous_start + self.duration * 2;
        if now <= current_end {
            // now が今のウィンドウに入っている
            return;
        }

        let next_end = self.previous_start + self.duration * 3;
        if now > next_end {
            // now が十分離れている
            self.previous_count = 0;
            self.current_count = 0;
            self.previous_start = now - self.duration;
        } else {
            // now が次のウィンドウ内に入っている
            self.previous_count = self.current_count;
            self.current_count = 0;
            self.previous_start += self.duration;
        }
    }
}

#[cfg(test)]
mod tests {
    use time::{Duration, macros::utc_datetime};

    use super::Bucket;

    #[test]
    fn bucket_counts() {
        let now = utc_datetime!(2025-04-01 00:00);
        let duration = Duration::seconds(60);
        let mut bucket = Bucket::new_from_now(now, duration);

        // 上限まで入れられる
        for i in 1..=10 {
            println!("increment {i}(th)");
            assert!(bucket.try_increment(now, 10));
            println!("window count: {}", bucket.weighted_count(now));
        }
        // 入れられない
        assert!(!bucket.try_increment(now, 10));

        // 次ウィンドウの中間地点では 5 回呼べる
        let next_window_now = now + Duration::seconds(90);
        for i in 1..=5 {
            println!("increment {i}(th)");
            assert!(bucket.try_increment(next_window_now, 10));
            println!("window count: {}", bucket.weighted_count(now));
        }
        // 入れられない
        assert!(!bucket.try_increment(next_window_now, 10));
    }
}
