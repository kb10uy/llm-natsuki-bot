use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Clone, Parser)]
#[clap(author, version)]
pub struct Arguments {
    /// Specify path for config file.
    #[clap(short, long, default_value = "./config.toml")]
    pub config: PathBuf,
}
