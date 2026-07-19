//! Shared turn finalization: summary receipt + SSE turn_done for incomplete exits.

use super::live_trace_emit;
use super::receipt::ReceiptGenerator;
use super::task_summary::llm_summary_receipt;
use super::AgentRuntime;
use anycode_core::prelude::*;
use anycode_core::Artifact;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;
use uuid::Uuid;

pub(super) struct TurnFinalizeParams<'a> {
    pub task_id: TaskId,
    pub agent_type: &'a AgentType,
    pub messages: &'a Arc<Mutex<Vec<Message>>>,
    pub working_directory: &'a str,
    pub live_trace_tx: &'a Option<UnboundedSender<LiveTraceEvent>>,
    pub loop_limits: AgentLoopLimits,
    pub last_model_turn: usize,
    pub total_tool_calls: usize,
    pub artifacts: Vec<Artifact>,
    pub turn_usage: TurnTokenUsage,
    pub termination_reason: TerminationReason,
}

#[must_use]
pub(super) fn turn_done_status(reason: TerminationReason) -> &'static str {
    match reason {
        TerminationReason::Completed => "completed",
        TerminationReason::MaxTurns => "max_turns",
        TerminationReason::MaxTools => "max_tools",
        TerminationReason::Budget => "budget",
        TerminationReason::RefusalNoTool => "refusal_no_tool",
        TerminationReason::Cancelled => "cancelled",
        TerminationReason::Partial => "partial",
        TerminationReason::Error => "error",
    }
}

#[must_use]
pub(super) fn task_end_line(reason: TerminationReason) -> String {
    if reason == TerminationReason::Completed {
        "[task_end] status=completed reason=completed".to_string()
    } else {
        format!("[task_end] status=failed reason={}", reason.as_str())
    }
}

impl AgentRuntime {
    pub(super) async fn finalize_incomplete_turn(
        &self,
        params: TurnFinalizeParams<'_>,
    ) -> TurnOutput {
        let logger = self.logger();
        let user_line = {
            let g = params.messages.lock().await;
            super::memory_hooks::last_user_plain_text_for_autosave(&g)
        };

        let output_tail = logger.tail(params.task_id, 24 * 1024);
        let artifacts_brief = ReceiptGenerator::artifacts_brief(&params.artifacts);
        let summary_model = self.model_for_summary().clone();
        let summary_task = Task {
            id: params.task_id,
            agent_type: params.agent_type.clone(),
            prompt: user_line.clone(),
            context: TaskContext {
                session_id: Uuid::new_v4(),
                working_directory: params.working_directory.to_string(),
                environment: HashMap::new(),
                user_id: None,
                system_prompt_append: None,
                context_injections: vec![],
                nested_model_override: None,
                nested_worktree_path: None,
                nested_worktree_repo_root: None,
                nested_cancel: None,
                channel_progress_tx: None,
                live_trace_tx: None,
                tool_deny_names: vec![],
                tool_deny_prefixes: vec![],
                user_vision_images: vec![],
                budget: TaskBudget::default(),
                loop_limits: params.loop_limits,
                chat_turn: None,
            },
            created_at: chrono::Utc::now(),
        };

        let summary_text = llm_summary_receipt(
            &self.llm_client,
            &summary_model,
            &summary_task,
            params.total_tool_calls,
            params.loop_limits.max_agent_turns,
            params.loop_limits.max_tool_calls,
            &artifacts_brief,
            &output_tail,
            params.termination_reason,
        )
        .await;

        let turn_done_status = turn_done_status(params.termination_reason);
        let task_end_line = task_end_line(params.termination_reason);

        logger.session_state(params.task_id, "idle");
        live_trace_emit::emit_assistant_done(
            params.live_trace_tx,
            params.last_model_turn,
            &summary_text,
        );
        live_trace_emit::emit_turn_done(params.live_trace_tx, turn_done_status);
        logger.line(params.task_id, &task_end_line);
        logger.assistant_response(params.task_id, params.last_model_turn, &summary_text);
        logger.line(params.task_id, "== summary ==");
        for line in summary_text.lines() {
            logger.line(params.task_id, line);
        }

        self.maybe_autosave_memory(params.task_id, &user_line, &summary_text)
            .await;

        if !summary_text.trim().is_empty() {
            let mut g = params.messages.lock().await;
            g.push(Message {
                id: Uuid::new_v4(),
                role: MessageRole::Assistant,
                content: MessageContent::Text(summary_text.clone()),
                timestamp: chrono::Utc::now(),
                metadata: HashMap::new(),
            });
        }

        let session_label = format!("tui_{}", params.task_id);
        self.pipeline_memory_hook_agent_turn(
            &session_label,
            params.task_id,
            params.last_model_turn,
            &summary_text,
        )
        .await;
        self.maybe_session_notify_agent_turn(
            &session_label,
            params.task_id,
            params.last_model_turn,
            &summary_text,
            Some(params.working_directory),
        );

        TurnOutput {
            final_text: summary_text,
            artifacts: params.artifacts,
            usage: params.turn_usage,
            termination_reason: params.termination_reason,
        }
    }

    /// Emit cancelled receipt when cooperative cancel happens mid-turn (feed still gets a summary).
    pub(super) async fn emit_cancelled_turn_receipt(&self, params: TurnFinalizeParams<'_>) {
        if params.total_tool_calls == 0 && params.artifacts.is_empty() {
            live_trace_emit::emit_turn_done(params.live_trace_tx, "cancelled");
            self.logger().line(
                params.task_id,
                "[task_end] status=cancelled reason=cancelled",
            );
            return;
        }
        let _ = self.finalize_incomplete_turn(params).await;
    }
}

#[cfg(test)]
mod tests {
    use super::{task_end_line, turn_done_status};
    use anycode_core::TerminationReason;

    #[test]
    fn turn_done_status_maps_all_reasons() {
        assert_eq!(turn_done_status(TerminationReason::MaxTurns), "max_turns");
        assert_eq!(turn_done_status(TerminationReason::MaxTools), "max_tools");
        assert_eq!(turn_done_status(TerminationReason::Budget), "budget");
        assert_eq!(turn_done_status(TerminationReason::Cancelled), "cancelled");
    }

    #[test]
    fn task_end_line_marks_incomplete_as_failed() {
        assert!(task_end_line(TerminationReason::MaxTools).contains("failed"));
        assert!(task_end_line(TerminationReason::Completed).contains("completed"));
    }
}
