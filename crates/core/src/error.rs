//! 核心错误类型。

use crate::ids::{AgentId, ToolName};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Agent not found: {0}")]
    AgentNotFound(AgentId),

    #[error("Tool not found: {0}")]
    ToolNotFound(ToolName),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Config error: {0}")]
    ConfigError(String),

    #[error("LLM error: {0}")]
    LLMError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}
