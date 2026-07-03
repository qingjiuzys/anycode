use thiserror::Error;

#[derive(Debug, Error)]
pub enum BrowserError {
    #[error("browser not available: {0}")]
    Unavailable(String),
    #[error("invalid URL: {0}")]
    InvalidUrl(String),
    #[error("navigation blocked: {0}")]
    NavigationBlocked(String),
    #[error("session not found: {0}")]
    SessionNotFound(String),
    #[error("tab not found: {0}")]
    TabNotFound(String),
    #[error("element ref not found: {0}")]
    RefNotFound(String),
    #[error("browser locked by {0}")]
    Locked(String),
    #[error("CDP method not allowed: {0}")]
    CdpDenied(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type BrowserResult<T> = Result<T, BrowserError>;
