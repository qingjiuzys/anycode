//! Shared tool_result sanitize/truncate/message build for execute_task and execute_turn.

use super::artifacts::truncate_text;
use super::limits::{TOOL_INPUT_LOG_MAX_BYTES, TOOL_RESULT_MAX_BYTES};
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
    task_id: TaskId,
    turn: usize,
    tool_idx: usize,
    tool_call: &ToolCall,
) {
    let tool_input_json =
        serde_json::to_string(&tool_call.input).unwrap_or_else(|_| "<unserializable>".to_string());
    let (tool_input_json, truncated) = truncate_text(tool_input_json, TOOL_INPUT_LOG_MAX_BYTES);
    logger.line(
        task_id,
        &format!(
            "[tool_call_input] turn={} idx={} name={} truncated={}",
            turn, tool_idx, tool_call.name, truncated
        ),
    );
    logger.line(task_id, &tool_input_json);
}

pub(super) fn log_tool_call_start(
    logger: &RunLogger,
    task_id: TaskId,
    turn: usize,
    tool_idx: usize,
    tool_call: &ToolCall,
) {
    logger.line(
        task_id,
        &format!(
            "[tool_call_start] turn={} idx={} name={}",
            turn, tool_idx, tool_call.name
        ),
    );
}

pub(super) fn log_tool_call_end(
    logger: &RunLogger,
    task_id: TaskId,
    turn: usize,
    tool_idx: usize,
    tool_call: &ToolCall,
    tool_result: &ToolOutput,
    elapsed_ms: u128,
) {
    logger.line(
        task_id,
        &format!(
            "[tool_call_end] turn={} idx={} name={} elapsed_ms={} error={}",
            turn,
            tool_idx,
            tool_call.name,
            elapsed_ms,
            tool_result
                .error
                .clone()
                .unwrap_or_else(|| "<none>".to_string())
        ),
    );
}
