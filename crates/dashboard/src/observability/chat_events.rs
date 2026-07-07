//! Map persisted project events / log lines into [`ChatStreamEvent`] for session SSE.

use crate::observability::log_parser::ParsedLine;
use crate::schema::{ChatStreamEvent, ProjectEvent, TranscriptBlock};
use chrono::Utc;
use serde_json::{json, Value};

fn turn_from_payload(payload: &Value) -> Option<u32> {
    payload
        .get("turn")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok())
        .or_else(|| {
            payload
                .get("turn")
                .and_then(|v| v.as_u64())
                .map(|n| n as u32)
        })
}

fn tool_key_from_payload(payload: &Value, turn: Option<u32>) -> Option<String> {
    if let Some(k) = payload.get("tool_key").and_then(|v| v.as_str()) {
        if !k.is_empty() {
            return Some(k.to_string());
        }
    }
    let idx = payload.get("idx").and_then(|v| v.as_str())?;
    let turn = turn?;
    Some(format!("{turn}:{idx}"))
}

pub fn chat_event_from_project_event(evt: &ProjectEvent) -> Option<ChatStreamEvent> {
    let session_id = evt.session_id.as_deref()?.to_string();
    let turn = turn_from_payload(&evt.payload);
    let tool_key = tool_key_from_payload(&evt.payload, turn);
    let tool_name = evt
        .payload
        .get("name")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let kind = match evt.event_type.as_str() {
        "user_prompt" => "user_message",
        "tool_call_start" => "tool_start",
        "tool_call_end" => "tool_result",
        "assistant_response" => "assistant_done",
        "task_end" | "session_completed" => "turn_done",
        "session_error" | "session_blocked" | "session_cancelled" | "tool_denied" => {
            "session_error"
        }
        _ => return None,
    };

    let text = if evt.body.is_empty() {
        None
    } else {
        Some(evt.body.clone())
    };

    let block = match kind {
        "user_message" => Some(TranscriptBlock {
            id: evt.id.clone(),
            block_type: "user_message".into(),
            at: evt.occurred_at.clone(),
            title: evt.title.clone(),
            body: evt.body.clone(),
            meta: evt.payload.clone(),
            collapsible: false,
            default_collapsed: false,
            event_id: Some(evt.id.clone()),
        }),
        "assistant_done" => Some(TranscriptBlock {
            id: evt.id.clone(),
            block_type: "assistant_message".into(),
            at: evt.occurred_at.clone(),
            title: evt.title.clone(),
            body: evt.body.clone(),
            meta: evt.payload.clone(),
            collapsible: false,
            default_collapsed: false,
            event_id: Some(evt.id.clone()),
        }),
        "tool_start" => Some(TranscriptBlock {
            id: evt.id.clone(),
            block_type: "tool_call".into(),
            at: evt.occurred_at.clone(),
            title: evt.title.clone(),
            body: evt.body.clone(),
            meta: merge_tool_meta(&evt.payload, turn, tool_key.as_deref(), "start"),
            collapsible: true,
            default_collapsed: true,
            event_id: Some(evt.id.clone()),
        }),
        "tool_result" => Some(TranscriptBlock {
            id: evt.id.clone(),
            block_type: "tool_result".into(),
            at: evt.occurred_at.clone(),
            title: evt.title.clone(),
            body: evt.body.clone(),
            meta: merge_tool_meta(&evt.payload, turn, tool_key.as_deref(), "end"),
            collapsible: true,
            default_collapsed: true,
            event_id: Some(evt.id.clone()),
        }),
        _ => None,
    };

    Some(ChatStreamEvent {
        session_id,
        project_id: evt.project_id.clone(),
        kind: kind.into(),
        turn,
        tool_key,
        tool_name,
        text,
        block,
        payload: evt.payload.clone(),
        at: evt.occurred_at.clone(),
    })
}

