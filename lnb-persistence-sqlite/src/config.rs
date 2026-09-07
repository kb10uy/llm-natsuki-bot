use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ConfigStorageSqlite {
    pub filepath: PathBuf,
}
