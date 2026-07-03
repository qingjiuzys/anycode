//! Emit [`LiveTraceEvent`] before disk log (SSE-first path).

use anycode_core::prelude::*;
use tokio::sync::mpsc::UnboundedSender;

pub(crate) fn try_emit(tx: &Option<UnboundedSender<LiveTraceEvent>>, event: LiveTraceEvent) {
    if let Some(sender) = tx {
        let _ = sender.send(event);
    }
}

pub(crate) fn emit_turn_start(tx: &Option<UnboundedSender<LiveTraceEvent>>, turn: usize) {
    try_emit(tx, LiveTraceEvent::TurnStart { turn: turn as u32 });
}

pub(crate) fn emit_llm_request_start(tx: &Option<UnboundedSender<LiveTraceEvent>>, turn: usize) {
    try_emit(tx, LiveTraceEvent::LlmRequestStart { turn: turn as u32 });
}

pub(crate) fn emit_assistant_delta(
    tx: &Option<UnboundedSender<LiveTraceEvent>>,
    turn: usize,
    delta: &str,
) {
    if delta.is_empty() {
        return;
    }
    try_emit(
        tx,
        LiveTraceEvent::AssistantDelta {
            turn: turn as u32,
            delta: delta.to_string(),
        },
    );
}

pub(crate) fn emit_assistant_done(
    tx: &Option<UnboundedSender<LiveTraceEvent>>,
    turn: usize,
    text: &str,
) {
    let text = text.trim();
    if text.is_empty() {
        return;
    }
    try_emit(
        tx,
        LiveTraceEvent::AssistantDone {
            turn: turn as u32,
            text: text.to_string(),
        },
    );
}

pub(crate) fn emit_tool_call_start(
    tx: &Option<UnboundedSender<LiveTraceEvent>>,
    turn: usize,
    tool_idx: usize,
    tool_call: &ToolCall,
    input_preview: &str,
) {
    try_emit(
        tx,
        LiveTraceEvent::ToolCallStart {
            turn: turn as u32,
            idx: tool_idx as u32,
            name: tool_call.name.clone(),
            input_preview: input_preview.to_string(),
        },
    );
}

pub(crate) fn emit_tool_call_end(
    tx: &Option<UnboundedSender<LiveTraceEvent>>,
    turn: usize,
    tool_idx: usize,
    tool_call: &ToolCall,
    elapsed_ms: u128,
    tool_result: &ToolOutput,
) {
    let output_preview = tool_output_preview(tool_result);
    try_emit(
        tx,
        LiveTraceEvent::ToolCallEnd {
            turn: turn as u32,
            idx: tool_idx as u32,
            name: tool_call.name.clone(),
            elapsed_ms: elapsed_ms.min(u64::MAX as u128) as u64,
            error: tool_result.error.clone(),
            output_preview,
        },
    );
}

fn tool_output_preview(tool_result: &ToolOutput) -> String {
    const MAX: usize = 500;
    if let Some(err) = tool_result.error.as_ref() {
        return truncate_preview(err, MAX);
    }
    let text = match &tool_result.result {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    truncate_preview(&text, MAX)
}

fn truncate_preview(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    trimmed.chars().take(max_chars).collect::<String>() + "…"
}

pub(crate) fn tool_output_preview_for_log(tool_result: &ToolOutput) -> String {
    tool_output_preview(tool_result)
}

pub(crate) fn emit_turn_done(tx: &Option<UnboundedSender<LiveTraceEvent>>, status: &str) {
    try_emit(
        tx,
        LiveTraceEvent::TurnDone {
            status: status.to_string(),
        },
    );
}
