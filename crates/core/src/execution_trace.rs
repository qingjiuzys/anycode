//! Structured execution trace events shared by runtime, dashboard, and eval.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const EXECUTION_TRACE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionTraceEvent {
    pub schema_version: u32,
    pub event_type: String,
    pub severity: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub body: String,
    #[serde(default)]
    pub payload: Value,
    pub occurred_at: String,
}

impl ExecutionTraceEvent {
    #[must_use]
    pub fn new(
        event_type: impl Into<String>,
        severity: impl Into<String>,
        title: impl Into<String>,
        body: impl Into<String>,
        payload: Value,
    ) -> Self {
        Self {
            schema_version: EXECUTION_TRACE_SCHEMA_VERSION,
            event_type: event_type.into(),
            severity: severity.into(),
            title: title.into(),
            body: body.into(),
            payload,
            occurred_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    #[must_use]
    pub fn from_log_line(line: &str) -> Option<Self> {
        let line = line.trim();
        let rest = line.strip_prefix('[')?;
        let (tag, kv) = rest.split_once(']')?;
        let fields = parse_kv(kv.trim());
        let payload = fields_to_json(&fields);
        let field = |key: &str| fields.get(key).cloned().unwrap_or_default();
        match tag {
            "task_start" => Some(Self::new(
                tag,
                "info",
                format!("Task started ({})", field("agent_type")),
                kv.trim(),
                payload,
            )),
            "task_end" => {
                let status = non_empty(field("status"), "unknown");
                let severity = match status.as_str() {
                    "failed" => "error",
                    "cancelled" => "warn",
                    _ => "info",
                };
                Some(Self::new(
                    tag,
                    severity,
                    format!("Task {status}"),
                    kv.trim(),
                    payload,
                ))
            }
            "turn_start" => Some(Self::new(
                tag,
                "info",
                format!("Turn {}", field("turn")),
                "",
                payload,
            )),
            "turn_end" => Some(Self::new(
                tag,
                "info",
                format!("Turn {} finished", field("turn")),
                "",
                payload,
            )),
            "llm_request_start" => Some(Self::new(
                tag,
                "info",
                format!("LLM {} turn {}", field("model"), field("turn")),
                "",
                payload,
            )),
            "llm_response_end" => {
                let input = field("input_tokens");
                let output = field("output_tokens");
                let title = if !input.is_empty() || !output.is_empty() {
                    format!("LLM response ({input} in / {output} out tokens)")
                } else {
                    "LLM response".to_string()
                };
                let severity = if field("status") == "error" {
                    "error"
                } else {
                    "info"
                };
                Some(Self::new(tag, severity, title, "", payload))
            }
            "tool_call_input" => Some(Self::new(
                tag,
                "info",
                format!("{} input", field("name")),
                kv.trim(),
                payload,
            )),
            "tool_call_start" => Some(Self::new(
                tag,
                "info",
                format!("{} started", field("name")),
                "",
                payload,
            )),
            "tool_call_end" => {
                let err = field("error");
                let failed = err != "<none>" && !err.is_empty();
                Some(Self::new(
                    tag,
                    if failed { "error" } else { "info" },
                    format!(
                        "{} {}",
                        field("name"),
                        if failed { "failed" } else { "finished" }
                    ),
                    if failed { err } else { String::new() },
                    payload,
                ))
            }
            "tool_denied" => Some(Self::new(
                tag,
                "warn",
                format!("{} denied", field("name")),
                field("reason"),
                payload,
            )),
            "tool_approval_pending" => Some(Self::new(
                tag,
                "warn",
                format!("{} awaiting approval", field("name")),
                "",
                payload,
            )),
            "tool_approval_resolved" => Some(Self::new(
                tag,
                "info",
                format!("{} approved", field("name")),
                "",
                payload,
            )),
            "gate" => Some(Self::new(
                tag,
                if field("status") == "failed" {
                    "error"
                } else {
                    "info"
                },
                format!("Gate: {}", field("name")),
                field("output"),
                payload,
            )),
            "budget_warning" | "budget_degrade" | "budget_exceeded" => Some(Self::new(
                tag,
                if tag == "budget_exceeded" {
                    "error"
                } else {
                    "warn"
                },
                match tag {
                    "budget_warning" => "Budget warning".to_string(),
                    "budget_degrade" => "Budget degradation".to_string(),
                    _ => "Budget exceeded".to_string(),
                },
                kv.trim(),
                payload,
            )),
            "user_prompt" => Some(Self::new(tag, "info", "User prompt", "", payload)),
            "assistant_response" => Some(Self::new(
                tag,
                "info",
                format!("Assistant (turn {})", field("turn")),
                "",
                payload,
            )),
            _ => None,
        }
    }
}

fn parse_kv(kv: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for part in kv.split_whitespace() {
        if let Some((k, v)) = part.split_once('=') {
            map.insert(k.to_string(), v.to_string());
        }
    }
    map
}

fn fields_to_json(fields: &BTreeMap<String, String>) -> Value {
    let mut map = serde_json::Map::new();
    for (k, v) in fields {
        map.insert(k.clone(), Value::String(v.clone()));
    }
    Value::Object(map)
}

fn non_empty(value: String, fallback: &str) -> String {
    if value.is_empty() {
        fallback.to_string()
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_trace_from_tool_end_log_line() {
        let event = ExecutionTraceEvent::from_log_line(
            "[tool_call_end] turn=1 idx=1 name=Bash elapsed_ms=7 error=<none>",
        )
        .unwrap();
        assert_eq!(event.schema_version, EXECUTION_TRACE_SCHEMA_VERSION);
        assert_eq!(event.event_type, "tool_call_end");
        assert_eq!(event.severity, "info");
        assert_eq!(event.payload["name"], "Bash");
    }

    #[test]
    fn builds_trace_from_budget_exceeded_log_line() {
        let event = ExecutionTraceEvent::from_log_line(
            "[budget_exceeded] consumed_tokens=100 token_budget=4 consumed_cost_usd=0.000000 cost_budget_usd=<none> elapsed_secs=0 max_duration_secs=<none>",
        )
        .unwrap();
        assert_eq!(event.event_type, "budget_exceeded");
        assert_eq!(event.severity, "error");
    }

    #[test]
    fn marks_failed_task_as_error() {
        let event = ExecutionTraceEvent::from_log_line("[task_end] status=failed").unwrap();
        assert_eq!(event.severity, "error");
        assert_eq!(event.title, "Task failed");
    }
}
