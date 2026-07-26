//! Typed task specification produced by the TaskCompiler.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::workflow::WorkflowDefinition;

/// High-level task family used for experience retrieval and workflow recipes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskFamily {
    WebDesign,
    CrossFileCoding,
    Refactor,
    Research,
    OfficeDelivery,
    DatabaseSql,
    General,
}

impl TaskFamily {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WebDesign => "web_design",
            Self::CrossFileCoding => "cross_file_coding",
            Self::Refactor => "refactor",
            Self::Research => "research",
            Self::OfficeDelivery => "office_delivery",
            Self::DatabaseSql => "database_sql",
            Self::General => "general",
        }
    }

    pub fn from_str_loose(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "web" | "web_design" | "webpage" | "frontend" | "html" | "css" => Self::WebDesign,
            "coding" | "cross_file_coding" | "rust" | "code" => Self::CrossFileCoding,
            "refactor" | "cleanup" => Self::Refactor,
            "research" | "docs" | "investigate" => Self::Research,
            "office" | "office_delivery" | "pptx" | "ppt" | "pdf" | "docx" | "slides" => {
                Self::OfficeDelivery
            }
            "database" | "database_sql" | "sql" | "schema" | "ddl" => Self::DatabaseSql,
            _ => Self::General,
        }
    }
}

/// Expected deliverable declared before execution (drives GatePlan).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExpectedArtifact {
    pub id: String,
    pub kind: String,
    #[serde(default = "default_true")]
    pub required: bool,
    #[serde(default)]
    pub path_globs: Vec<String>,
}

fn default_true() -> bool {
    true
}

/// Clarifying question the compiler may ask (max 1–2).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClarifyingQuestion {
    pub id: String,
    pub prompt: String,
    #[serde(default)]
    pub options: Vec<String>,
}

/// Per-agent minimal prompt pack (not the whole knowledge base).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct AgentPromptPack {
    pub agent_id: String,
    pub role: String,
    pub objective: String,
    #[serde(default)]
    pub constraints: Vec<String>,
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    #[serde(default)]
    pub done_when: Vec<String>,
    #[serde(default)]
    pub experience_examples: Vec<String>,
    #[serde(default)]
    pub recovery_hints: Vec<String>,
}

/// Typed task compiled from a user request.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskSpec {
    pub goal: String,
    #[serde(default)]
    pub family: Option<TaskFamily>,
    #[serde(default)]
    pub constraints: Vec<String>,
    #[serde(default)]
    pub deliverables: Vec<String>,
    /// Fine-grained capabilities derived from the task itself (not Experience).
    #[serde(default)]
    pub required_capabilities: Vec<String>,
    /// Expected artifacts used to build GatePlan before execution.
    #[serde(default)]
    pub expected_artifacts: Vec<ExpectedArtifact>,
    #[serde(default)]
    pub acceptance: Vec<String>,
    #[serde(default)]
    pub risks: Vec<String>,
    #[serde(default)]
    pub missing_preferences: Vec<String>,
    #[serde(default)]
    pub clarifying_questions: Vec<ClarifyingQuestion>,
    #[serde(default)]
    pub preference_hits: Vec<String>,
    #[serde(default)]
    pub experience_card_ids: Vec<String>,
    #[serde(default)]
    pub agent_packs: Vec<AgentPromptPack>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow: Option<WorkflowDefinition>,
    #[serde(default)]
    pub extras: HashMap<String, String>,
}

impl TaskSpec {
    /// Compact segment text for system prompt injection / A/B observability.
    pub fn to_prompt_segment(&self) -> String {
        let mut lines = vec![
            "## Task Spec".to_string(),
            format!("goal: {}", self.goal.trim()),
        ];
        if let Some(family) = self.family {
            lines.push(format!("family: {}", family.as_str()));
        }
        if !self.constraints.is_empty() {
            lines.push(format!("constraints: {}", self.constraints.join("; ")));
        }
        if !self.deliverables.is_empty() {
            lines.push(format!("deliverables: {}", self.deliverables.join("; ")));
        }
        if !self.required_capabilities.is_empty() {
            lines.push(format!(
                "capabilities: {}",
                self.required_capabilities.join(", ")
            ));
        }
        if !self.expected_artifacts.is_empty() {
            let arts: Vec<_> = self
                .expected_artifacts
                .iter()
                .map(|a| format!("{}:{}", a.id, a.kind))
                .collect();
            lines.push(format!("expected_artifacts: {}", arts.join(", ")));
        }
        if !self.acceptance.is_empty() {
            lines.push(format!("acceptance: {}", self.acceptance.join("; ")));
        }
        if !self.preference_hits.is_empty() {
            lines.push(format!(
                "preferences_applied: {}",
                self.preference_hits.join("; ")
            ));
        }
        if !self.experience_card_ids.is_empty() {
            lines.push(format!(
                "experience_cards: {}",
                self.experience_card_ids.join(", ")
            ));
        }
        if !self.missing_preferences.is_empty() {
            lines.push(format!(
                "missing_preferences: {}",
                self.missing_preferences.join("; ")
            ));
        }
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_parse_and_segment() {
        assert_eq!(TaskFamily::from_str_loose("webpage"), TaskFamily::WebDesign);
        assert_eq!(
            TaskFamily::from_str_loose("docx"),
            TaskFamily::OfficeDelivery
        );
        assert_eq!(TaskFamily::from_str_loose("sql"), TaskFamily::DatabaseSql);
        let spec = TaskSpec {
            goal: "build landing".into(),
            family: Some(TaskFamily::WebDesign),
            preference_hits: vec!["dark theme".into()],
            ..Default::default()
        };
        let seg = spec.to_prompt_segment();
        assert!(seg.contains("family: web_design"));
        assert!(seg.contains("dark theme"));
    }
}
