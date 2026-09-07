use std::error::Error as StdError;

use thiserror::Error as ThisError;

type ErasedError = Box<dyn StdError + Send + Sync + 'static>;

#[derive(Debug, ThisError)]
pub enum PersistenceError {
    #[error("backend error: {0}")]
    Backend(#[source] ErasedError),

    #[error("serialization error: {0}")]
    Serialization(#[source] ErasedError),
}

impl PersistenceError {
    pub fn by_serialization(source: impl Into<ErasedError>) -> PersistenceError {
        PersistenceError::Serialization(source.into())
    }

    pub fn by_backend(source: impl Into<ErasedError>) -> PersistenceError {
        PersistenceError::Backend(source.into())
    }
}
