//! Minimal YAML workflow schema.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowDefinition {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default)]
    pub steps: Vec<WorkflowStep>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry: Option<WorkflowRetry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub done_when: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handoff: Option<WorkflowHandoff>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowStep {
    pub id: String,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub done_when: Option<String>,
    #[serde(default)]
    pub vars: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowRetry {
    #[serde(default = "default_attempts")]
    pub max_attempts: u32,
    #[serde(default)]
    pub backoff_ms: u64,
}

fn default_attempts() -> u32 {
    3
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowHandoff {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
