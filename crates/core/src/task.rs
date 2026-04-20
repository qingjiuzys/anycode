//! 任务、产物与单轮产出。

use crate::ids::{SessionId, TaskId};
use crate::llm_types::Usage;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

use crate::agent_type::AgentType;

/// `execute_task` 协作式取消：与 [`TaskContext::nested_cancel`] 对应；**`TaskStop`** 对后台嵌套任务会置位。
pub const NESTED_TASK_COOPERATIVE_CANCEL_ERROR: &str = "cancelled";

/// 任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub agent_type: AgentType,
    pub prompt: String,
    pub context: TaskContext,
    pub created_at: DateTime<Utc>,
}

/// 任务上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContext {
    pub session_id: SessionId,
    pub working_directory: String,
    pub environment: HashMap<String, String>,
    pub user_id: Option<String>,
    /// 追加到合成后的 system 消息末尾（如微信 `systemPrompt`）；与 `config.json` 的 append 叠加。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt_append: Option<String>,
    /// 作为会话状态上下文注入到 system 之后（非 system 规则）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context_injections: Vec<String>,
    /// Claude Code `Agent` tool: `sonnet` / `opus` / `haiku` or raw model id — applied only for nested runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nested_model_override: Option<String>,
    /// When set with [`Self::nested_worktree_repo_root`], `execute_task` removes this git worktree after the run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nested_worktree_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nested_worktree_repo_root: Option<String>,
    /// 嵌套子 Agent（如 `run_in_background`）：`true` 时 turn / 工具边界提前退出（非 serde）。
    #[serde(skip)]
    pub nested_cancel: Option<Arc<AtomicBool>>,
    /// 可选：工具进度短行（如微信桥）；`execute_task` 在工具开始/结束时 **try-send** UTF-8 行。
    #[serde(skip, default)]
    pub channel_progress_tx: Option<UnboundedSender<String>>,
}

/// Parameters for [`crate::SubAgentExecutor::run_nested_task`] (Claude Code `Agent` / `Task` tool parity).
#[derive(Debug, Clone)]
pub struct NestedTaskInvoke {
    pub agent_type: AgentType,
    pub prompt: String,
    pub working_directory: String,
    pub model: Option<String>,
    /// `Some("worktree")` → isolated git worktree (Claude `isolation: "worktree"`).
    pub isolation: Option<String>,
    /// When set, nested `Task.id` uses this UUID so callers can return `nested_task_id` before `execute_task` finishes (background agents).
    pub task_id: Option<crate::ids::TaskId>,
    /// Shared flag for cooperative cancel (e.g. background nested agent + **`TaskStop`**).
    pub cancel: Option<Arc<AtomicBool>>,
}

/// 嵌套 Agent / `Task` 工具一次调用的结果：携带与 `DiskTaskOutput` / `output.log` 一致的 **`task_id`**。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NestedTaskRun {
    pub task_id: TaskId,
    pub result: TaskResult,
}

/// 任务结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskResult {
    Success {
        output: String,
        artifacts: Vec<Artifact>,
    },
    Failure {
        error: String,
        details: Option<String>,
    },
    Partial {
        success: String,
        remaining: String,
    },
}

/// 产物 (文件、数据等)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub name: String,
    pub path: Option<String>,
    pub content: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// 单轮 `execute_turn_from_messages` 内各次 LLM 调用的 token 聚合（供 HUD / 脚标 / status line）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnTokenUsage {
    /// 各次 `usage.input_tokens` 的最大值（与自动压缩阈值一致）。
    pub max_input_tokens: u32,
    /// 各次 `usage.output_tokens` 之和。
    pub total_output_tokens: u32,
    pub total_cache_read_tokens: u32,
    pub total_cache_creation_tokens: u32,
}

impl TurnTokenUsage {
    /// 映射为单次 `Usage`，供 JSON status line 等消费。
    #[must_use]
    pub fn to_usage(&self) -> Usage {
        Usage {
            input_tokens: self.max_input_tokens,
            output_tokens: self.total_output_tokens,
            cache_creation_tokens: (self.total_cache_creation_tokens > 0)
                .then_some(self.total_cache_creation_tokens),
            cache_read_tokens: (self.total_cache_read_tokens > 0)
                .then_some(self.total_cache_read_tokens),
        }
    }
}

/// TUI / 行式 REPL 单轮 `execute_turn_from_messages` 的产出。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnOutput {
    pub final_text: String,
    pub artifacts: Vec<Artifact>,
    pub usage: TurnTokenUsage,
}
