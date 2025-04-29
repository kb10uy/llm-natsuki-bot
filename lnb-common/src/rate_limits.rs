use std::{fs::read_to_string, io::Error as IoError, path::Path};

use lnb_rate_limiter::{Rate, RateFilter, RateLimiter};
use regex::{Error as RegexError, Regex};
use serde::Deserialize;
use serde_json::Error as SerdeJsonError;
use thiserror::Error as ThisError;
use time::Duration;

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimits {
    pub conversation: RateLimitsCategory,
    pub image_generator: RateLimitsCategory,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RateLimitsCategory {
    pub default: RateLimitsRateDefinition,
    pub filters: Vec<RateLimitsFilterDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RateLimitsFilterDefinition {
    pub pattern: String,
    pub rate: RateLimitsRateDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RateLimitsRateDefinition {
    Unlimited,
    Prohibited,

    #[serde(untagged)]
    Limited {
        duration_seconds: usize,
        count: usize,
    },
}

pub fn load_rate_limits(path: impl AsRef<Path>) -> Result<RateLimits, RateLimitsError> {
    let config_str = read_to_string(path).map_err(RateLimitsError::Io)?;
    let config = serde_json::from_str(&config_str).map_err(RateLimitsError::Serialization)?;
    Ok(config)
}

impl TryFrom<RateLimitsCategory> for RateLimiter {
    type Error = RateLimitsError;

    fn try_from(value: RateLimitsCategory) -> Result<RateLimiter, RateLimitsError> {
        let filters: Result<Vec<_>, _> = value.filters.into_iter().map(|f| f.try_into()).collect();
        Ok(RateLimiter::new(value.default.into(), filters?))
    }
}

impl TryFrom<RateLimitsFilterDefinition> for RateFilter {
    type Error = RateLimitsError;

    fn try_from(value: RateLimitsFilterDefinition) -> Result<RateFilter, RateLimitsError> {
        Ok(RateFilter::new(
            Regex::new(&value.pattern).map_err(RateLimitsError::Regex)?,
            value.rate.into(),
        ))
    }
}

impl From<RateLimitsRateDefinition> for Rate {
    fn from(value: RateLimitsRateDefinition) -> Rate {
        match value {
            RateLimitsRateDefinition::Unlimited => Rate::Unlimited,
            RateLimitsRateDefinition::Prohibited => Rate::Prohibited,
            RateLimitsRateDefinition::Limited {
                duration_seconds,
                count,
            } => Rate::Limited {
                duration: Duration::seconds(duration_seconds as i64),
                count,
            },
        }
    }
}

#[derive(Debug, ThisError)]
pub enum RateLimitsError {
    #[error("io error: {0}")]
    Io(IoError),

    #[error("serialization error: {0}")]
    Serialization(SerdeJsonError),

    #[error("regex error: {0}")]
    Regex(RegexError),
}
