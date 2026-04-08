//! 可替换实现边界（Agent / Tool / LLM / 记忆 / 通道）。

use async_trait::async_trait;

use crate::agent_type::AgentType;
use crate::channel::{ChannelMessage, ChannelType};
use crate::error::CoreError;
use crate::ids::ToolName;
use crate::llm_types::{
    LLMResponse, ModelConfig, PermissionMode, StreamEvent, ToolInput, ToolOutput, ToolSchema,
};
use crate::memory_model::{Memory, MemoryType};
use crate::message::Message;
use crate::security_policy::SecurityPolicy;
use crate::task::{NestedTaskInvoke, NestedTaskRun, Task, TaskResult};

/// Agent 抽象（类型、工具子集、说明与可选独立执行路径）
///
/// **编排权威**：实际多轮 LLM + 工具循环由 `anycode_agent::AgentRuntime::execute_task`
/// 与同类型的 `execute_turn_from_messages`（TUI）完成；本 trait 的 `execute` **不是**主路径，内置 agent 多为占位实现，
/// 供类型系统与将来「非 runtime 编排」扩展预留。新增能力时优先扩展 runtime 与 `Tool`，而非假设会调用 `Agent::execute`。
#[async_trait]
pub trait Agent: Send + Sync {
    fn agent_type(&self) -> &AgentType;
    fn description(&self) -> &str;
    fn tools(&self) -> Vec<ToolName>;
    async fn execute(&mut self, task: Task) -> Result<TaskResult, CoreError>;
    fn supports_concurrency(&self) -> bool {
        false
    }
    fn system_prompt_replaces_default_sections(&self) -> Option<&str> {
        None
    }
}

/// 由主 `AgentRuntime` 实现，供 `Agent` / `Task` 等工具嵌套调用 `execute_task`（避免 tools ↔ agent 循环依赖）。
#[async_trait]
pub trait SubAgentExecutor: Send + Sync {
    async fn run_nested_task(&self, invoke: NestedTaskInvoke) -> Result<NestedTaskRun, CoreError>;
}

/// 工具抽象（名称、Schema、执行与 API 面向模型的描述）
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn api_tool_description(&self) -> String {
        self.description().to_string()
    }
    fn schema(&self) -> serde_json::Value;
    fn permission_mode(&self) -> PermissionMode;
    fn security_policy(&self) -> Option<&SecurityPolicy>;
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError>;
}

/// 记忆存储（多类型 Project/User/Session 等）
#[async_trait]
pub trait MemoryStore: Send + Sync {
    async fn save(&self, memory: Memory) -> Result<(), CoreError>;
    async fn recall(&self, query: &str, mem_type: MemoryType) -> Result<Vec<Memory>, CoreError>;
    async fn update(&self, id: &str, memory: Memory) -> Result<(), CoreError>;
    async fn delete(&self, id: &str) -> Result<(), CoreError>;
}

/// LLM 客户端抽象
#[async_trait]
pub trait LLMClient: Send + Sync {
    async fn chat(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSchema>,
        config: &ModelConfig,
    ) -> Result<LLMResponse, CoreError>;

    async fn chat_stream(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSchema>,
        config: &ModelConfig,
    ) -> Result<tokio::sync::mpsc::Receiver<StreamEvent>, CoreError>;
}

/// 通道处理器
#[async_trait]
pub trait ChannelHandler: Send + Sync {
    fn channel_type(&self) -> ChannelType;
    async fn send_message(&self, msg: ChannelMessage) -> Result<(), CoreError>;
    async fn message_stream(
        &self,
    ) -> Result<tokio::sync::mpsc::Receiver<ChannelMessage>, CoreError>;
    fn supports_streaming(&self) -> bool {
        false
    }
}
