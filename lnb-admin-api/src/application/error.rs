use std::error::Error as StdError;

use thiserror::Error as ThisError;

type ErasedError = Box<dyn StdError + Send + Sync + 'static>;

#[derive(Debug, ThisError)]
pub enum ApplicationError {
    #[error("backend error: {0}")]
    Backend(#[source] ErasedError),

    #[error("serialization error: {0}")]
    Serialization(#[source] ErasedError),
}

impl ApplicationError {
    pub fn by_serialization(source: impl Into<ErasedError>) -> ApplicationError {
        ApplicationError::Serialization(source.into())
    }

    pub fn by_backend(source: impl Into<ErasedError>) -> ApplicationError {
        ApplicationError::Backend(source.into())
    }
}
