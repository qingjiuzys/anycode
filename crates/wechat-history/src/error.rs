use thiserror::Error;

#[derive(Debug, Error)]
pub enum WechatHistoryError {
    #[error("wechat history is disabled in config")]
    Disabled,
    #[error("wechat history backend not configured: {0}")]
    BackendNotConfigured(String),
    #[error("invalid query: {0}")]
    InvalidQuery(String),
    #[error("HTTP backend error: {0}")]
    Http(String),
    #[error("SQLite backend error: {0}")]
    Sqlite(String),
    #[error("SQLCipher backend error: {0}")]
    SqlCipher(String),
    #[error("unsupported backend: {0}")]
    UnsupportedBackend(String),
}

pub type Result<T> = std::result::Result<T, WechatHistoryError>;
