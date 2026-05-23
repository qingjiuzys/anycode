//! Parse `output.log` structured lines into dashboard events.

use anycode_core::{decode_log_text, EXECUTION_TRACE_SCHEMA_VERSION};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedLine {
    pub event_type: String,
    pub severity: String,
    pub title: String,
    pub body: String,
    pub payload: Value,
}

/// Parse a single log line; returns `None` for blank lines or unrecognized content.
#[must_use]
pub fn parse_line(line: &str) -> Option<ParsedLine> {
    let line = line.trim_end();
    if line.is_empty() || line.starts_with("== ") {
        return None;
    }
    if line.starts_with('{') {
        return parse_trace_json_line(line);
    }
    if let Some(rest) = line.strip_prefix('[') {
        if let Some((tag, kv)) = rest.split_once(']') {
            return parse_tagged(tag, kv.trim());
        }
    }
    None
}

fn parse_trace_json_line(line: &str) -> Option<ParsedLine> {
    let value: Value = serde_json::from_str(line).ok()?;
    let version = value.get("schema_version")?.as_u64()?;
    if version != u64::from(EXECUTION_TRACE_SCHEMA_VERSION) {
        return None;
    }
    let event_type = value.get("event_type")?.as_str()?.to_string();
    let severity = value
        .get("severity")
        .and_then(|v| v.as_str())
        .unwrap_or("info")
        .to_string();
    let title = value
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or(&event_type)
        .to_string();
    let body = value
        .get("body")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let payload = value
        .get("payload")
        .cloned()
        .unwrap_or_else(|| Value::Object(Default::default()));
    Some(ParsedLine {
        event_type,
        severity,
        title,
        body,
        payload,
    })
}

fn parse_tagged(tag: &str, kv: &str) -> Option<ParsedLine> {
    let fields = parse_kv(kv);
    match tag {
        "task_start" => Some(ParsedLine {
            event_type: "task_start".into(),
            severity: "info".into(),
            title: format!("Task started ({})", field(&fields, "agent_type")),
            body: kv.into(),
            payload: fields_to_json(&fields),
        }),
        "task_end" => {
            let status = field(&fields, "status");
            let status = if status.is_empty() {
                "unknown".into()
            } else {
                status
            };
            let severity = if status == "failed" {
                "error"
            } else if status == "cancelled" {
                "warn"
            } else {
                "info"
            };
            Some(ParsedLine {
                event_type: "task_end".into(),
                severity: severity.into(),
                title: format!("Task {status}"),
                body: kv.into(),
                payload: fields_to_json(&fields),
            })
        }
        "turn_start" => Some(ParsedLine {
            event_type: "turn_start".into(),
            severity: "info".into(),
            title: format!("Turn {}", field(&fields, "turn")),
            body: String::new(),
            payload: fields_to_json(&fields),
        }),
        "turn_end" => Some(ParsedLine {
            event_type: "turn_end".into(),
            severity: "info".into(),
            title: format!("Turn {} finished", field(&fields, "turn")),
            body: String::new(),
            payload: fields_to_json(&fields),
        }),
        "llm_request_start" => Some(ParsedLine {
            event_type: "llm_request_start".into(),
            severity: "info".into(),
            title: format!(
                "LLM {} turn {}",
                field(&fields, "model"),
                field(&fields, "turn")
            ),
            body: String::new(),
            payload: fields_to_json(&fields),
        }),
        "llm_response_end" => {
            let input = field(&fields, "input_tokens");
            let output = field(&fields, "output_tokens");
            let title = if !input.is_empty() || !output.is_empty() {
                format!("LLM response ({input} in / {output} out tokens)")
            } else {
                "LLM response".into()
            };
            Some(ParsedLine {
                event_type: "llm_response_end".into(),
                severity: "info".into(),
                title,
                body: String::new(),
                payload: fields_to_json(&fields),
            })
        }
        "tool_call_start" => Some(ParsedLine {
            event_type: "tool_call_start".into(),
            severity: "info".into(),
            title: format!("{} started", field(&fields, "name")),
            body: String::new(),
            payload: fields_to_json(&fields),
        }),
        "tool_call_end" => {
            let err = field(&fields, "error");
            let failed = err != "<none>" && !err.is_empty();
            Some(ParsedLine {
                event_type: "tool_call_end".into(),
                severity: if failed { "error" } else { "info" }.into(),
                title: format!(
                    "{} {}",
                    field(&fields, "name"),
                    if failed { "failed" } else { "finished" }
                ),
                body: if failed { err } else { String::new() },
                payload: fields_to_json(&fields),
            })
        }
        "tool_denied" => {
            let name = field(&fields, "name");
            let reason = field(&fields, "reason");
            Some(ParsedLine {
                event_type: "tool_denied".into(),
                severity: "warn".into(),
                title: format!("{name} denied"),
                body: reason.clone(),
                payload: fields_to_json(&fields),
            })
        }
        "tool_approval_pending" => {
            let name = field(&fields, "name");
            Some(ParsedLine {
                event_type: "tool_approval_pending".into(),
                severity: "warn".into(),
                title: format!("{name} awaiting approval"),
                body: String::new(),
                payload: fields_to_json(&fields),
            })
        }
        "tool_approval_resolved" => {
            let name = field(&fields, "name");
            Some(ParsedLine {
                event_type: "tool_approval_resolved".into(),
                severity: "info".into(),
                title: format!("{name} approved"),
                body: String::new(),
                payload: fields_to_json(&fields),
            })
        }
        "gate" => Some(ParsedLine {
            event_type: "gate".into(),
            severity: match fields.get("status").map(String::as_str) {
                Some("failed") => "error",
                Some("passed") => "info",
                _ => "info",
            }
            .into(),
            title: format!("Gate: {}", field(&fields, "name")),
            body: field(&fields, "output"),
            payload: fields_to_json(&fields),
        }),
        "budget_warning" | "budget_degrade" | "budget_exceeded" => Some(ParsedLine {
            event_type: tag.into(),
            severity: if tag == "budget_exceeded" {
                "error"
            } else {
                "warn"
            }
            .into(),
            title: match tag {
                "budget_warning" => "Budget warning",
                "budget_degrade" => "Budget degradation",
                _ => "Budget exceeded",
            }
            .into(),
            body: kv.into(),
            payload: fields_to_json(&fields),
        }),
        "user_prompt" => {
            let text = extract_text_suffix(kv);
            Some(ParsedLine {
                event_type: "user_prompt".into(),
                severity: "info".into(),
                title: "User prompt".into(),
                body: text,
                payload: fields_to_json(&fields),
            })
        }
        "assistant_response" => {
            let text = extract_text_suffix(kv);
            Some(ParsedLine {
                event_type: "assistant_response".into(),
                severity: "info".into(),
                title: format!("Assistant (turn {})", field(&fields, "turn")),
                body: text,
                payload: fields_to_json(&fields),
            })
        }
        _ => None,
    }
}

