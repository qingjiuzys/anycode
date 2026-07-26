//! Shared tool_result sanitize/truncate/message build for execute_task and execute_turn.

use super::artifacts::truncate_text;
use super::limits::{TOOL_INPUT_LOG_MAX_BYTES, TOOL_RESULT_MAX_BYTES};
use super::live_trace_emit;
use super::logging::RunLogger;
use super::tool_output_sanitize;
use anycode_core::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

pub(super) struct PreparedToolResult {
    pub message: Message,
    pub for_hook: String,
}

pub(super) fn prepare_tool_result_message(
    task_id: TaskId,
    tool_call: &ToolCall,
    tool_result: &ToolOutput,
    logger: &RunLogger,
) -> PreparedToolResult {
    let tool_text = if let Some(err) = tool_result.error.clone() {
        format!("ERROR: {}\nRESULT: {}", err, tool_result.result)
    } else {
        format!("{}", tool_result.result)
    };
    let (tool_text, sanitize_report) = tool_output_sanitize::sanitize_tool_output(&tool_text);
    let (tool_text, truncated) = truncate_text(tool_text, TOOL_RESULT_MAX_BYTES);
    if truncated {
        logger.line(
            task_id,
            &format!(
                "[tool_result] truncated=true max_bytes={}",
                TOOL_RESULT_MAX_BYTES
            ),
        );
    }
    let for_hook = tool_text.clone();
    let mut metadata = HashMap::new();
    metadata.insert(
        "tool_name".to_string(),
        serde_json::Value::String(tool_call.name.clone()),
    );
    if sanitize_report.redacted_secret_patterns > 0 {
        metadata.insert(
            "sanitizer_redacted".to_string(),
            serde_json::json!(sanitize_report.redacted_secret_patterns),
        );
    }
    if sanitize_report.marked_prompt_injection {
        metadata.insert(
            "sanitizer_prompt_injection".to_string(),
            serde_json::Value::Bool(true),
        );
    }
    let message = Message {
        id: Uuid::new_v4(),
        role: MessageRole::Tool,
        content: MessageContent::ToolResult {
            tool_use_id: tool_call.id.clone(),
            content: tool_text,
            is_error: tool_result.error.is_some(),
        },
        timestamp: chrono::Utc::now(),
        metadata,
    };
    PreparedToolResult { message, for_hook }
}

pub(super) fn log_tool_call_input(
    logger: &RunLogger,
    live_trace_tx: &Option<tokio::sync::mpsc::UnboundedSender<LiveTraceEvent>>,
    task_id: TaskId,
    turn: usize,
    tool_idx: usize,
    tool_call: &ToolCall,
) -> String {
    let tool_input_json =
        serde_json::to_string(&tool_call.input).unwrap_or_else(|_| "<unserializable>".to_string());
    if let Some(path) = artifact_path_from_tool_input(&tool_call.input) {
        logger.line(task_id, &format!("artifact_path={path}"));
    }
    let (tool_input_json, truncated) = truncate_text(tool_input_json, TOOL_INPUT_LOG_MAX_BYTES);
    let preview = if truncated {
        format!("{tool_input_json}…")
    } else {
        tool_input_json.clone()
    };
    live_trace_emit::emit_tool_call_start(live_trace_tx, turn, tool_idx, tool_call, &preview);
    logger.line(
        task_id,
        &format!(
            "[tool_call_input] turn={} idx={} name={} truncated={}",
            turn, tool_idx, tool_call.name, truncated
        ),
    );
    logger.line(task_id, &tool_input_json);
    preview
}

pub(super) fn log_tool_call_start(
    logger: &RunLogger,
    live_trace_tx: &Option<tokio::sync::mpsc::UnboundedSender<LiveTraceEvent>>,
    task_id: TaskId,
    turn: usize,
    tool_idx: usize,
    tool_call: &ToolCall,
) {
    logger.line(
        task_id,
        &format!(
            "[tool_call_start] turn={} idx={} name={} command={}",
            turn,
            tool_idx,
            tool_call.name,
            shell_escape_kv(&preview_from_call(tool_call))
        ),
    );
    let _ = live_trace_tx;
}

fn preview_from_call(tool_call: &ToolCall) -> String {
    let json = serde_json::to_string(&tool_call.input).unwrap_or_default();
    // Keep enough of the input for trajectory evidence to distinguish calls —
    // 120 chars collapses `cd <workspace> && A` and `cd <workspace> && B` into
    // false "identical tool call" verdicts in the eval trajectory gate.
    let (preview, _) = truncate_text(json, 2048);
    preview
}

fn shell_escape_kv(value: &str) -> String {
    if value.is_empty() {
        return "<none>".to_string();
    }
    if value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "-_./".contains(c))
    {
        return value.to_string();
    }
    value.replace(' ', "_")
}

pub(super) fn log_tool_call_end(
    logger: &RunLogger,
    live_trace_tx: &Option<tokio::sync::mpsc::UnboundedSender<LiveTraceEvent>>,
    task_id: TaskId,
    turn: usize,
    tool_idx: usize,
    tool_call: &ToolCall,
    tool_result: &ToolOutput,
    elapsed_ms: u128,
    progress_seq: &mut u32,
) {
    live_trace_emit::emit_tool_call_end(
        live_trace_tx,
        turn,
        tool_idx,
        tool_call,
        elapsed_ms,
        tool_result,
    );
    if let Some(err) = tool_result.error.as_deref() {
        *progress_seq += 1;
        live_trace_emit::emit_progress_update(
            live_trace_tx,
            super::progress_update::build_discovery_from_failure(
                turn as u32,
                *progress_seq,
                &tool_call.name,
                err,
                0,
                turn as u32,
                tool_idx as u32,
            ),
        );
    }
    let preview = live_trace_emit::tool_output_preview_for_log(tool_result);
    let preview_hex: String = preview.bytes().map(|b| format!("{b:02x}")).collect();
    logger.line(
        task_id,
        &format!(
            "[tool_call_end] turn={} idx={} name={} elapsed_ms={} error={} output_preview_hex={}",
            turn,
            tool_idx,
            tool_call.name,
            elapsed_ms,
            tool_result
                .error
                .clone()
                .unwrap_or_else(|| "<none>".to_string()),
            preview_hex
        ),
    );
}

fn artifact_path_from_tool_input(input: &serde_json::Value) -> Option<String> {
    for key in ["file_path", "path", "notebook_path", "target_file"] {
        if let Some(p) = input.get(key).and_then(|x| x.as_str()) {
            if !p.is_empty() {
                return Some(p.to_string());
            }
        }
    }
    None
}
