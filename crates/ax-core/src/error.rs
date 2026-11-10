use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AxError {
    #[error("capacity value overflow")]
    CapacityOverflow,
    #[error("invalid capacity: {0}")]
    InvalidCapacity(&'static str),
    #[error("invalid identifier: {0}")]
    InvalidIdentifier(&'static str),
    #[error("invalid resource: {0}")]
    InvalidResource(&'static str),
    #[error("resource is not available for reservation")]
    ResourceUnavailable,
    #[error("physical capacity has not been verified by a node probe")]
    HardwareUnverified,
}

pub type AxResult<T> = Result<T, AxError>;
