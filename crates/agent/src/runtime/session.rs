//! 会话级压缩 API（`/compact`）；与 `mod.rs` 中其它 `AgentRuntime` 方法分文件以降低体量。

use super::AgentRuntime;
use crate::compact::{self, CompactionPostContext, CompactionPreContext, SessionCompactionState};
use anycode_core::prelude::*;

impl AgentRuntime {
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
        let (raw, usage) = compact::run_compact_llm(
            &self.llm_client,
            &summary_model,
            api_msgs,
            custom_instructions,
        )
        .await?;
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
        Ok((out, usage))
    }
}
