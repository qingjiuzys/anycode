//! Map persisted project events / log lines into [`ChatStreamEvent`] for session SSE.

use crate::observability::log_parser::ParsedLine;
use crate::schema::{ChatStreamEvent, ProjectEvent, TranscriptBlock};
use anycode_core::strip_llm_reasoning_for_display;
use anycode_dashboard_ipc::question_ipc::PendingQuestionRecord;
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
        "tool_approval_pending" => "approval_request",
        "tool_approval_resolved" => "approval_resolved",
        "ask_user_question_pending" => "question_request",
        "ask_user_question_resolved" => "question_resolved",
        "message_queued" => "message_queued",
        "message_dequeued" => "message_dequeued",
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
        "approval_request" => Some(TranscriptBlock {
            id: evt.id.clone(),
            block_type: "approval_request".into(),
            at: evt.occurred_at.clone(),
            title: evt.title.clone(),
            body: evt.body.clone(),
            meta: evt.payload.clone(),
            collapsible: true,
            default_collapsed: false,
            event_id: Some(evt.id.clone()),
        }),
        "approval_resolved" => {
            let mut meta = evt.payload.clone();
            if let Value::Object(ref mut map) = meta {
                map.insert("source".into(), json!("approval_resolved"));
                map.insert("severity".into(), json!("info"));
            }
            Some(TranscriptBlock {
                id: evt.id.clone(),
                block_type: "system_notice".into(),
                at: evt.occurred_at.clone(),
                title: evt.title.clone(),
                body: evt.body.clone(),
                meta,
                collapsible: true,
                default_collapsed: true,
                event_id: Some(evt.id.clone()),
            })
        }
        "question_request" => Some(TranscriptBlock {
            id: evt.id.clone(),
            block_type: "question_request".into(),
            at: evt.occurred_at.clone(),
            title: evt.title.clone(),
            body: evt.body.clone(),
            meta: evt.payload.clone(),
            collapsible: false,
            default_collapsed: false,
            event_id: Some(evt.id.clone()),
        }),
        "question_resolved" => {
            let mut meta = evt.payload.clone();
            if let Value::Object(ref mut map) = meta {
                map.insert("source".into(), json!("question_resolved"));
                map.insert("severity".into(), json!("info"));
            }
            Some(TranscriptBlock {
                id: evt.id.clone(),
                block_type: "system_notice".into(),
                at: evt.occurred_at.clone(),
                title: evt.title.clone(),
                body: evt.body.clone(),
                meta,
                collapsible: true,
                default_collapsed: true,
                event_id: Some(evt.id.clone()),
            })
        }
        "message_queued" => {
            let queue_id = evt
                .payload
                .get("queue_id")
                .and_then(|v| v.as_str())
                .unwrap_or(&evt.id);
            let mut meta = evt.payload.clone();
            if let Value::Object(ref mut map) = meta {
                map.insert("source".into(), json!("message_queue"));
                map.insert("status".into(), json!("pending"));
            }
            Some(TranscriptBlock {
                id: format!("queue:{queue_id}"),
                block_type: "user_message".into(),
                at: evt.occurred_at.clone(),
                title: evt.title.clone(),
                body: evt.body.clone(),
                meta,
                collapsible: false,
                default_collapsed: false,
                event_id: Some(evt.id.clone()),
            })
        }
        "message_dequeued" => None,
        _ => None,
    };

    Some(ChatStreamEvent {
        session_id,
        project_id: evt.project_id.clone(),
        kind: kind.into(),
        turn,
        conversation_turn_id: evt
            .payload
            .get("user_turn_id")
            .or_else(|| evt.payload.get("conversation_turn_id"))
            .and_then(|v| v.as_u64())
            .map(|n| n as u32),
        seq: None,
        event_id: None,
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
            false,
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
    assistant_raw_buffers: &mut std::collections::HashMap<u32, String>,
    assistant_display_buffers: &mut std::collections::HashMap<u32, String>,
) -> Option<ChatStreamEvent> {
    let at = Utc::now().to_rfc3339();
    match evt {
        anycode_core::LiveTraceEvent::AssistantDelta {
            turn,
            delta,
            narration,
        } => {
            let raw = assistant_raw_buffers.entry(*turn).or_default();
            raw.push_str(delta);
            let new_display = strip_llm_reasoning_for_display(raw);
            let prev_display = assistant_display_buffers
                .get(turn)
                .cloned()
                .unwrap_or_default();
            let display_delta = display_text_suffix_delta(&prev_display, &new_display);
            assistant_display_buffers.insert(*turn, new_display.clone());
            if display_delta.is_empty() && new_display.is_empty() {
                return None;
            }
            Some(assistant_delta_event(
                session_id,
                project_id,
                user_turn_id,
                *turn,
                &display_delta,
                &new_display,
                *narration,
            ))
        }
        anycode_core::LiveTraceEvent::AssistantNarrationMark { turn } => {
            let new_display = assistant_display_buffers
                .get(turn)
                .cloned()
                .unwrap_or_default();
            if new_display.is_empty() {
                return None;
            }
            Some(assistant_delta_event(
                session_id,
                project_id,
                user_turn_id,
                *turn,
                "",
                &new_display,
                true,
            ))
        }
        anycode_core::LiveTraceEvent::AssistantDone { turn, text } => {
            let display = strip_llm_reasoning_for_display(text);
            assistant_raw_buffers.insert(*turn, text.clone());
            assistant_display_buffers.insert(*turn, display.clone());
            Some(ChatStreamEvent {
                session_id: session_id.to_string(),
                project_id: project_id.to_string(),
                kind: "assistant_done".into(),
                turn: Some(*turn),
                conversation_turn_id: Some(user_turn_id),
                seq: None,
                event_id: None,
                tool_key: None,
                tool_name: None,
                text: Some(display.clone()),
                block: Some(TranscriptBlock {
                    id: live_assistant_block_id(user_turn_id, *turn),
                    block_type: "assistant_message".into(),
                    at: at.clone(),
                    title: format!("Assistant (turn {turn})"),
                    body: display,
                    meta: live_assistant_meta(user_turn_id, *turn, false, false),
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
                conversation_turn_id: Some(user_turn_id),
                seq: None,
                event_id: None,
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
                conversation_turn_id: Some(user_turn_id),
                seq: None,
                event_id: None,
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
            let mut end_meta = json!({
                "turn": turn.to_string(),
                "idx": idx.to_string(),
                "name": name,
                "elapsed_ms": elapsed_ms,
                "duration_ms": elapsed_ms.to_string(),
                "output_preview": output_preview,
                "user_turn_id": user_turn_id.to_string(),
            });
            if !failed {
                super::session_transcript::merge_activity_count_meta(&mut end_meta, name, &body);
            }
            Some(ChatStreamEvent {
                session_id: session_id.to_string(),
                project_id: project_id.to_string(),
                kind: "tool_result".into(),
                turn: Some(*turn),
                conversation_turn_id: Some(user_turn_id),
                seq: None,
                event_id: None,
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
                    meta: merge_tool_meta(&end_meta, Some(*turn), Some(&tool_key), "end"),
                    collapsible: true,
                    default_collapsed: true,
                    event_id: None,
                }),
                payload: json!({ "turn": turn, "idx": idx, "name": name, "elapsed_ms": elapsed_ms, "user_turn_id": user_turn_id }),
                at,
            })
        }
        anycode_core::LiveTraceEvent::ProgressUpdate {
            turn,
            seq,
            phase,
            work_stage,
            summary,
            next,
            discovery,
            evidence_refs,
        } => Some(progress_update_event(
            session_id,
            project_id,
            user_turn_id,
            *turn,
            *seq,
            phase,
            work_stage.as_ref(),
            summary,
            next.as_ref(),
            discovery.as_ref(),
            evidence_refs,
            true,
            &at,
        )),
        anycode_core::LiveTraceEvent::TurnDone { status } => Some(ChatStreamEvent {
            session_id: session_id.to_string(),
            project_id: project_id.to_string(),
            kind: "turn_done".into(),
            turn: None,
            conversation_turn_id: Some(user_turn_id),
            seq: None,
            event_id: None,
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
            conversation_turn_id: Some(user_turn_id),
            seq: None,
            event_id: None,
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

pub fn turn_phase_event(
    session_id: &str,
    project_id: &str,
    user_turn_id: u32,
    turn: u32,
    phase: &str,
) -> ChatStreamEvent {
    let at = Utc::now().to_rfc3339();
    ChatStreamEvent {
        session_id: session_id.to_string(),
        project_id: project_id.to_string(),
        kind: "turn_phase".into(),
        turn: Some(turn),
        conversation_turn_id: Some(user_turn_id),
        seq: None,
        event_id: None,
        tool_key: None,
        tool_name: None,
        text: None,
        block: Some(TranscriptBlock {
            id: format!("turn-phase:u{user_turn_id}:{turn}"),
            block_type: "system_notice".into(),
            at: at.clone(),
            title: "Turn phase".into(),
            body: String::new(),
            meta: json!({
                "source": "turn_phase",
                "phase": phase,
                "live": true,
                "turn": turn,
                "user_turn_id": user_turn_id.to_string(),
                "started_at": at,
            }),
            collapsible: false,
            default_collapsed: false,
            event_id: None,
        }),
        payload: json!({ "phase": phase, "turn": turn, "user_turn_id": user_turn_id }),
        at,
    }
}

pub fn question_resolved_event(
    session_id: &str,
    project_id: &str,
    user_turn_id: u32,
    question_id: &str,
) -> ChatStreamEvent {
    let at = Utc::now().to_rfc3339();
    ChatStreamEvent {
        session_id: session_id.to_string(),
        project_id: project_id.to_string(),
        kind: "question_resolved".into(),
        turn: None,
        conversation_turn_id: Some(user_turn_id),
        seq: None,
        event_id: None,
        tool_key: None,
        tool_name: None,
        text: None,
        block: Some(TranscriptBlock {
            id: format!("question-resolved:{question_id}"),
            block_type: "system_notice".into(),
            at: at.clone(),
            title: "Question answered".into(),
            body: String::new(),
            meta: json!({
                "source": "question_resolved",
                "question_id": question_id,
                "user_turn_id": user_turn_id,
            }),
            collapsible: true,
            default_collapsed: true,
            event_id: None,
        }),
        payload: json!({ "question_id": question_id, "user_turn_id": user_turn_id }),
        at,
    }
}

pub fn approval_request_event(
    session_id: &str,
    project_id: &str,
    user_turn_id: u32,
    rec: &anycode_dashboard_ipc::approval_ipc::PendingApprovalRecord,
) -> ChatStreamEvent {
    let at = Utc::now().to_rfc3339();
    let payload = json!({
        "approval_id": rec.approval_id,
        "session_id": rec.session_id,
        "tool": rec.tool,
        "input_preview": rec.input_preview,
        "user_turn_id": user_turn_id,
    });
    ChatStreamEvent {
        session_id: session_id.to_string(),
        project_id: project_id.to_string(),
        kind: "approval_request".into(),
        turn: None,
        conversation_turn_id: Some(user_turn_id),
        seq: None,
        event_id: None,
        tool_key: None,
        tool_name: Some(rec.tool.clone()),
        text: Some(rec.input_preview.clone()),
        block: Some(TranscriptBlock {
            id: format!("approval-live:{}", rec.approval_id),
            block_type: "approval_request".into(),
            at: at.clone(),
            title: format!("Approve {}", rec.tool),
            body: rec.input_preview.clone(),
            meta: payload.clone(),
            collapsible: false,
            default_collapsed: false,
            event_id: None,
        }),
        payload,
        at,
    }
}

pub fn approval_resolved_event(
    session_id: &str,
    project_id: &str,
    user_turn_id: u32,
    approval_id: &str,
    decision: &str,
) -> ChatStreamEvent {
    let at = Utc::now().to_rfc3339();
    ChatStreamEvent {
        session_id: session_id.to_string(),
        project_id: project_id.to_string(),
        kind: "approval_resolved".into(),
        turn: None,
        conversation_turn_id: Some(user_turn_id),
        seq: None,
        event_id: None,
        tool_key: None,
        tool_name: None,
        text: Some(decision.to_string()),
        block: Some(TranscriptBlock {
            id: format!("approval-resolved:{approval_id}"),
            block_type: "system_notice".into(),
            at: at.clone(),
            title: "Approval resolved".into(),
            body: decision.to_string(),
            meta: json!({
                "source": "approval_resolved",
                "approval_id": approval_id,
                "decision": decision,
                "user_turn_id": user_turn_id,
            }),
            collapsible: true,
            default_collapsed: true,
            event_id: None,
        }),
        payload: json!({ "approval_id": approval_id, "decision": decision, "user_turn_id": user_turn_id }),
        at,
    }
}

pub fn question_request_event(
    session_id: &str,
    project_id: &str,
    user_turn_id: u32,
    rec: &PendingQuestionRecord,
) -> ChatStreamEvent {
    let at = Utc::now().to_rfc3339();
    let id = format!("question-live:{}", rec.question_id);
    let payload = json!({
        "question_id": rec.question_id,
        "session_id": rec.session_id,
        "header": rec.header,
        "options": rec.options,
        "multi_select": rec.multi_select,
        "user_turn_id": user_turn_id,
    });
    ChatStreamEvent {
        session_id: session_id.to_string(),
        project_id: project_id.to_string(),
        kind: "question_request".into(),
        turn: None,
        conversation_turn_id: Some(user_turn_id),
        seq: None,
        event_id: None,
        tool_key: None,
        tool_name: Some("AskUserQuestion".into()),
        text: Some(rec.question.clone()),
        block: Some(TranscriptBlock {
            id,
            block_type: "question_request".into(),
            at: at.clone(),
            title: rec.header.clone(),
            body: rec.question.clone(),
            meta: payload.clone(),
            collapsible: false,
            default_collapsed: false,
            event_id: None,
        }),
        payload,
        at,
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
pub fn live_progress_block_id(user_turn_id: u32, seq: u32) -> String {
    format!("progress-live:u{user_turn_id}:{seq}")
}

fn normalize_evidence_refs(user_turn_id: u32, refs: &[String]) -> Vec<String> {
    refs.iter()
        .map(|r| {
            if r.starts_with("tool:") {
                r.clone()
            } else if r.contains(':') {
                format!("tool:u{user_turn_id}:{r}")
            } else {
                r.clone()
            }
        })
        .collect()
}

fn progress_update_event(
    session_id: &str,
    project_id: &str,
    user_turn_id: u32,
    turn: u32,
    seq: u32,
    phase: &str,
    work_stage: Option<&String>,
    summary: &str,
    next: Option<&String>,
    discovery: Option<&String>,
    evidence_refs: &[String],
    live: bool,
    at: &str,
) -> ChatStreamEvent {
    let refs = normalize_evidence_refs(user_turn_id, evidence_refs);
    let body = summary.trim().to_string();
    ChatStreamEvent {
        session_id: session_id.to_string(),
        project_id: project_id.to_string(),
        kind: "progress_update".into(),
        turn: Some(turn),
        conversation_turn_id: Some(user_turn_id),
        seq: Some(i64::from(seq)),
        event_id: None,
        tool_key: None,
        tool_name: None,
        text: Some(body.clone()),
        block: Some(TranscriptBlock {
            id: live_progress_block_id(user_turn_id, seq),
            block_type: "progress_update".into(),
            at: at.to_string(),
            title: phase.to_string(),
            body,
            meta: json!({
                "live": live,
                "turn": turn,
                "seq": seq,
                "phase": phase,
                "work_stage": work_stage,
                "summary": summary,
                "next": next,
                "discovery": discovery,
                "evidence_refs": refs,
                "user_turn_id": user_turn_id.to_string(),
            }),
            collapsible: true,
            default_collapsed: !live,
            event_id: None,
        }),
        payload: json!({
            "turn": turn,
            "seq": seq,
            "phase": phase,
            "user_turn_id": user_turn_id,
        }),
        at: at.to_string(),
    }
}

#[must_use]
pub fn live_assistant_block_id(user_turn_id: u32, turn: u32) -> String {
    format!("assistant-live:u{user_turn_id}:{turn}")
}

fn live_assistant_meta(
    user_turn_id: u32,
    turn: u32,
    live: bool,
    narration: bool,
) -> serde_json::Value {
    let mut meta = serde_json::json!({
        "live": live,
        "turn": turn,
        "user_turn_id": user_turn_id.to_string(),
        "source": if live { serde_json::Value::String("llm_start".into()) } else { serde_json::Value::Null },
    });
    if narration {
        meta["narration"] = serde_json::json!(true);
        meta["message_role"] = serde_json::json!("status");
    }
    meta
}

pub fn assistant_delta_event(
    session_id: &str,
    project_id: &str,
    user_turn_id: u32,
    turn: u32,
    display_delta: &str,
    display_full: &str,
    narration: bool,
) -> ChatStreamEvent {
    ChatStreamEvent {
        session_id: session_id.to_string(),
        project_id: project_id.to_string(),
        kind: "assistant_delta".into(),
        turn: Some(turn),
        conversation_turn_id: Some(user_turn_id),
        seq: None,
        event_id: None,
        tool_key: None,
        tool_name: None,
        text: Some(display_delta.to_string()),
        block: Some(TranscriptBlock {
            id: live_assistant_block_id(user_turn_id, turn),
            block_type: "assistant_message".into(),
            at: Utc::now().to_rfc3339(),
            title: format!("Assistant (turn {turn})"),
            body: display_full.to_string(),
            meta: live_assistant_meta(user_turn_id, turn, true, narration),
            collapsible: false,
            default_collapsed: false,
            event_id: None,
        }),
        payload: json!({
            "turn": turn,
            "delta": display_delta,
            "user_turn_id": user_turn_id
        }),
        at: Utc::now().to_rfc3339(),
    }
}

/// Incremental sanitized text: suffix of `new_display` after `prev_display`.
fn display_text_suffix_delta(prev_display: &str, new_display: &str) -> String {
    if new_display.starts_with(prev_display) {
        new_display[prev_display.len()..].to_string()
    } else if prev_display.is_empty() {
        new_display.to_string()
    } else {
        String::new()
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
        let mut raw = std::collections::HashMap::new();
        let mut display = std::collections::HashMap::new();
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
            &mut raw,
            &mut display,
        )
        .expect("mapped");
        assert_eq!(chat.kind, "tool_start");
        assert_eq!(chat.tool_key.as_deref(), Some("u3:2:1"));
    }

    #[test]
    fn live_trace_assistant_delta_strips_redacted_thinking() {
        let mut raw = std::collections::HashMap::new();
        let mut display = std::collections::HashMap::new();
        let payload = [
            "<redacted",
            "_thinking>secret</redacted",
            "_thinking>\nHello",
        ]
        .concat();
        let chat = chat_event_from_live_trace(
            "s1",
            "p1",
            1,
            &anycode_core::LiveTraceEvent::AssistantDelta {
                turn: 1,
                delta: payload,
                narration: false,
            },
            &mut raw,
            &mut display,
        )
        .expect("visible tail");
        assert_eq!(chat.text.as_deref().map(str::trim), Some("Hello"));
        assert!(!chat.block.as_ref().unwrap().body.contains("secret"));
    }

    #[test]
    fn live_trace_narration_mark_tags_assistant_block() {
        let mut raw = std::collections::HashMap::new();
        let mut display = std::collections::HashMap::new();
        raw.insert(2, "Now let me check".into());
        display.insert(2, "Now let me check".into());
        let chat = chat_event_from_live_trace(
            "s1",
            "p1",
            1,
            &anycode_core::LiveTraceEvent::AssistantNarrationMark { turn: 2 },
            &mut raw,
            &mut display,
        )
        .expect("narration mark");
        assert_eq!(chat.kind, "assistant_delta");
        let meta = chat.block.as_ref().unwrap().meta.clone();
        assert_eq!(meta.get("narration").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(
            meta.get("message_role").and_then(|v| v.as_str()),
            Some("status")
        );
    }

    #[test]
    fn live_trace_progress_update_maps_to_block() {
        let mut raw = std::collections::HashMap::new();
        let mut display = std::collections::HashMap::new();
        let chat = chat_event_from_live_trace(
            "s1",
            "p1",
            2,
            &anycode_core::LiveTraceEvent::ProgressUpdate {
                turn: 3,
                seq: 1,
                phase: "execute".into(),
                work_stage: Some("inspect".into()),
                summary: "Checking tests".into(),
                next: Some("Run grep".into()),
                discovery: None,
                evidence_refs: vec!["3:1".into()],
            },
            &mut raw,
            &mut display,
        )
        .expect("progress");
        assert_eq!(chat.kind, "progress_update");
        let block = chat.block.expect("block");
        assert_eq!(block.block_type, "progress_update");
        assert_eq!(
            block.meta.get("phase").and_then(|v| v.as_str()),
            Some("execute")
        );
    }

    #[test]
    fn turn_phase_event_carries_phase_meta() {
        let evt = turn_phase_event("s1", "p1", 2, 3, "waiting_first_token");
        assert_eq!(evt.kind, "turn_phase");
        let block = evt.block.expect("block");
        assert_eq!(
            block.meta.get("source").and_then(|v| v.as_str()),
            Some("turn_phase")
        );
        assert_eq!(
            block.meta.get("phase").and_then(|v| v.as_str()),
            Some("waiting_first_token")
        );
    }
}
