//! 会话级压缩 API（`/compact`）；与 `mod.rs` 中其它 `AgentRuntime` 方法分文件以降低体量。

use super::AgentRuntime;
use crate::compact::{self, CompactionPostContext, CompactionPreContext, SessionCompactionState};
use anycode_core::prelude::*;
use std::sync::Arc;

impl AgentRuntime {
    pub(crate) async fn maybe_auto_compact_messages(
        &self,
        task_id: TaskId,
        agent_type: &AgentType,
        working_directory: &str,
        model: &ModelConfig,
        messages: &mut Vec<Message>,
        last_input_tokens: u32,
        _turn: usize,
    ) -> Result<bool, CoreError> {
        // Overflow protection is unconditional: once the context threshold is
        // reached compaction must run, including for weak local models during
        // their warm-up turns (tool-schema reduction handles those separately).
        let context_tokens = self.effective_context_window_tokens(model);
        // PreCompact skip（对齐 Claude `SKIP_PRECOMPACT_THRESHOLD`）：消息/指纹过少时跳过压缩，
        // `CLAUDE_CODE_DISABLE_PRECOMPACT_SKIP` 可禁用该跳过。
        let skip = compact::should_skip_precompact(
            messages.len(),
            compact::SKIP_PRECOMPACT_THRESHOLD,
            self.auto_compact_policy.disable_precompact_skip,
        );
        if !self.auto_compact
            || !self
                .auto_compact_policy
                .should_compact(context_tokens, last_input_tokens)
            || skip
            || messages.len() < 3
        {
            return Ok(false);
        }
        self.log_task_line(
            task_id,
            &format!(
                "[auto_compact] input_tokens={} context_tokens={} action=start",
                last_input_tokens, context_tokens
            ),
        );
        let snapshot = messages.clone();
        let (compacted, _) = self
            .compact_session_messages(
                agent_type,
                working_directory,
                &snapshot,
                None,
                self.auto_compact_policy.suppress_follow_up_questions,
                None,
            )
            .await?;
        *messages = compacted;
        self.log_task_line(task_id, "[auto_compact] action=completed");
        Ok(true)
    }

    pub(crate) async fn maybe_auto_compact_shared(
        &self,
        task_id: TaskId,
        agent_type: &AgentType,
        working_directory: &str,
        model: &ModelConfig,
        messages: &Arc<tokio::sync::Mutex<Vec<Message>>>,
        last_input_tokens: u32,
        turn: usize,
    ) -> Result<bool, CoreError> {
        let mut snapshot = messages.lock().await.clone();
        if !self
            .maybe_auto_compact_messages(
                task_id,
                agent_type,
                working_directory,
                model,
                &mut snapshot,
                last_input_tokens,
                turn,
            )
            .await?
        {
            return Ok(false);
        }
        *messages.lock().await = snapshot;
        Ok(true)
    }

    /// 会话压缩（Claude Code `/compact`）：折叠为 `[system, compact_summary_user]`。
    pub async fn compact_session_messages(
        &self,
        agent_type: &AgentType,
        working_directory: &str,
        session: &[Message],
        custom_instructions: Option<&str>,
        suppress_follow_up: bool,
        transcript_path: Option<&str>,
    ) -> Result<(Vec<Message>, Usage), CoreError> {
        let fresh_system = self
            .build_system_message(agent_type, working_directory)
            .await?;
        let mut api_msgs = compact::build_compact_api_messages(fresh_system.clone(), session)?;
        let microcompact_cleared = {
            let mut pre_ctx = CompactionPreContext {
                session,
                api_messages: &mut api_msgs,
                microcompact_cleared: 0,
                variant: compact::CompactVariant::Precompact,
            };
            self.compaction_hooks.pre_compact(&mut pre_ctx)?;
            pre_ctx.microcompact_cleared
        };
        if microcompact_cleared > 0 {
            tracing::info!(
                target: "anycode_agent",
                cleared = microcompact_cleared,
                "microcompact before full compact"
            );
        }
        let summary_model = self.model_for_summary().clone();
        // 压缩缓存（对齐 Claude `.precompact.json`）：同尾部消息再次压缩时复用摘要，跳过 LLM。
        let session_id = compact::cache_session_id(transcript_path, working_directory);
        let fingerprint = compact::fingerprint_message_ids(session);
        let cache_hit = if self.auto_compact_policy.disable_precompact_skip {
            None
        } else {
            compact::read_precompact_cache(&session_id, &fingerprint)
        };
        let (raw, usage) = match cache_hit {
            Some(raw) => {
                tracing::info!(
                    target: "anycode_agent",
                    session_id,
                    "precompact cache hit, skipping summary LLM"
                );
                (
                    raw,
                    Usage {
                        input_tokens: 0,
                        output_tokens: 0,
                        cache_creation_tokens: None,
                        cache_read_tokens: Some(fingerprint.len() as u32),
                    },
                )
            }
            None => {
                let (raw, usage) = compact::run_compact_llm(
                    &self.llm_client,
                    &summary_model,
                    api_msgs,
                    custom_instructions,
                )
                .await?;
                compact::write_precompact_cache(&session_id, &fingerprint, &raw);
                (raw, usage)
            }
        };
        let mut out = compact::build_post_compact_messages(
            fresh_system,
            &raw,
            suppress_follow_up,
            transcript_path,
        )?;
        let mut compaction_state = SessionCompactionState::default();
        self.compaction_hooks
            .post_compact(&mut CompactionPostContext {
                session_before: session,
                compacted_messages: &mut out,
                state: &mut compaction_state,
            })?;
        compact::append_compaction_checkpoint(session, &out, "manual_compact");
        Ok((out, usage))
    }

    /// One-shot overflow recovery: compact in place and replace `messages`.
    pub(crate) async fn recover_from_context_overflow(
        &self,
        task_id: TaskId,
        agent_type: &AgentType,
        working_directory: &str,
        messages: &Arc<tokio::sync::Mutex<Vec<Message>>>,
    ) -> Result<(), CoreError> {
        let snapshot = { messages.lock().await.clone() };
        if snapshot.len() < 2 {
            return Err(CoreError::LLMError(
                "context overflow with insufficient messages to compact".into(),
            ));
        }
        self.log_task_line(
            task_id,
            "[overflow_recovery] compacting session after context overflow",
        );
        let (compacted, _) = self
            .compact_session_messages(agent_type, working_directory, &snapshot, None, true, None)
            .await?;
        compact::append_compaction_checkpoint(&snapshot, &compacted, "overflow_recovery");
        *messages.lock().await = compacted;
        Ok(())
    }
}
