//! Shared agentic loop helpers (Template Method + Bridge for `execute_task` / `execute_turn`).

use super::agentic_loop::{nested_coop_cancelled, opt_coop_cancelled, task_cancelled_failure};
use anycode_core::prelude::*;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Per-turn context shared by task and TUI turn paths.
pub(super) struct TurnToolCtx<'a> {
    pub task_id: TaskId,
    pub agent_type: &'a AgentType,
    pub working_directory: &'a str,
    pub session_label: &'a str,
    pub turn: usize,
    pub loop_limits: AgentLoopLimits,
}

/// Mutable counters/state updated while dispatching tool calls in a turn.
pub(super) struct TurnToolState {
    pub total_tool_calls: usize,
    pub artifacts: Vec<anycode_core::Artifact>,
    pub budget_state: Option<super::budget::RuntimeBudgetState>,
}

pub(super) enum TurnToolCancel<'a> {
    /// Nested task cooperative cancel (`execute_task`).
    Nested(&'a anycode_core::TaskContext),
    /// TUI / channel cooperative cancel flag.
    Coop(Option<Arc<AtomicBool>>),
}

impl TurnToolCancel<'_> {
    pub(super) fn cancelled(&self) -> bool {
        match self {
            Self::Nested(ctx) => nested_coop_cancelled(ctx),
            Self::Coop(flag) => opt_coop_cancelled(flag),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TurnToolCancelOutcome {
    None,
    TaskCancelled,
    TurnCancelled,
}

impl TurnToolCancelOutcome {
    pub fn into_task_result(self) -> Option<anycode_core::TaskResult> {
        match self {
            Self::TaskCancelled => Some(task_cancelled_failure()),
            _ => None,
        }
    }

    pub fn into_core_error(self) -> Option<CoreError> {
        match self {
            Self::TurnCancelled => Some(CoreError::CooperativeCancel),
            _ => None,
        }
    }
}

/// Where tool_result messages are appended (Bridge: Vec vs shared mutex history).
pub(super) enum MessageAppendSink<'a> {
    Vec(&'a mut Vec<Message>),
    Shared(&'a Arc<Mutex<Vec<Message>>>),
}

impl MessageAppendSink<'_> {
    pub(super) async fn push(&mut self, message: Message) {
        match self {
            Self::Vec(v) => v.push(message),
            Self::Shared(m) => {
                let mut g = m.lock().await;
                g.push(message);
            }
        }
    }
}

/// Outcome after processing a batch of tool calls for one assistant turn.
pub(super) enum TurnToolBatchOutcome {
    Ok,
    Cancelled(TurnToolCancelOutcome),
    MaxToolCalls,
    BudgetExceeded,
}
