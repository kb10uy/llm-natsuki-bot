use std::path::PathBuf;

use clap::Parser;
use lnb_common::debug::{DebugOptionValue, parse_debug_option};

#[derive(Debug, Clone, Parser)]
#[clap(author, version)]
pub struct Arguments {
    /// Specify path for config file.
    #[clap(short, long, default_value = "./config.json")]
    pub config: PathBuf,

    /// Specify path for config file.
    #[clap(short, long, default_value = "./rate-limits.json")]
    pub rate_limits: PathBuf,

    /// Specify path for config file.
    #[clap(short, long, default_value = "./user-roles.json")]
    pub user_roles: PathBuf,

    #[clap(short, long, value_parser = parse_debug_option)]
    pub debug_options: Vec<(String, DebugOptionValue)>,
}
