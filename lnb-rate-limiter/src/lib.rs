mod bucket;
mod rate;

use std::{collections::HashMap, sync::Arc};

use time::UtcDateTime;
use tokio::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rated {
    Success,
    Failure,
}

#[derive(Debug, Clone)]
pub struct RateLimiter {
    default_rate: rate::Rate,
    filters: Arc<[rate::RateFilter]>,
    buckets: Arc<Mutex<HashMap<String, bucket::Bucket>>>,
}

impl RateLimiter {
    pub fn new(default_rate: rate::Rate, filters: impl IntoIterator<Item = rate::RateFilter>) -> RateLimiter {
        RateLimiter {
            default_rate,
            filters: filters.into_iter().collect(),
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn export_buckets(&self) -> HashMap<String, bucket::Bucket> {
        self.buckets.lock().await.clone()
    }

    pub async fn import_buckets(&self, buckets: HashMap<String, bucket::Bucket>) {
        let mut locked = self.buckets.lock().await;
        *locked = buckets;
    }

    pub async fn check(&self, now: UtcDateTime, key: impl Into<String>) -> Rated {
        let key = key.into();
        let (rate_duration, rate_count) = match self.find_rate(&key) {
            rate::Rate::Limited { duration, count } => (*duration, *count),
            rate::Rate::Unlimited => return Rated::Success,
            rate::Rate::Prohibited => return Rated::Failure,
        };

        let mut locked_buckets = self.buckets.lock().await;
        let bucket = {
            let bucket_entry = locked_buckets.entry(key);
            bucket_entry.or_insert_with(|| bucket::Bucket::new_from_now(now, rate_duration))
        };

        if bucket.try_increment(now, rate_count) {
            Rated::Success
        } else {
            Rated::Failure
        }
    }

    fn find_rate(&self, key: &str) -> &rate::Rate {
        self.filters
            .iter()
            .find_map(|f| f.matches(key))
            .unwrap_or(&self.default_rate)
    }
}
