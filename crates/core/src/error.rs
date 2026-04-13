//! 核心错误类型。

use crate::ids::{AgentId, ToolName};
use crate::task::NESTED_TASK_COOPERATIVE_CANCEL_ERROR;
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

    /// User- or host-requested cooperative cancel (turn / nested task). Display matches legacy `LLMError("cancelled")`.
    #[error("LLM error: cancelled")]
    CooperativeCancel,

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}

impl CoreError {
    /// `true` for structured cooperative cancel or the legacy `LLMError("cancelled")` form.
    #[must_use]
    pub fn is_cooperative_cancel(&self) -> bool {
        matches!(self, Self::CooperativeCancel)
            || matches!(
                self,
                Self::LLMError(s) if s.as_str() == NESTED_TASK_COOPERATIVE_CANCEL_ERROR
            )
    }
}

/// When [`anyhow::Error`] was built with `From<CoreError>` (e.g. `map_err(anyhow::Error::from)`), detects cooperative cancel.
#[must_use]
pub fn anyhow_error_is_cooperative_cancel(e: &anyhow::Error) -> bool {
    e.downcast_ref::<CoreError>()
        .is_some_and(CoreError::is_cooperative_cancel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cooperative_cancel_display_matches_legacy_llm_error() {
        let a = CoreError::CooperativeCancel.to_string();
        let b = CoreError::LLMError(NESTED_TASK_COOPERATIVE_CANCEL_ERROR.to_string()).to_string();
        assert_eq!(a, b);
        assert_eq!(a, "LLM error: cancelled");
    }

    #[test]
    fn anyhow_error_is_cooperative_cancel_downcasts() {
        let e = anyhow::Error::from(CoreError::CooperativeCancel);
        assert!(anyhow_error_is_cooperative_cancel(&e));
    }
}
