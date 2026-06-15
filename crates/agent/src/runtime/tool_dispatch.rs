//! Tool dispatch: safe execution, cancel scopes, read-only batches, pairing invariant.

use super::agentic_turn::{
    MessageAppendSink, TurnToolBatchOutcome, TurnToolCancel, TurnToolCancelOutcome, TurnToolCtx,
    TurnToolState,
};
use super::artifacts::extract_artifacts;
use super::budget::{tick_budget, tool_blocked_under_degrade};
use super::evidence;
use super::logging::RunLogger;
use super::session_activity::{ActivityReason, SessionActivityGuard};
use super::tool_result_injection;
use super::AgentRuntime;
use anycode_core::prelude::*;
use futures::future::join_all;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const MAX_READONLY_TOOL_CONCURRENCY: usize = 10;

/// How a tool responds to cooperative cancel while running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ToolCancelPolicy {
    Cancel,
    Block,
}

pub(super) fn tool_cancel_policy(name: &str) -> ToolCancelPolicy {
    match name {
        "Bash" | "PowerShell" | "Task" | "Agent" => ToolCancelPolicy::Cancel,
        _ => ToolCancelPolicy::Block,
    }
}

pub(super) fn is_readonly_tool(name: &str) -> bool {
    matches!(
        name,
        "Glob" | "Grep" | "Read" | "FileRead" | "WebFetch" | "WebSearch" | "SemanticSearch"
    )
}

struct ToolBatch {
    concurrent: bool,
    calls: Vec<ToolCall>,
}

fn partition_tool_calls(calls: Vec<ToolCall>) -> Vec<ToolBatch> {
    let mut batches: Vec<ToolBatch> = Vec::new();
    for call in calls {
        let concurrent = is_readonly_tool(&call.name);
        if let Some(last) = batches.last_mut() {
            if last.concurrent && concurrent {
                last.calls.push(call);
                continue;
            }
        }
        batches.push(ToolBatch {
            concurrent,
            calls: vec![call],
        });
    }
    batches
}

