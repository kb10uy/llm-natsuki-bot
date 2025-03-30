pub mod error;
pub mod interface;
pub mod model;

/// クライアントに設定する UserAgent。
pub const APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
