//! Structured live trace events for dashboard SSE (emit before disk log).

use serde::{Deserialize, Serialize};

/// Runtime → dashboard live trace (SSE-first; log is audit/replay).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LiveTraceEvent {
    TurnStart {
        turn: u32,
    },
    LlmRequestStart {
        turn: u32,
    },
    AssistantDelta {
        turn: u32,
        delta: String,
        /// When true, this turn's assistant text is intermediate tool-round narration.
        #[serde(default)]
        narration: bool,
    },
    /// Re-tag a turn's assistant block as tool-round narration (after tool_calls are known).
    AssistantNarrationMark {
        turn: u32,
    },
    /// User-facing progress card (intent / execute / discovery / deliver).
    ProgressUpdate {
        turn: u32,
        seq: u32,
        phase: String,
        work_stage: Option<String>,
        summary: String,
        next: Option<String>,
        discovery: Option<String>,
        evidence_refs: Vec<String>,
    },
    ToolCallStart {
        turn: u32,
        idx: u32,
        name: String,
        input_preview: String,
    },
    ToolCallEnd {
        turn: u32,
        idx: u32,
        name: String,
        elapsed_ms: u64,
        error: Option<String>,
        output_preview: String,
    },
    ToolCallProgress {
        turn: u32,
        idx: u32,
        name: String,
        elapsed_ms: u64,
    },
    AssistantDone {
        turn: u32,
        text: String,
    },
    TurnDone {
        status: String,
    },
}
