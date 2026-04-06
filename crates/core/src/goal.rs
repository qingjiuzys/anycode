//! Goal loop state for long-running completion-oriented tasks.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GoalSpec {
    pub objective: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub done_when: Option<String>,
    #[serde(default)]
    pub allow_infinite_retries: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GoalProgress {
    #[serde(default)]
    pub attempts: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_output: Option<String>,
    #[serde(default)]
    pub completed: bool,
}
