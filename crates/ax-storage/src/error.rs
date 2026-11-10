use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("manifest serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("storage integrity check failed")]
    IntegrityMismatch,
    #[error("storage quota exceeded")]
    QuotaExceeded,
    #[error("object not found")]
    NotFound,
    #[error("invalid storage configuration: {0}")]
    InvalidConfiguration(&'static str),
    #[error("storage accounting lock poisoned")]
    AccountingUnavailable,
    #[error("object size overflow")]
    SizeOverflow,
}

pub type StorageResult<T> = Result<T, StorageError>;
