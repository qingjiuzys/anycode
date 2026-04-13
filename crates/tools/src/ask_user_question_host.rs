//! Host-mediated UI for the `AskUserQuestion` tool (REPL / TUI attach a channel-backed impl).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// One selectable option from the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskUserQuestionOption {
    pub label: String,
    #[serde(default, alias = "description")]
    pub description: String,
}

/// Payload sent to a host (mirrors tool JSON).
#[derive(Debug, Clone)]
pub struct AskUserQuestionRequest {
    pub question: String,
    pub header: String,
    pub options: Vec<AskUserQuestionOption>,
    pub multi_select: bool,
}

/// Successful selection: one or more option labels (order preserved).
#[derive(Debug, Clone)]
pub struct AskUserQuestionResponse {
    pub selected_labels: Vec<String>,
}

/// User cancelled or host could not prompt (e.g. multi-select on a single-select-only UI).
#[derive(Debug, Clone)]
pub struct AskUserQuestionHostError(pub String);

impl std::fmt::Display for AskUserQuestionHostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for AskUserQuestionHostError {}

/// Interactive host for `AskUserQuestion` (CLI dialoguer, TUI mpsc, etc.).
#[async_trait]
pub trait AskUserQuestionHost: Send + Sync {
    async fn ask_user_question(
        &self,
        request: AskUserQuestionRequest,
    ) -> Result<AskUserQuestionResponse, AskUserQuestionHostError>;
}

pub type AskUserQuestionHostArc = Arc<dyn AskUserQuestionHost>;
