//! anyCode Core - 核心抽象层
//!
//! 任务、消息、工具、Agent、记忆与安全策略等领域类型与 trait。
//! 实现按子模块拆分；本文件仅做聚合与 `prelude` 导出。

mod agent_type;
mod channel;
mod error;
mod execution_trace;
mod feature_flags;
mod goal;
mod ids;
mod llm_retry_observer;
mod llm_types;
mod memory_model;
mod memory_pipeline;
mod message;
mod model_profile;
mod plan_tree;
mod query_source;
mod reasoning;
mod runtime_profile;
mod secret_ref;
mod security_policy;
mod session_notification;
mod slash_command;
mod task;
mod task_gate_log;
mod task_output;
mod tool_catalog;
mod traits;
mod vision;
mod workflow;

pub use agent_type::AgentType;
pub use channel::{ChannelMessage, ChannelType};
pub use error::{anyhow_error_is_cooperative_cancel, CoreError};
pub use execution_trace::{ExecutionTraceEvent, EXECUTION_TRACE_SCHEMA_VERSION};
pub use feature_flags::{FeatureFlag, FeatureRegistry};
pub use goal::{GoalProgress, GoalSpec};
pub use ids::{
    AgentId, SessionId, TaskId, ToolName, ANYCODE_COMPACT_SUMMARY_METADATA_KEY,
    ANYCODE_CONTEXT_USER_METADATA_KEY, ANYCODE_TOOL_CALLS_METADATA_KEY,
};
pub use llm_retry_observer::LlmRetryObserver;
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
pub use plan_tree::{
    apply_plan_patches, format_plan_tree_summary, format_plan_tree_terminal,
    plan_tree_all_completed, plan_tree_is_empty, rollup_plan_statuses, validate_plan_tree,
    PlanLimits, PlanNode, PlanNodeKind, PlanPatch, PlanStatus, PlanTree, PlanValidationError,
    PLAN_TREE_CONTEXT_PREFIX, PLAN_TREE_MAX_DEPTH, PLAN_TREE_MAX_NODES,
};
pub use query_source::QuerySource;
pub use reasoning::{strip_llm_reasoning_for_display, strip_llm_reasoning_xml_blocks};
pub use runtime_profile::{RuntimeMode, RuntimeProfile};
pub use secret_ref::{SecretRef, SecretResolver};
pub use security_policy::SecurityPolicy;
pub use session_notification::SessionNotificationSettings;
pub use slash_command::{SlashCommand, SlashCommandScope, BUILTIN_SLASH_COMMANDS};
pub use task::{
    resolve_agent_loop_limits, AgentLoopLimits, Artifact, NestedTaskInvoke, NestedTaskRun, Task,
    TaskBudget, TaskContext, TaskResult, TurnOutput, TurnTokenUsage, DEFAULT_MAX_AGENT_TURNS,
    DEFAULT_MAX_TOOL_CALLS, MAX_AGENT_TURNS_CLAMP, MAX_TOOL_CALLS_CLAMP,
    NESTED_TASK_COOPERATIVE_CANCEL_ERROR,
};
pub use task_gate_log::{
    append_gate_log, decode_log_text, encode_log_text, format_assistant_response_log_line,
    format_gate_log_line, format_user_prompt_log_line,
};
pub use task_output::DiskTaskOutput;
pub use tool_catalog::{
    tool_catalog, tool_catalog_entry, ToolCatalogEntry, DEFAULT_TOOL_IDS,
    SECURITY_SENSITIVE_TOOL_IDS,
};
pub use traits::{Agent, ChannelHandler, LLMClient, MemoryStore, SubAgentExecutor, Tool};
pub use vision::{
    attach_vision_images, vision_images_from_metadata, VisionImage,
    ANYCODE_VISION_IMAGES_METADATA_KEY,
};
pub use workflow::{
    PlanValidationIssue, PlanValidationResult, WorkflowDefinition, WorkflowHandoff, WorkflowRetry,
    WorkflowStep,
};

/// Workspace product version (from root `Cargo.toml` via `CARGO_PKG_VERSION`).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// HTTP User-Agent string for a named anyCode component.
#[must_use]
pub fn user_agent(component: &str) -> String {
    format!("{component}/{VERSION}")
}

pub mod prelude {
    pub use super::anyhow_error_is_cooperative_cancel;
    pub use super::CoreError;
    pub use super::{
        attach_vision_images, vision_images_from_metadata, Agent, AgentLoopLimits, AgentType,
        ChannelHandler, ChannelMessage, ChannelType, DiskTaskOutput, EmbeddingProvider,
        ExecutionTraceEvent, FeatureFlag, FeatureRegistry, GoalProgress, GoalSpec, LLMClient,
        LLMProvider, LLMResponse, Memory, MemoryPipeline, MemoryPipelineSettings, MemoryScope,
        MemoryStore, MemoryType, Message, MessageContent, MessageRole, ModelConfig,
        ModelRouteProfile, NestedTaskInvoke, NestedTaskRun, PermissionMode, PlanLimits, PlanNode,
        PlanNodeKind, PlanPatch, PlanStatus, PlanTree, PlanValidationError, PlanValidationIssue,
        PlanValidationResult, PreSemanticFragment, RuntimeMode, RuntimeProfile, SecretRef,
        SecretResolver, SecurityPolicy, SessionNotificationSettings, SlashCommand,
        SlashCommandScope, StreamEvent, SubAgentExecutor, Task, TaskBudget, TaskContext, TaskId,
        TaskResult, Tool, ToolCall, ToolInput, ToolName, ToolOutput, ToolSchema, TurnOutput,
        TurnTokenUsage, Usage, VectorMemoryBackend, VisionImage, WorkflowDefinition,
        WorkflowHandoff, WorkflowRetry, WorkflowStep, ANYCODE_COMPACT_SUMMARY_METADATA_KEY,
        ANYCODE_CONTEXT_USER_METADATA_KEY, ANYCODE_TOOL_CALLS_METADATA_KEY,
        ANYCODE_VISION_IMAGES_METADATA_KEY, BUILTIN_SLASH_COMMANDS, DEFAULT_MAX_AGENT_TURNS,
        DEFAULT_MAX_TOOL_CALLS, EXECUTION_TRACE_SCHEMA_VERSION, MAX_AGENT_TURNS_CLAMP,
        MAX_TOOL_CALLS_CLAMP, NESTED_TASK_COOPERATIVE_CANCEL_ERROR, PLAN_TREE_CONTEXT_PREFIX,
        PLAN_TREE_MAX_DEPTH, PLAN_TREE_MAX_NODES,
    };
}
