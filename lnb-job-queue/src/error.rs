use std::error::Error as StdError;

use redis::RedisError;
use thiserror::Error as ThisError;

type ErasedError = Box<dyn StdError + Send + Sync + 'static>;

#[derive(Debug, ThisError)]
pub enum WorkerError {
    #[error("internal Redis error: {0}")]
    Internal(#[from] RedisError),

    #[error("serialization error: {0}")]
    Serialization(ErasedError),

    #[error("cannot push job anymore")]
    CannotPushAnymore,
}

impl WorkerError {
    pub fn by_serialization(source: impl Into<ErasedError>) -> WorkerError {
        WorkerError::Serialization(source.into())
    }
}
