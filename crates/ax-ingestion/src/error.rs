use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum IngestionError {
    #[error("unsupported input format")]
    UnsupportedFormat,
    #[error("payload is empty")]
    EmptyPayload,
    #[error("payload exceeds configured maximum size")]
    PayloadTooLarge,
    #[error("payload content does not match its declared format")]
    InvalidPayload,
    #[error("project id and filename are required")]
    MissingMetadata,
    #[error("ingestion queue is full; apply backpressure and retry")]
    QueueFull,
    #[error("project hourly ingestion limit reached")]
    RateLimitExceeded,
    #[error("duplicate payload for this project")]
    Duplicate,
    #[error("ingestion state lock is unavailable")]
    StateUnavailable,
    #[error("invalid ingestion configuration")]
    InvalidConfiguration,
}

pub type IngestionResult<T> = Result<T, IngestionError>;