impl AgentRuntime {
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
        let batches = partition_tool_calls(tool_calls);
        for batch in batches {
            if cancel.cancelled() {
                return Ok(TurnToolBatchOutcome::Cancelled(cancel_outcome));
            }
            if batch.concurrent && batch.calls.len() > 1 {
                let outcome = self
                    .dispatch_readonly_batch(
                        logger,
                        ctx,
                        state,
                        cancel,
                        sink,
                        batch.calls,
                        record_evidence,
                        cancel_outcome,
                    )
                    .await?;
                if !matches!(outcome, TurnToolBatchOutcome::Ok) {
                    return Ok(outcome);
                }
            } else {
                for tool_call in batch.calls {
                    let outcome = self
                        .dispatch_single_tool_call(
                            logger,
                            ctx,
                            state,
                            cancel,
                            sink,
                            tool_call,
                            record_evidence,
                            cancel_outcome,
                        )
                        .await?;
                    if !matches!(outcome, TurnToolBatchOutcome::Ok) {
                        return Ok(outcome);
                    }
                }
            }
        }
        Ok(TurnToolBatchOutcome::Ok)
    }

    async fn dispatch_readonly_batch(
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
        let mut planned: Vec<(ToolCall, usize)> = Vec::new();
        for tool_call in tool_calls {
            if cancel.cancelled() {
                self.emit_synthetic_for_planned(logger, ctx, sink, &planned, "cooperative_cancel")
                    .await;
                return Ok(TurnToolBatchOutcome::Cancelled(cancel_outcome));
            }
            let outcome = self
                .prepare_tool_dispatch(logger, ctx, state, cancel, cancel_outcome)
                .await?;
            if !matches!(outcome, TurnToolBatchOutcome::Ok) {
                return Ok(outcome);
            }
            if budget_degrade_blocks(state, &tool_call) {
                let idx = state.total_tool_calls;
                self.finalize_budget_denied(logger, ctx, state, sink, &tool_call, idx)
                    .await;
                continue;
            }
            planned.push((tool_call, state.total_tool_calls));
        }

        let chunk_size = MAX_READONLY_TOOL_CONCURRENCY.max(1);
        for chunk in planned.chunks(chunk_size) {
            let _activity =
                SessionActivityGuard::start(logger.clone(), ctx.task_id, ActivityReason::ToolExec);
            let futures = chunk
                .iter()
                .map(|(tool_call, tool_idx)| {
                    self.execute_tool_call_with_policy(logger, ctx, *tool_idx, tool_call, cancel)
                })
                .collect::<Vec<_>>();
            let results = join_all(futures).await;
            for ((tool_call, tool_idx), (tool_result, elapsed_ms)) in
                chunk.iter().zip(results.into_iter())
            {
                self.finalize_tool_call(
                    logger,
                    ctx,
                    state,
                    sink,
                    tool_call,
                    *tool_idx,
                    tool_result,
                    elapsed_ms,
                    record_evidence,
                )
                .await;
            }
            if cancel.cancelled() {
                return Ok(TurnToolBatchOutcome::Cancelled(cancel_outcome));
            }
        }
        Ok(TurnToolBatchOutcome::Ok)
    }

    async fn dispatch_single_tool_call(
        &self,
        logger: &RunLogger,
        ctx: &TurnToolCtx<'_>,
        state: &mut TurnToolState,
        cancel: &TurnToolCancel<'_>,
        sink: &mut MessageAppendSink<'_>,
        tool_call: ToolCall,
        record_evidence: bool,
        cancel_outcome: TurnToolCancelOutcome,
    ) -> Result<TurnToolBatchOutcome, CoreError> {
        if cancel.cancelled() {
            return Ok(TurnToolBatchOutcome::Cancelled(cancel_outcome));
        }
        let outcome = self
            .prepare_tool_dispatch(logger, ctx, state, cancel, cancel_outcome)
            .await?;
        if !matches!(outcome, TurnToolBatchOutcome::Ok) {
            return Ok(outcome);
        }
        let tool_idx = state.total_tool_calls;
        if budget_degrade_blocks(state, &tool_call) {
            self.finalize_budget_denied(logger, ctx, state, sink, &tool_call, tool_idx)
                .await;
            return Ok(TurnToolBatchOutcome::Ok);
        }
        let _activity =
            SessionActivityGuard::start(logger.clone(), ctx.task_id, ActivityReason::ToolExec);
        let (tool_result, elapsed_ms) = self
            .execute_tool_call_with_policy(logger, ctx, tool_idx, &tool_call, cancel)
            .await;
        self.finalize_tool_call(
            logger,
            ctx,
            state,
            sink,
            &tool_call,
            tool_idx,
            tool_result,
            elapsed_ms,
            record_evidence,
        )
        .await;
        if cancel.cancelled() {
            return Ok(TurnToolBatchOutcome::Cancelled(cancel_outcome));
        }
        Ok(TurnToolBatchOutcome::Ok)
    }

    async fn prepare_tool_dispatch(
        &self,
        logger: &RunLogger,
        ctx: &TurnToolCtx<'_>,
        state: &mut TurnToolState,
        cancel: &TurnToolCancel<'_>,
        cancel_outcome: TurnToolCancelOutcome,
    ) -> Result<TurnToolBatchOutcome, CoreError> {
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
        Ok(TurnToolBatchOutcome::Ok)
    }

    async fn execute_tool_call_with_policy(
        &self,
        logger: &RunLogger,
        ctx: &TurnToolCtx<'_>,
        tool_idx: usize,
        tool_call: &ToolCall,
        cancel: &TurnToolCancel<'_>,
    ) -> (ToolOutput, u128) {
        tool_result_injection::log_tool_call_input(
            logger,
            ctx.task_id,
            ctx.turn,
            tool_idx,
            tool_call,
        );
        tool_result_injection::log_tool_call_start(
            logger,
            ctx.task_id,
            ctx.turn,
            tool_idx,
            tool_call,
        );

        let policy = tool_cancel_policy(&tool_call.name);
        let t0 = std::time::Instant::now();

        let tool_result = if policy == ToolCancelPolicy::Cancel {
            tokio::select! {
                biased;
                _ = wait_cancel_flag(cancel.clone_flag()) => synthetic_tool_output("cooperative_cancel"),
                result = self.execute_tool_call(
                    ctx.task_id,
                    ctx.agent_type,
                    ctx.working_directory,
                    tool_call,
                ) => match result {
                    Ok(out) => out,
                    Err(e) => {
                        logger.turn_error(ctx.task_id, ctx.turn, &tool_call.name, &e.to_string());
                        synthetic_tool_error(&e)
                    }
                },
            }
        } else {
            match self
                .execute_tool_call(
                    ctx.task_id,
                    ctx.agent_type,
                    ctx.working_directory,
                    tool_call,
                )
                .await
            {
                Ok(out) => out,
                Err(e) => {
                    logger.turn_error(ctx.task_id, ctx.turn, &tool_call.name, &e.to_string());
                    synthetic_tool_error(&e)
                }
            }
        };

        (tool_result, t0.elapsed().as_millis())
    }

    async fn finalize_budget_denied(
        &self,
        logger: &RunLogger,
        ctx: &TurnToolCtx<'_>,
        state: &mut TurnToolState,
        sink: &mut MessageAppendSink<'_>,
        tool_call: &ToolCall,
        tool_idx: usize,
    ) {
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
        self.finalize_tool_call(
            logger,
            ctx,
            state,
            sink,
            tool_call,
            tool_idx,
            tool_result,
            0,
            false,
        )
        .await;
    }

    async fn finalize_tool_call(
        &self,
        logger: &RunLogger,
        ctx: &TurnToolCtx<'_>,
        state: &mut TurnToolState,
        sink: &mut MessageAppendSink<'_>,
        tool_call: &ToolCall,
        tool_idx: usize,
        tool_result: ToolOutput,
        elapsed_ms: u128,
        record_evidence: bool,
    ) {
        tool_result_injection::log_tool_call_end(
            logger,
            ctx.task_id,
            ctx.turn,
            tool_idx,
            tool_call,
            &tool_result,
            elapsed_ms,
        );
        let prepared = tool_result_injection::prepare_tool_result_message(
            ctx.task_id,
            tool_call,
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
            .extend(extract_artifacts(tool_call, &tool_result));
    }

    async fn emit_synthetic_for_planned(
        &self,
        logger: &RunLogger,
        ctx: &TurnToolCtx<'_>,
        sink: &mut MessageAppendSink<'_>,
        planned: &[(ToolCall, usize)],
        reason: &str,
    ) {
        for (tool_call, tool_idx) in planned {
            logger.tool_synthetic_result(ctx.task_id, ctx.turn, *tool_idx, &tool_call.name, reason);
            let output = synthetic_tool_output(reason);
            tool_result_injection::log_tool_call_end(
                logger,
                ctx.task_id,
                ctx.turn,
                *tool_idx,
                tool_call,
                &output,
                0,
            );
            let prepared = tool_result_injection::prepare_tool_result_message(
                ctx.task_id,
                tool_call,
                &output,
                logger,
            );
            sink.push(prepared.message).await;
        }
    }
}

