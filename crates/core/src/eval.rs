//! Runtime-native eval contract shared by agent, dashboard, and `test/run.py`.

use crate::execution_trace::ExecutionTraceEvent;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvalStatus {
    Passed,
    Failed,
    Skipped,
    Error,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvalExpectation {
    #[serde(default)]
    pub event_types: Vec<String>,
    #[serde(default)]
    pub terminal_status: Option<String>,
    #[serde(default)]
    pub contains_text: Option<String>,
    #[serde(default)]
    pub excludes_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalScenario {
    pub id: String,
    pub prompt: String,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub expectations: EvalExpectation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalResult {
    pub scenario_id: String,
    pub status: EvalStatus,
    #[serde(default)]
    pub trace: Vec<ExecutionTraceEvent>,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_text: Option<String>,
}

impl EvalResult {
    #[must_use]
    pub fn passed(scenario_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            scenario_id: scenario_id.into(),
            status: EvalStatus::Passed,
            trace: Vec::new(),
            message: message.into(),
            final_text: None,
        }
    }

    #[must_use]
    pub fn failed(
        scenario_id: impl Into<String>,
        message: impl Into<String>,
        trace: Vec<ExecutionTraceEvent>,
    ) -> Self {
        Self {
            scenario_id: scenario_id.into(),
            status: EvalStatus::Failed,
            trace,
            message: message.into(),
            final_text: None,
        }
    }
}

/// Judge a scenario outcome against expectations (trace + optional final assistant text).
#[must_use]
pub fn judge_eval_scenario(
    scenario: &EvalScenario,
    trace: &[ExecutionTraceEvent],
    terminal_status: &str,
    final_text: Option<&str>,
) -> EvalResult {
    let exp = &scenario.expectations;
    if let Some(expected) = exp.terminal_status.as_deref() {
        if terminal_status != expected {
            return EvalResult::failed(
                &scenario.id,
                format!("expected terminal_status {expected}, got {terminal_status}"),
                trace.to_vec(),
            );
        }
    }
    for want in &exp.event_types {
        if !trace.iter().any(|e| e.event_type == *want) {
            return EvalResult::failed(
                &scenario.id,
                format!("missing trace event_type {want}"),
                trace.to_vec(),
            );
        }
    }
    if let Some(needle) = exp.contains_text.as_deref() {
        let hay = final_text.unwrap_or("");
        if !hay.contains(needle) {
            return EvalResult::failed(
                &scenario.id,
                format!("final text missing substring: {needle}"),
                trace.to_vec(),
            );
        }
    }
    if let Some(needle) = exp.excludes_text.as_deref() {
        let hay = final_text.unwrap_or("");
        if hay.contains(needle) {
            return EvalResult::failed(
                &scenario.id,
                format!("final text must not contain: {needle}"),
                trace.to_vec(),
            );
        }
    }
    let mut ok = EvalResult::passed(&scenario.id, "expectations met");
    ok.trace = trace.to_vec();
    ok.final_text = final_text.map(str::to_string);
    ok
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn judge_checks_terminal_status_and_text() {
        let scenario = EvalScenario {
            id: "s1".into(),
            prompt: "hi".into(),
            agent: None,
            mode: None,
            expectations: EvalExpectation {
                terminal_status: Some("completed".into()),
                contains_text: Some("done".into()),
                ..Default::default()
            },
        };
        let trace = vec![ExecutionTraceEvent::new(
            "task_end",
            "info",
            "done",
            "",
            json!({}),
        )];
        let ok = judge_eval_scenario(&scenario, &trace, "completed", Some("all done"));
        assert_eq!(ok.status, EvalStatus::Passed);
        let bad = judge_eval_scenario(&scenario, &trace, "failed", Some("all done"));
        assert_eq!(bad.status, EvalStatus::Failed);
    }
}