pub fn chat_event_from_parsed_line(
    session_id: &str,
    project_id: &str,
    parsed: &ParsedLine,
) -> Option<ChatStreamEvent> {
    if parsed.event_type == "assistant_response" {
        let turn = turn_from_payload(&parsed.payload).unwrap_or(1);
        if parsed.body.is_empty() {
            return None;
        }
        return Some(assistant_delta_event(
            session_id,
            project_id,
            0,
            turn,
            &parsed.body,
            &parsed.body,
        ));
    }

    let stable_id = stable_parsed_block_id(parsed);
    let evt = ProjectEvent {
        id: stable_id.clone(),
        project_id: project_id.to_string(),
        session_id: Some(session_id.to_string()),
        task_id: None,
        agent_id: None,
        event_type: parsed.event_type.clone(),
        severity: parsed.severity.clone(),
        title: parsed.title.clone(),
        body: parsed.body.clone(),
        payload: parsed.payload.clone(),
        occurred_at: Utc::now().to_rfc3339(),
    };
    let mut chat = chat_event_from_project_event(&evt)?;
    if let Some(block) = chat.block.as_mut() {
        block.id = stable_id;
        block.meta = merge_tool_meta(
            &block.meta,
            chat.turn,
            chat.tool_key.as_deref(),
            if chat.kind == "tool_result" {
                "end"
            } else {
                "start"
            },
        );
    }
    Some(chat)
}