impl TurnToolCancel<'_> {
    fn clone_flag(&self) -> Option<Arc<AtomicBool>> {
        match self {
            TurnToolCancel::Coop(flag) => flag.clone(),
            TurnToolCancel::Nested(_) => None,
        }
    }
}

async fn wait_cancel_flag(flag: Option<Arc<AtomicBool>>) {
    let Some(flag) = flag else {
        std::future::pending::<()>().await;
        return;
    };
    while !flag.load(Ordering::SeqCst) {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}

fn synthetic_tool_output(reason: &str) -> ToolOutput {
    ToolOutput {
        result: serde_json::json!({ "cancelled": true, "reason": reason }),
        error: Some(format!("cancelled: {reason}")),
        duration_ms: 0,
    }
}

fn synthetic_tool_error(err: &CoreError) -> ToolOutput {
    ToolOutput {
        result: serde_json::json!({ "error": err.to_string() }),
        error: Some(err.to_string()),
        duration_ms: 0,
    }
}

fn budget_degrade_blocks(state: &TurnToolState, tool_call: &ToolCall) -> bool {
    state
        .budget_state
        .as_ref()
        .is_some_and(|s| tool_blocked_under_degrade(s, &tool_call.name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partitions_consecutive_readonly_tools() {
        let calls = vec![
            ToolCall {
                id: "1".into(),
                name: "Glob".into(),
                input: serde_json::json!({}),
            },
            ToolCall {
                id: "2".into(),
                name: "Grep".into(),
                input: serde_json::json!({}),
            },
            ToolCall {
                id: "3".into(),
                name: "Bash".into(),
                input: serde_json::json!({}),
            },
        ];
        let batches = partition_tool_calls(calls);
        assert_eq!(batches.len(), 2);
        assert!(batches[0].concurrent);
        assert_eq!(batches[0].calls.len(), 2);
        assert!(!batches[1].concurrent);
    }

    #[test]
    fn bash_tools_are_cancellable() {
        assert_eq!(tool_cancel_policy("Bash"), ToolCancelPolicy::Cancel);
        assert_eq!(tool_cancel_policy("Edit"), ToolCancelPolicy::Block);
    }
}
