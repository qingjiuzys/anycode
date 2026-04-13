//! anyCode Core - 核心抽象层
//!
//! 任务、消息、工具、Agent、记忆与安全策略等领域类型与 trait。
//! 实现按子模块拆分；本文件仅做聚合与 `prelude` 导出。

mod agent_type;
mod channel;
mod error;
mod feature_flags;
mod goal;
mod ids;
mod llm_types;
mod memory_model;
mod memory_pipeline;
mod message;
mod model_profile;
mod reasoning;
mod runtime_profile;
mod secret_ref;
mod security_policy;
mod slash_command;
mod task;
mod task_output;
mod traits;
mod workflow;

pub use agent_type::AgentType;
pub use channel::{ChannelMessage, ChannelType};
pub use error::CoreError;
pub use feature_flags::{FeatureFlag, FeatureRegistry};
pub use goal::{GoalProgress, GoalSpec};
pub use ids::{
    AgentId, SessionId, TaskId, ToolName, ANYCODE_COMPACT_SUMMARY_METADATA_KEY,
    ANYCODE_CONTEXT_USER_METADATA_KEY, ANYCODE_TOOL_CALLS_METADATA_KEY,
};
pub use llm_types::{
    LLMProvider, LLMResponse, ModelConfig, PermissionMode, StreamEvent, ToolCall, ToolInput,
    ToolOutput, ToolSchema, Usage,
};
pub use memory_model::{Memory, MemoryScope, MemoryType};
pub use memory_pipeline::{
    EmbeddingProvider, MemoryPipeline, MemoryPipelineSettings, PreSemanticFragment,
    VectorMemoryBackend,
};
pub use message::{Message, MessageContent, MessageRole};
pub use model_profile::ModelRouteProfile;
pub use reasoning::strip_llm_reasoning_xml_blocks;
pub use runtime_profile::{RuntimeMode, RuntimeProfile};
pub use secret_ref::{SecretRef, SecretResolver};
pub use security_policy::SecurityPolicy;
pub use slash_command::{SlashCommand, SlashCommandScope, BUILTIN_SLASH_COMMANDS};
pub use task::{
    Artifact, NestedTaskInvoke, NestedTaskRun, Task, TaskContext, TaskResult, TurnOutput,
    TurnTokenUsage, NESTED_TASK_COOPERATIVE_CANCEL_ERROR,
};
pub use task_output::DiskTaskOutput;
pub use traits::{Agent, ChannelHandler, LLMClient, MemoryStore, SubAgentExecutor, Tool};
pub use workflow::{WorkflowDefinition, WorkflowHandoff, WorkflowRetry, WorkflowStep};

pub mod prelude {
    pub use super::CoreError;
    pub use super::{
        Agent, AgentType, ChannelHandler, ChannelMessage, ChannelType, DiskTaskOutput,
        EmbeddingProvider, FeatureFlag, FeatureRegistry, GoalProgress, GoalSpec, LLMClient,
        LLMProvider, LLMResponse, Memory, MemoryPipeline, MemoryPipelineSettings, MemoryScope,
        MemoryStore, MemoryType, Message, MessageContent, MessageRole, ModelConfig,
        ModelRouteProfile, NestedTaskInvoke, NestedTaskRun, PermissionMode, PreSemanticFragment,
        RuntimeMode, RuntimeProfile, SecretRef, SecretResolver, SecurityPolicy, SlashCommand,
        SlashCommandScope, StreamEvent, SubAgentExecutor, Task, TaskContext, TaskId, TaskResult,
        Tool, ToolCall, ToolInput, ToolName, ToolOutput, ToolSchema, TurnOutput, TurnTokenUsage,
        Usage, VectorMemoryBackend, WorkflowDefinition, WorkflowHandoff, WorkflowRetry,
        WorkflowStep, ANYCODE_COMPACT_SUMMARY_METADATA_KEY, ANYCODE_CONTEXT_USER_METADATA_KEY,
        ANYCODE_TOOL_CALLS_METADATA_KEY, BUILTIN_SLASH_COMMANDS,
        NESTED_TASK_COOPERATIVE_CANCEL_ERROR,
    };
}
