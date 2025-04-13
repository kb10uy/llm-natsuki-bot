use std::path::PathBuf;

use clap::Parser;
use lnb_core::{DebugOptionValue, parse_debug_option};

#[derive(Debug, Clone, Parser)]
#[clap(author, version)]
pub struct Arguments {
    /// Specify path for config file.
    #[clap(short, long, default_value = "./config.toml")]
    pub config: PathBuf,

    #[clap(short, long, value_parser = parse_debug_option)]
    pub debug_options: Vec<(String, DebugOptionValue)>,
}
