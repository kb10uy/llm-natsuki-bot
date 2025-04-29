use regex::Regex;
use time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rate {
    Unlimited,
    Limited { duration: Duration, count: usize },
    Prohibited,
}

#[derive(Debug, Clone)]
pub struct RateFilter {
    regex: Regex,
    rate: Rate,
}

impl RateFilter {
    pub fn new(regex: Regex, rate: Rate) -> RateFilter {
        RateFilter { regex, rate }
    }

    pub fn matches(&self, key: &str) -> Option<&Rate> {
        self.regex.is_match(key).then_some(&self.rate)
    }
}
