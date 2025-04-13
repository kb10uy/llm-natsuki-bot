pub mod error;
pub mod interface;
pub mod model;

use thiserror::Error as ThisError;
use time::{format_description::BorrowedFormatItem, macros::format_description};

/// クライアントに設定する UserAgent。
pub const APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

/// RFC3339 拡張形式のタイムゾーンをつねに数値表記にしたもの。
pub const RFC3339_NUMOFFSET: &[BorrowedFormatItem<'static>] =
    format_description!("[year]-[month]-[day]T[hour]:[minute]:[second][offset_hour sign:mandatory]:[offset_minute]");

/// デバッグ用オプション定義。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugOptionValue {
    Enabled,
    Disabled,
    Specified(String),
}

#[derive(Debug, Clone, ThisError)]
#[error("invalid syntax, +/- prefix or = separated expected: {0}")]
pub struct InvalidDebugOption(String);

pub fn parse_debug_option(s: &str) -> Result<(String, DebugOptionValue), InvalidDebugOption> {
    if let Some(name) = s.strip_prefix('+') {
        Ok((name.to_string(), DebugOptionValue::Enabled))
    } else if let Some(name) = s.strip_prefix('-') {
        Ok((name.to_string(), DebugOptionValue::Disabled))
    } else if let Some((key, value)) = s.split_once('=') {
        Ok((key.to_string(), DebugOptionValue::Specified(value.to_string())))
    } else {
        Err(InvalidDebugOption(s.to_string()))
    }
}

impl DebugOptionValue {
    pub fn value(&self) -> Option<&str> {
        match self {
            DebugOptionValue::Specified(s) => Some(s),
            _ => None,
        }
    }

    pub fn is_enabled(&self) -> bool {
        matches!(self, DebugOptionValue::Enabled)
    }

    pub fn is_disabled(&self) -> bool {
        matches!(self, DebugOptionValue::Disabled)
    }
}
