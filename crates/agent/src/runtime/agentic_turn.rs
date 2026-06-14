//! Shared agentic loop helpers (Template Method + Bridge for `execute_task` / `execute_turn`).

use super::agentic_loop::{nested_coop_cancelled, opt_coop_cancelled, task_cancelled_failure};
use super::artifacts::extract_artifacts;
use super::budget::{tick_budget, tool_blocked_under_degrade, RuntimeBudgetState};
use super::evidence;
use super::logging::RunLogger;
use super::tool_result_injection;
use super::AgentRuntime;
use anycode_core::prelude::*;
use anycode_core::Artifact;
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
    pub artifacts: Vec<Artifact>,
    pub budget_state: Option<RuntimeBudgetState>,
}

pub(super) enum TurnToolCancel<'a> {
    /// Nested task cooperative cancel (`execute_task`).
    Nested(&'a TaskContext),
    /// TUI / channel cooperative cancel flag.
    Coop(Option<Arc<AtomicBool>>),
}

impl TurnToolCancel<'_> {
    fn cancelled(&self) -> bool {
        match self {
            Self::Nested(ctx) => nested_coop_cancelled(ctx),
            Self::Coop(flag) => opt_coop_cancelled(flag),
        }
    }
}

pub(super) enum TurnToolCancelOutcome {
    None,
    TaskCancelled,
    TurnCancelled,
}

impl TurnToolCancelOutcome {
    pub fn into_task_result(self) -> Option<TaskResult> {
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
    async fn push(&mut self, message: Message) {
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

impl AgentRuntime {
    /// Dispatch tool calls for one model turn — shared by task and TUI paths.
    pub(super) async fn dispatch_turn_tool_calls(
        &self,
        logger: &RunLogger,
        ctx: &TurnToolCtx<'_>,
        state: &mut TurnToolState,
        cancel: &TurnToolCancel<'_>,
        sink: &mut MessageAppendSink<'_>,
        tool_calls: Vec<ToolCall>,
        record_evidence: bool,
        cancel_outcome: TurnToolCancelOutcome,
    ) -> Result<TurnToolBatchOutcome, CoreError> {
        for tool_call in tool_calls {
            if cancel.cancelled() {
                return Ok(TurnToolBatchOutcome::Cancelled(cancel_outcome));
            }
            if tick_budget(logger, ctx.task_id, &mut state.budget_state) {
                logger.line(
                    ctx.task_id,
                    "[task_end] status=failed reason=budget_exceeded",
                );
                return Ok(TurnToolBatchOutcome::BudgetExceeded);
            }
            state.total_tool_calls += 1;
            if state.total_tool_calls > ctx.loop_limits.max_tool_calls {
                logger.line(
                    ctx.task_id,
                    &format!(
                        "[task_end] status=failed reason=max_tool_calls({})",
                        ctx.loop_limits.max_tool_calls
                    ),
                );
                return Ok(TurnToolBatchOutcome::MaxToolCalls);
            }

            tool_result_injection::log_tool_call_input(
                logger,
                ctx.task_id,
                ctx.turn,
                state.total_tool_calls,
                &tool_call,
            );
            tool_result_injection::log_tool_call_start(
                logger,
                ctx.task_id,
                ctx.turn,
                state.total_tool_calls,
                &tool_call,
            );
            if state
                .budget_state
                .as_ref()
                .is_some_and(|s| tool_blocked_under_degrade(s, &tool_call.name))
            {
                logger.line(
                    ctx.task_id,
                    &format!(
                        "[tool_denied] name={} reason=budget_degrade",
                        tool_call.name
                    ),
                );
                let tool_result = ToolOutput {
                    result: serde_json::json!({ "error": "tool blocked under budget degradation" }),
                    error: Some("tool blocked under budget degradation".into()),
                    duration_ms: 0,
                };
                tool_result_injection::log_tool_call_end(
                    logger,
                    ctx.task_id,
                    ctx.turn,
                    state.total_tool_calls,
                    &tool_call,
                    &tool_result,
                    0,
                );
                let prepared = tool_result_injection::prepare_tool_result_message(
                    ctx.task_id,
                    &tool_call,
                    &tool_result,
                    logger,
                );
                sink.push(prepared.message).await;
                continue;
            }
            let t0 = std::time::Instant::now();
            let tool_result = self
                .execute_tool_call(
                    ctx.task_id,
                    ctx.agent_type,
                    ctx.working_directory,
                    &tool_call,
                )
                .await?;
            tool_result_injection::log_tool_call_end(
                logger,
                ctx.task_id,
                ctx.turn,
                state.total_tool_calls,
                &tool_call,
                &tool_result,
                t0.elapsed().as_millis(),
            );

            let prepared = tool_result_injection::prepare_tool_result_message(
                ctx.task_id,
                &tool_call,
                &tool_result,
                logger,
            );
            if record_evidence {
                evidence::append_tool_evidence(ctx.task_id, &tool_call.name, &prepared.for_hook);
            }
            sink.push(prepared.message).await;

            self.pipeline_memory_hook_tool_result(
                ctx.session_label,
                ctx.task_id,
                &tool_call.name,
                &prepared.for_hook,
            )
            .await;
            self.maybe_session_notify_tool_result(
                ctx.session_label,
                ctx.task_id,
                ctx.turn,
                &tool_call.name,
                &prepared.for_hook,
                Some(ctx.working_directory),
            );

            state
                .artifacts
                .extend(extract_artifacts(&tool_call, &tool_result));
            if cancel.cancelled() {
                return Ok(TurnToolBatchOutcome::Cancelled(cancel_outcome));
            }
        }
        Ok(TurnToolBatchOutcome::Ok)
    }
}
