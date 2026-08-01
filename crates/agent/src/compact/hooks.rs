//! 压缩生命周期钩子（与 Claude PreCompact / PostCompact 对齐的可扩展点）。

use super::microcompact::{apply_microcompact, default_keep_recent};
use super::post_compact::{inject_file_snippets_from_state, run_post_compact_cleanup};
use super::state::SessionCompactionState;
use super::variant::{apply_tool_use_summaries, CompactVariant};
use anycode_core::prelude::*;

/// 摘要 API 请求已组装、尚未调用摘要模型之前。
pub struct CompactionPreContext<'a> {
    pub session: &'a [Message],
    pub api_messages: &'a mut Vec<Message>,
    /// 若 `pre_compact` 中执行了 microcompact，应写入被置为占位文案的 `tool_result` 条数（默认实现由 [`DefaultCompactionHooks`] 填写）。
    pub microcompact_cleared: usize,
    /// 压缩变体（对齐 Claude PreCompact/Cold/Away/Classifier/ToolUse 分支）。
    pub variant: CompactVariant,
}

/// 摘要已写入 `compacted_messages`（通常为 system + compact user），可再追加或打日志。
pub struct CompactionPostContext<'a> {
    pub session_before: &'a [Message],
    pub compacted_messages: &'a mut Vec<Message>,
    pub state: &'a mut SessionCompactionState,
}

/// 会话压缩管线扩展点；默认实现 = 变体分流 + microcompact + FileRead 摘录 + cleanup 占位。
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
        // 变体分流（对齐 Claude PreCompact / Cold / Away / Classifier / ToolUse 分支）：
        // - ToolUse：先聚合工具结果（`apply_tool_use_summaries`）再压缩；
        // - 其余变体：执行常规 microcompact（Cold/Away 整段压缩，无需额外跳过逻辑）。
        ctx.microcompact_cleared = match ctx.variant {
            CompactVariant::ToolUse => apply_tool_use_summaries(ctx.api_messages),
            CompactVariant::Precompact
            | CompactVariant::Cold
            | CompactVariant::Away
            | CompactVariant::Classifier => {
                apply_microcompact(ctx.api_messages, default_keep_recent())
            }
        };
        Ok(())
    }

    fn post_compact(&self, ctx: &mut CompactionPostContext<'_>) -> Result<(), CoreError> {
        ctx.state.refresh_from_messages(ctx.session_before);
        inject_file_snippets_from_state(ctx.compacted_messages, ctx.state);
        run_post_compact_cleanup();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anycode_tools::catalog::TOOL_FILE_READ;
    use std::collections::HashMap;

    fn asst_with_tool(id: &str, name: &str) -> Message {
        let mut meta = HashMap::new();
        meta.insert(
            ANYCODE_TOOL_CALLS_METADATA_KEY.to_string(),
            serde_json::to_value(vec![ToolCall {
                id: id.into(),
                name: name.into(),
                input: serde_json::json!({}),
            }])
            .unwrap(),
        );
        Message {
            id: uuid::Uuid::new_v4(),
            role: MessageRole::Assistant,
            content: MessageContent::Text("ok".into()),
            timestamp: chrono::Utc::now(),
            metadata: meta,
        }
    }

    fn tool_result_msg(id: &str, content: &str) -> Message {
        let mut meta = HashMap::new();
        meta.insert(
            "tool_name".to_string(),
            serde_json::Value::String(TOOL_FILE_READ.to_string()),
        );
        Message {
            id: uuid::Uuid::new_v4(),
            role: MessageRole::Tool,
            content: MessageContent::ToolResult {
                tool_use_id: id.into(),
                content: content.into(),
                is_error: false,
            },
            timestamp: chrono::Utc::now(),
            metadata: meta,
        }
    }

    fn api_with_tool_results(n: usize) -> Vec<Message> {
        let mut out = Vec::new();
        for i in 0..n {
            let id = format!("t{i}");
            out.push(asst_with_tool(&id, TOOL_FILE_READ));
            out.push(tool_result_msg(&id, &format!("file body {i}")));
        }
        out
    }

    #[test]
    fn pre_compact_applies_tool_use_summaries_variant() {
        // 4 条 tool result > 默认保留 3 条 → 清除 1 条。
        let session = api_with_tool_results(4);
        let mut api = api_with_tool_results(4);
        let mut ctx = CompactionPreContext {
            session: &session,
            api_messages: &mut api,
            microcompact_cleared: 0,
            variant: CompactVariant::ToolUse,
        };
        DefaultCompactionHooks::new()
            .pre_compact(&mut ctx)
            .expect("pre_compact");
        assert_eq!(ctx.microcompact_cleared, 1);
    }

    #[test]
    fn pre_compact_default_variant_runs_microcompact() {
        let session = api_with_tool_results(4);
        let mut api = api_with_tool_results(4);
        let mut ctx = CompactionPreContext {
            session: &session,
            api_messages: &mut api,
            microcompact_cleared: 0,
            variant: CompactVariant::Precompact,
        };
        DefaultCompactionHooks::new()
            .pre_compact(&mut ctx)
            .expect("pre_compact");
        assert_eq!(ctx.microcompact_cleared, 1);
    }
}
