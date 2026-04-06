//! 跨域标识与消息元数据常量。

use uuid::Uuid;

/// 任务 ID
pub type TaskId = Uuid;

/// Agent ID
pub type AgentId = Uuid;

/// 会话 ID
pub type SessionId = Uuid;

/// 工具名称
pub type ToolName = String;

/// Assistant `Message.metadata` 键：保存本轮 `Vec<ToolCall>` JSON，供 LLM 客户端重建工具调用历史。
pub const ANYCODE_TOOL_CALLS_METADATA_KEY: &str = "anycode_tool_calls";

/// User `Message.metadata`：本条为会话压缩后的续接摘要（与 Claude Code `isCompactSummary` 对齐）。
pub const ANYCODE_COMPACT_SUMMARY_METADATA_KEY: &str = "anycode_compact_summary";
