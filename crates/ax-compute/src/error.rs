use thiserror::Error;

#[derive(Debug, Error)]
pub enum ComputeError {
    #[error("invalid compute job: {0}")]
    InvalidJob(&'static str),
    #[error("executable is not allowed by the node policy")]
    ExecutableNotAllowed,
    #[error("working directory is outside the configured sandbox root")]
    WorkingDirectoryOutsideSandbox,
    #[error("reservation is invalid or no longer active: {0}")]
    InvalidReservation(String),
    #[error("compute policy is invalid: {0}")]
    InvalidPolicy(&'static str),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("output reader thread failed")]
    OutputReaderFailed,
    #[error("compute state lock is poisoned")]
    LockPoisoned,
}

pub type ComputeResult<T> = Result<T, ComputeError>;
