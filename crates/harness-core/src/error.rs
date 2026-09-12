#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid input: {0}")]
    Invalid(String),
    #[error("authorization denied: {0}")]
    Denied(String),
    #[error("run cancelled or deadline elapsed")]
    Cancelled,
    #[error("token budget exhausted")]
    Budget,
    #[error("concurrency capacity exhausted")]
    Capacity,
    #[error("operation requires reconciliation: {0}")]
    Uncertain(String),
    #[error("persistence conflict: {0}")]
    Conflict(String),
    #[error("unsupported capability: {0}")]
    Unsupported(String),
    #[error("host failure: {0}")]
    Host(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
pub type Result<T> = std::result::Result<T, Error>;
