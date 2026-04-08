//! 任务、产物与单轮产出。

use crate::ids::{SessionId, TaskId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::agent_type::AgentType;

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

/// TUI 单轮 `execute_turn_from_messages` 的产出（含用于自动压缩的上下文规模）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnOutput {
    pub final_text: String,
    pub artifacts: Vec<Artifact>,
    /// 本轮工具循环内各次 `LLMClient::chat` 的 `usage.input_tokens` 最大值。
    pub max_input_tokens: u32,
}
