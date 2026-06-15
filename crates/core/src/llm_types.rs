//! LLM / 工具调用相关的配置与载荷类型。

use serde::{Deserialize, Serialize};

use crate::ids::ToolName;
use crate::llm_retry_observer::LlmRetryObserver;
use crate::message::Message;
use crate::query_source::QuerySource;
use std::sync::Arc;

/// LLM 提供商
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum LLMProvider {
    Anthropic,
    OpenAI,
    Local,
    Custom(String),
}

/// 模型配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub provider: LLMProvider,
    pub model: String,
    #[serde(default)]
    pub base_url: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    /// 按路由覆盖时使用；`None` 表示使用对应 LLM 客户端构造时的默认密钥。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Retry/backoff tier for this request (defaults to foreground main turn).
    #[serde(default)]
    pub query_source: QuerySource,
    /// Per-request retry observer (runtime only; not serialized).
    #[serde(default, skip_serializing, skip_deserializing)]
    pub retry_observer: Option<Arc<dyn LlmRetryObserver>>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            provider: LLMProvider::Anthropic,
            model: String::new(),
            base_url: None,
            temperature: None,
            max_tokens: None,
            api_key: None,
            query_source: QuerySource::default(),
            retry_observer: None,
        }
    }
}

/// 权限模式（交互审批之上的一层快捷策略）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PermissionMode {
    Default,
    Auto,
    Plan,
    AcceptEdits,
    BypassPermissions,
}

/// 工具输入
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInput {
    pub name: ToolName,
    pub input: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,
    #[serde(default)]
    pub sandbox_mode: bool,
}

impl Default for ToolInput {
    fn default() -> Self {
        Self {
            name: String::new(),
            input: serde_json::Value::Null,
            working_directory: None,
            sandbox_mode: false,
        }
    }
}

/// 工具输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    pub result: serde_json::Value,
    pub error: Option<String>,
    pub duration_ms: u64,
}

/// 工具 Schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// LLM 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    pub message: Message,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Usage,
}

/// 工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
}

/// Token 使用情况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_creation_tokens: Option<u32>,
    pub cache_read_tokens: Option<u32>,
}

/// 流式事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamEvent {
    Delta(String),
    ToolCall(ToolCall),
    Usage(Usage),
    Done,
}
