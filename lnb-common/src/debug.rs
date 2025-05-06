use std::{
    collections::HashMap,
    sync::{LazyLock, RwLock},
};

use thiserror::Error as ThisError;

static DEBUG_OPTIONS: LazyLock<RwLock<HashMap<String, DebugOptionValue>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub fn set_debug_options(options: HashMap<String, DebugOptionValue>) {
    let mut debug_options = DEBUG_OPTIONS.write().expect("poisoned");
    *debug_options = options;
}

pub fn get_debug_option(name: &str) -> Option<DebugOptionValue> {
    let debug_options = DEBUG_OPTIONS.read().expect("poisoned");
    debug_options.get(name).cloned()
}

/// デバッグ用オプション定義。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugOptionValue {
    Enabled,
    Disabled,
    Specified(String),
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

#[derive(Debug, Clone, ThisError)]
#[error("invalid syntax, +/- prefix or = separated expected: {0}")]
pub struct InvalidDebugOption(String);
