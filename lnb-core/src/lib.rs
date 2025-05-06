pub mod error;
pub mod interface;
pub mod model;

use time::{format_description::BorrowedFormatItem, macros::format_description};

/// クライアントに設定する UserAgent。
pub const APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

/// RFC3339 拡張形式のタイムゾーンをつねに数値表記にしたもの。
pub const RFC3339_NUMOFFSET: &[BorrowedFormatItem<'static>] =
    format_description!("[year]-[month]-[day]T[hour]:[minute]:[second][offset_hour sign:mandatory]:[offset_minute]");