/// Map in-process [`LiveTraceEvent`] → session `chat_event` SSE payload.
pub fn chat_event_from_live_trace(
    session_id: &str,
    project_id: &str,
    user_turn_id: u32,
    evt: &anycode_core::LiveTraceEvent,
    assistant_buffers: &mut std::collections::HashMap<u32, String>,
) -> Option<ChatStreamEvent> {
    let at = Utc::now().to_rfc3339();
    match evt {
        anycode_core::LiveTraceEvent::AssistantDelta { turn, delta } => {
            let full = assistant_buffers.entry(*turn).or_default();
            full.push_str(delta);
            Some(assistant_delta_event(
                session_id,
                project_id,
                user_turn_id,
                *turn,
                delta,
                full,
            ))
        }
        anycode_core::LiveTraceEvent::AssistantDone { turn, text } => {
            assistant_buffers.insert(*turn, text.clone());
            Some(ChatStreamEvent {
                session_id: session_id.to_string(),
                project_id: project_id.to_string(),
                kind: "assistant_done".into(),
                turn: Some(*turn),
                tool_key: None,
                tool_name: None,
                text: Some(text.clone()),
                block: Some(TranscriptBlock {
                    id: live_assistant_block_id(user_turn_id, *turn),
                    block_type: "assistant_message".into(),
                    at: at.clone(),
                    title: format!("Assistant (turn {turn})"),
                    body: text.clone(),
                    meta: live_assistant_meta(user_turn_id, *turn, false),
                    collapsible: false,
                    default_collapsed: false,
                    event_id: None,
                }),
                payload: json!({ "turn": turn, "user_turn_id": user_turn_id }),
                at,
            })
        }
        anycode_core::LiveTraceEvent::ToolCallStart {
            turn,
            idx,
            name,
            input_preview,
        } => {
            let tool_key = live_tool_key(user_turn_id, *turn, *idx);
            let id = live_tool_block_id(user_turn_id, *turn, *idx, "call");
            Some(ChatStreamEvent {
                session_id: session_id.to_string(),
                project_id: project_id.to_string(),
                kind: "tool_start".into(),
                turn: Some(*turn),
                tool_key: Some(tool_key.clone()),
                tool_name: Some(name.clone()),
                text: Some(input_preview.clone()),
                block: Some(TranscriptBlock {
                    id,
                    block_type: "tool_call".into(),
                    at: at.clone(),
                    title: format!("{name} started"),
                    body: input_preview.clone(),
                    meta: merge_tool_meta(
                        &json!({
                            "turn": turn.to_string(),
                            "idx": idx.to_string(),
                            "name": name,
                            "user_turn_id": user_turn_id.to_string(),
                        }),
                        Some(*turn),
                        Some(&tool_key),
                        "start",
                    ),
                    collapsible: true,
                    default_collapsed: true,
                    event_id: None,
                }),
                payload: json!({ "turn": turn, "idx": idx, "name": name, "user_turn_id": user_turn_id }),
                at,
            })
        }
        anycode_core::LiveTraceEvent::ToolCallProgress {
            turn,
            idx,
            name,
            elapsed_ms,
        } => {
            let tool_key = live_tool_key(user_turn_id, *turn, *idx);
            let id = live_tool_block_id(user_turn_id, *turn, *idx, "call");
            Some(ChatStreamEvent {
                session_id: session_id.to_string(),
                project_id: project_id.to_string(),
                kind: "tool_progress".into(),
                turn: Some(*turn),
                tool_key: Some(tool_key.clone()),
                tool_name: Some(name.clone()),
                text: None,
                block: Some(TranscriptBlock {
                    id,
                    block_type: "tool_call".into(),
                    at: at.clone(),
                    title: format!("{name} started"),
                    body: String::new(),
                    meta: merge_tool_meta(
                        &json!({
                            "turn": turn.to_string(),
                            "idx": idx.to_string(),
                            "name": name,
                            "elapsed_ms": elapsed_ms,
                            "duration_ms": elapsed_ms.to_string(),
                            "user_turn_id": user_turn_id.to_string(),
                        }),
                        Some(*turn),
                        Some(&tool_key),
                        "running",
                    ),
                    collapsible: true,
                    default_collapsed: true,
                    event_id: None,
                }),
                payload: json!({
                    "turn": turn,
                    "idx": idx,
                    "name": name,
                    "elapsed_ms": elapsed_ms,
                    "user_turn_id": user_turn_id,
                }),
                at,
            })
        }
        anycode_core::LiveTraceEvent::ToolCallEnd {
            turn,
            idx,
            name,
            elapsed_ms,
            error,
            output_preview,
        } => {
            let tool_key = live_tool_key(user_turn_id, *turn, *idx);
            let id = live_tool_block_id(user_turn_id, *turn, *idx, "result");
            let failed = error.is_some();
            let body = if failed {
                error.clone().unwrap_or_default()
            } else {
                output_preview.clone()
            };
            Some(ChatStreamEvent {
                session_id: session_id.to_string(),
                project_id: project_id.to_string(),
                kind: "tool_result".into(),
                turn: Some(*turn),
                tool_key: Some(tool_key.clone()),
                tool_name: Some(name.clone()),
                text: if body.is_empty() {
                    None
                } else {
                    Some(body.clone())
                },
                block: Some(TranscriptBlock {
                    id,
                    block_type: "tool_result".into(),
                    at: at.clone(),
                    title: format!("{name} {}", if failed { "failed" } else { "finished" }),
                    body,
                    meta: merge_tool_meta(
                        &json!({
                            "turn": turn.to_string(),
                            "idx": idx.to_string(),
                            "name": name,
                            "elapsed_ms": elapsed_ms,
                            "duration_ms": elapsed_ms.to_string(),
                            "output_preview": output_preview,
                            "user_turn_id": user_turn_id.to_string(),
                        }),
                        Some(*turn),
                        Some(&tool_key),
                        "end",
                    ),
                    collapsible: true,
                    default_collapsed: true,
                    event_id: None,
                }),
                payload: json!({ "turn": turn, "idx": idx, "name": name, "elapsed_ms": elapsed_ms, "user_turn_id": user_turn_id }),
                at,
            })
        }
        anycode_core::LiveTraceEvent::TurnDone { status } => Some(ChatStreamEvent {
            session_id: session_id.to_string(),
            project_id: project_id.to_string(),
            kind: "turn_done".into(),
            turn: None,
            tool_key: None,
            tool_name: None,
            text: Some(status.clone()),
            block: None,
            payload: json!({ "status": status, "user_turn_id": user_turn_id }),
            at,
        }),
        anycode_core::LiveTraceEvent::TurnStart { .. } => None,
        anycode_core::LiveTraceEvent::LlmRequestStart { turn } => Some(ChatStreamEvent {
            session_id: session_id.to_string(),
            project_id: project_id.to_string(),
            kind: "llm_start".into(),
            turn: Some(*turn),
            tool_key: None,
            tool_name: None,
            text: None,
            block: Some(TranscriptBlock {
                id: format!("llm-start:u{user_turn_id}:{turn}"),
                block_type: "system_notice".into(),
                at: at.clone(),
                title: "Thinking".into(),
                body: String::new(),
                meta: json!({
                    "source": "llm_start",
                    "live": true,
                    "turn": turn,
                    "user_turn_id": user_turn_id.to_string(),
                }),
                collapsible: false,
                default_collapsed: false,
                event_id: None,
            }),
            payload: json!({ "turn": turn, "user_turn_id": user_turn_id }),
            at,
        }),
    }
}

#[must_use]
pub fn live_tool_key(user_turn_id: u32, turn: u32, idx: u32) -> String {
    format!("u{user_turn_id}:{turn}:{idx}")
}

#[must_use]
pub fn live_tool_block_id(user_turn_id: u32, turn: u32, idx: u32, phase: &str) -> String {
    format!("tool-live:u{user_turn_id}:{turn}:{idx}:{phase}")
}

#[must_use]
pub fn live_assistant_block_id(user_turn_id: u32, turn: u32) -> String {
    format!("assistant-live:u{user_turn_id}:{turn}")
}

