//! 压缩生命周期钩子（与 Claude PreCompact / PostCompact 对齐的可扩展点）。

use super::microcompact::{apply_microcompact, default_keep_recent};
use super::post_compact::{inject_file_snippets_from_state, run_post_compact_cleanup};
use super::state::SessionCompactionState;
use anycode_core::prelude::*;

/// 摘要 API 请求已组装、尚未调用摘要模型之前。
pub struct CompactionPreContext<'a> {
    pub session: &'a [Message],
    pub api_messages: &'a mut Vec<Message>,
    /// 若 `pre_compact` 中执行了 microcompact，应写入被置为占位文案的 `tool_result` 条数（默认实现由 [`DefaultCompactionHooks`] 填写）。
    pub microcompact_cleared: usize,
}

/// 摘要已写入 `compacted_messages`（通常为 system + compact user），可再追加或打日志。
pub struct CompactionPostContext<'a> {
    pub session_before: &'a [Message],
    pub compacted_messages: &'a mut Vec<Message>,
    pub state: &'a mut SessionCompactionState,
}

/// 会话压缩管线扩展点；默认实现 = microcompact + FileRead 摘录 + cleanup 占位。
pub trait CompactionHooks: Send + Sync {
    fn pre_compact(&self, ctx: &mut CompactionPreContext<'_>) -> Result<(), CoreError>;
    fn post_compact(&self, ctx: &mut CompactionPostContext<'_>) -> Result<(), CoreError>;
}

/// Claude Code 默认行为的 anyCode 实现。
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultCompactionHooks;

impl DefaultCompactionHooks {
    pub fn new() -> Self {
        Self
    }
}

impl CompactionHooks for DefaultCompactionHooks {
    fn pre_compact(&self, ctx: &mut CompactionPreContext<'_>) -> Result<(), CoreError> {
        ctx.microcompact_cleared = apply_microcompact(ctx.api_messages, default_keep_recent());
        Ok(())
    }

    fn post_compact(&self, ctx: &mut CompactionPostContext<'_>) -> Result<(), CoreError> {
        ctx.state.refresh_from_messages(ctx.session_before);
        inject_file_snippets_from_state(ctx.compacted_messages, ctx.state);
        run_post_compact_cleanup();
        Ok(())
    }
}
