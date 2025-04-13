use std::path::PathBuf;

use clap::Parser;
use thiserror::Error as ThisError;

#[derive(Debug, Clone, Parser)]
#[clap(author, version)]
pub struct Arguments {
    /// Specify path for config file.
    #[clap(short, long, default_value = "./config.toml")]
    pub config: PathBuf,

    #[clap(short, long, value_parser = parse_debug_option)]
    pub debug_options: Vec<(String, DebugOptionValue)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugOptionValue {
    Enabled,
    Disabled,
    Specified(String),
}

#[derive(Debug, Clone, ThisError)]
#[error("invalid syntax, +/- prefix or = separated expected: {0}")]
struct InvalidDebugOption(String);

fn parse_debug_option(s: &str) -> Result<(String, DebugOptionValue), InvalidDebugOption> {
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