fn live_assistant_meta(user_turn_id: u32, turn: u32, live: bool) -> serde_json::Value {
    json!({
        "live": live,
        "turn": turn,
        "user_turn_id": user_turn_id.to_string(),
        "source": if live { serde_json::Value::String("llm_start".into()) } else { serde_json::Value::Null },
    })
}

pub fn assistant_delta_event(
    session_id: &str,
    project_id: &str,
    user_turn_id: u32,
    turn: u32,
    delta: &str,
    full_text: &str,
) -> ChatStreamEvent {
    ChatStreamEvent {
        session_id: session_id.to_string(),
        project_id: project_id.to_string(),
        kind: "assistant_delta".into(),
        turn: Some(turn),
        tool_key: None,
        tool_name: None,
        text: Some(delta.to_string()),
        block: Some(TranscriptBlock {
            id: live_assistant_block_id(user_turn_id, turn),
            block_type: "assistant_message".into(),
            at: Utc::now().to_rfc3339(),
            title: format!("Assistant (turn {turn})"),
            body: full_text.to_string(),
            meta: live_assistant_meta(user_turn_id, turn, true),
            collapsible: false,
            default_collapsed: false,
            event_id: None,
        }),
        payload: json!({ "turn": turn, "delta": delta, "user_turn_id": user_turn_id }),
        at: Utc::now().to_rfc3339(),
    }
}

fn merge_tool_meta(
    payload: &Value,
    turn: Option<u32>,
    tool_key: Option<&str>,
    phase: &str,
) -> Value {
    let mut meta = payload.clone();
    if let Some(t) = turn {
        meta["turn"] = json!(t.to_string());
    }
    if let Some(k) = tool_key {
        meta["tool_key"] = json!(k);
    }
    meta["phase"] = json!(phase);
    meta
}

fn stable_parsed_block_id(parsed: &ParsedLine) -> String {
    match parsed.event_type.as_str() {
        "tool_call_start" | "tool_call_end" => {
            let turn = parsed
                .payload
                .get("turn")
                .and_then(|v| v.as_str())
                .unwrap_or("0");
            let idx = parsed
                .payload
                .get("idx")
                .and_then(|v| v.as_str())
                .unwrap_or("0");
            let phase = if parsed.event_type == "tool_call_end" {
                "result"
            } else {
                "call"
            };
            format!("tool-live:{turn}:{idx}:{phase}")
        }
        "assistant_response" => {
            let turn = parsed
                .payload
                .get("turn")
                .and_then(|v| v.as_str())
                .unwrap_or("1");
            format!("assistant-live:{turn}")
        }
        _ => uuid::Uuid::new_v4().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observability::log_parser::ParsedLine;

    #[test]
    fn maps_tool_start_from_project_event() {
        let evt = ProjectEvent {
            id: "e1".into(),
            project_id: "p1".into(),
            session_id: Some("s1".into()),
            task_id: None,
            agent_id: None,
            event_type: "tool_call_start".into(),
            severity: "info".into(),
            title: "Bash started".into(),
            body: "ls".into(),
            payload: json!({ "turn": "3", "idx": "1", "name": "Bash" }),
            occurred_at: "2026-01-01T00:00:00Z".into(),
        };
        let chat = chat_event_from_project_event(&evt).expect("mapped");
        assert_eq!(chat.kind, "tool_start");
        assert_eq!(chat.tool_key.as_deref(), Some("3:1"));
    }

    #[test]
    fn parsed_line_uses_stable_tool_ids() {
        let parsed = ParsedLine {
            event_type: "tool_call_start".into(),
            severity: "info".into(),
            title: "Bash started".into(),
            body: String::new(),
            payload: json!({ "turn": "2", "idx": "1", "name": "Bash" }),
        };
        let chat = chat_event_from_parsed_line("s1", "p1", &parsed).expect("mapped");
        assert_eq!(chat.kind, "tool_start");
        assert_eq!(
            chat.block.as_ref().map(|b| b.id.as_str()),
            Some("tool-live:2:1:call")
        );
    }

    #[test]
    fn live_trace_maps_tool_start() {
        let mut buffers = std::collections::HashMap::new();
        let chat = chat_event_from_live_trace(
            "s1",
            "p1",
            3,
            &anycode_core::LiveTraceEvent::ToolCallStart {
                turn: 2,
                idx: 1,
                name: "Bash".into(),
                input_preview: "ls".into(),
            },
            &mut buffers,
        )
        .expect("mapped");
        assert_eq!(chat.kind, "tool_start");
        assert_eq!(chat.tool_key.as_deref(), Some("u3:2:1"));
    }
}