fn extract_text_suffix(kv: &str) -> String {
    if let Some(pos) = kv.find(" text=") {
        decode_log_text(kv[pos + 6..].trim())
    } else if let Some(rest) = kv.strip_prefix("text=") {
        decode_log_text(rest.trim())
    } else {
        String::new()
    }
}

/// Extract task end status from parsed lines in a chunk.
#[must_use]
pub fn task_end_status(lines: &[&str]) -> Option<String> {
    for line in lines.iter().rev() {
        if let Some(p) = parse_line(line) {
            if p.event_type == "task_end" {
                return p
                    .payload
                    .get("status")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
            }
        }
    }
    None
}

fn parse_kv(kv: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for part in kv.split_whitespace() {
        if let Some((k, v)) = part.split_once('=') {
            map.insert(k.to_string(), v.to_string());
        }
    }
    map
}

fn fields_to_json(fields: &HashMap<String, String>) -> Value {
    let mut map = serde_json::Map::new();
    for (k, v) in fields {
        map.insert(k.clone(), Value::String(v.clone()));
    }
    Value::Object(map)
}

fn field(fields: &HashMap<String, String>, key: &str) -> String {
    fields.get(key).cloned().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tool_denied() {
        let p = parse_line("[tool_denied] name=Bash reason=User denied").unwrap();
        assert_eq!(p.event_type, "tool_denied");
        assert_eq!(p.severity, "warn");
        assert_eq!(p.payload["name"], "Bash");
    }

    #[test]
    fn parses_tool_approval_pending() {
        let p = parse_line("[tool_approval_pending] name=FileWrite").unwrap();
        assert_eq!(p.event_type, "tool_approval_pending");
        assert_eq!(p.severity, "warn");
    }

    #[test]
    fn parses_tool_approval_resolved() {
        let p = parse_line("[tool_approval_resolved] name=Bash").unwrap();
        assert_eq!(p.event_type, "tool_approval_resolved");
        assert_eq!(p.severity, "info");
    }

    #[test]
    fn parses_tool_call_end() {
        let p = parse_line("[tool_call_end] turn=1 idx=1 name=Bash elapsed_ms=10 error=<none>")
            .unwrap();
        assert_eq!(p.event_type, "tool_call_end");
        assert_eq!(p.severity, "info");
    }

    #[test]
    fn parses_task_end_failed() {
        let p = parse_line("[task_end] status=failed").unwrap();
        assert_eq!(p.severity, "error");
        assert_eq!(p.payload["status"], "failed");
    }

    #[test]
    fn parses_execution_trace_json() {
        let line = serde_json::json!({
            "schema_version": anycode_core::EXECUTION_TRACE_SCHEMA_VERSION,
            "event_type": "tool_call_end",
            "severity": "info",
            "title": "Bash finished",
            "body": "",
            "payload": { "name": "Bash" },
            "occurred_at": "2026-05-23T00:00:00Z"
        })
        .to_string();
        let p = parse_line(&line).unwrap();
        assert_eq!(p.event_type, "tool_call_end");
        assert_eq!(p.title, "Bash finished");
        assert_eq!(p.payload["name"], "Bash");
    }

    #[test]
    fn parses_user_prompt_with_spaces() {
        let line = anycode_core::format_user_prompt_log_line("fix the bug in main.rs");
        let p = parse_line(&line).unwrap();
        assert_eq!(p.event_type, "user_prompt");
        assert_eq!(p.body, "fix the bug in main.rs");
    }

    #[test]
    fn parses_assistant_response_multiline() {
        let line = anycode_core::format_assistant_response_log_line(2, "line one\nline two");
        let p = parse_line(&line).unwrap();
        assert_eq!(p.event_type, "assistant_response");
        assert_eq!(p.body, "line one\nline two");
    }
}
