use std::{
    collections::HashMap,
    str::FromStr,
    sync::{LazyLock, RwLock},
};

use thiserror::Error as ThisError;

static DEBUG_OPTIONS: LazyLock<RwLock<HashMap<String, DebugOptionValue>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub fn set_debug_options(options: HashMap<String, DebugOptionValue>) {
    let mut debug_options = DEBUG_OPTIONS.write().expect("poisoned");
    *debug_options = options;
}

pub fn debug_option_enabled(name: &str) -> Option<bool> {
    let debug_options = DEBUG_OPTIONS.read().expect("poisoned");
    let value = debug_options.get(name);
    value.map(DebugOptionValue::is_enabled)
}

pub fn debug_option_disabled(name: &str) -> Option<bool> {
    let debug_options = DEBUG_OPTIONS.read().expect("poisoned");
    let value = debug_options.get(name);
    value.map(DebugOptionValue::is_disabled)
}

pub fn debug_option_value(name: &str) -> Option<String> {
    let debug_options = DEBUG_OPTIONS.read().expect("poisoned");
    Some(debug_options.get(name).and_then(DebugOptionValue::value)?.to_string())
}

pub fn debug_option_parsed<T: FromStr>(name: &str) -> Result<Option<String>, DebugOptionError> {
    let debug_options = DEBUG_OPTIONS.read().expect("poisoned");
    debug_options
        .get(name)
        .and_then(DebugOptionValue::value)
        .map(str::parse)
        .transpose()
        .map_err(|_| DebugOptionError::Parse)
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

pub fn parse_debug_option(s: &str) -> Result<(String, DebugOptionValue), DebugOptionError> {
    if let Some(name) = s.strip_prefix('+') {
        Ok((name.to_string(), DebugOptionValue::Enabled))
    } else if let Some(name) = s.strip_prefix('-') {
        Ok((name.to_string(), DebugOptionValue::Disabled))
    } else if let Some((key, value)) = s.split_once('=') {
        Ok((key.to_string(), DebugOptionValue::Specified(value.to_string())))
    } else {
        Err(DebugOptionError::Syntax(s.to_string()))
    }
}

#[derive(Debug, Clone, ThisError)]
pub enum DebugOptionError {
    #[error("invalid syntax, +/- prefix or = separated expected: {0}")]
    Syntax(String),

    #[error("parse error")]
    Parse,
}
