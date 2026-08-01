//! anyCode Core - 核心抽象层
//!
//! 任务、消息、工具、Agent、记忆与安全策略等领域类型与 trait。
//! 实现按子模块拆分；本文件仅做聚合与 `prelude` 导出。

mod agent_type;
mod artifact_kind;
mod chat_turn;
mod error;
mod eval;
mod eval_baseline;
mod execution_trace;
mod experience_pack;
mod feature_flags;
mod goal;
mod ids;
mod live_trace;
mod llm_retry_observer;
mod llm_types;
mod memory_episode;
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
mod task_spec;
mod tool_catalog;
mod traits;
mod verification;
mod vision;
mod workflow;

pub use agent_type::AgentType;
pub use artifact_kind::{
    artifact_kind_for_path, artifact_kind_is_inline, artifact_title_for_path, mime_for_path,
};
pub use chat_turn::{
    current_chat_turn, current_dashboard_session_id, current_host_intent_hint,
    current_reply_language, current_user_turn_id, scope_chat_turn, ChatTurnContext,
};
pub use error::{anyhow_error_is_cooperative_cancel, CoreError};
pub use eval::{judge_eval_scenario, EvalExpectation, EvalResult, EvalScenario, EvalStatus};
pub use eval_baseline::{
    builtin_baseline_scenarios, BaselineScenario, BaselineTaskCategory, EvalArm, EvalArmMetrics,
    EvalSuiteSummary,
};
pub use execution_trace::{ExecutionTraceEvent, EXECUTION_TRACE_SCHEMA_VERSION};
pub use experience_pack::{
    builtin_web_and_rust_pack, ExperienceCard, ExperiencePack, ExperiencePackMeta,
};
pub use feature_flags::{FeatureFlag, FeatureRegistry};
pub use goal::{GoalProgress, GoalSpec};
pub use ids::{
    AgentId, SessionId, TaskId, ToolName, ANYCODE_COMPACT_SUMMARY_METADATA_KEY,
    ANYCODE_CONTEXT_USER_METADATA_KEY, ANYCODE_REASONING_CONTENT_METADATA_KEY,
    ANYCODE_TOOL_CALLS_METADATA_KEY,
};
pub use live_trace::LiveTraceEvent;
pub use llm_retry_observer::LlmRetryObserver;
pub use llm_types::{
    LLMProvider, LLMResponse, ModelConfig, PermissionMode, StreamEvent, ToolCall, ToolInput,
    ToolOutput, ToolSchema, Usage,
};
pub use memory_episode::{
    looks_like_secret, EpisodeEvent, EpisodeRecord, MemoryKind, MemoryMetaV2, SurveyRating,
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
pub use reasoning::{
    extract_unclosed_reasoning_content, strip_llm_reasoning_for_display,
    strip_llm_reasoning_xml_blocks,
};
pub use runtime_profile::{RuntimeMode, RuntimeProfile};
pub use secret_ref::{SecretRef, SecretResolver};
pub use security_policy::SecurityPolicy;
pub use session_notification::SessionNotificationSettings;
pub use slash_command::{SlashCommand, SlashCommandScope, BUILTIN_SLASH_COMMANDS};
pub use task::{
    resolve_agent_loop_limits, AgentLoopLimits, Artifact, NestedTaskInvoke, NestedTaskRun, Task,
    TaskBudget, TaskContext, TaskResult, TerminationReason, TurnOutput, TurnTokenUsage,
    DEFAULT_MAX_AGENT_TURNS, DEFAULT_MAX_TOOL_CALLS, MAX_AGENT_TURNS_CLAMP, MAX_TOOL_CALLS_CLAMP,
    NESTED_TASK_COOPERATIVE_CANCEL_ERROR,
};
pub use task_gate_log::{
    append_gate_log, decode_log_text, encode_log_text, format_assistant_response_log_line,
    format_gate_log_line, format_user_prompt_log_line,
};
pub use task_output::DiskTaskOutput;
pub use task_spec::{AgentPromptPack, ClarifyingQuestion, ExpectedArtifact, TaskFamily, TaskSpec};
pub use tool_catalog::{
    tool_catalog, tool_catalog_entry, ToolCatalogEntry, DEFAULT_TOOL_IDS,
    SECURITY_SENSITIVE_TOOL_IDS, TOOL_BROWSER_CDP, TOOL_BROWSER_CLICK, TOOL_BROWSER_NAVIGATE,
    TOOL_BROWSER_PRESS_KEY, TOOL_BROWSER_SCREENSHOT, TOOL_BROWSER_SCROLL, TOOL_BROWSER_SNAPSHOT,
    TOOL_BROWSER_TABS, TOOL_BROWSER_TYPE,
};
pub use traits::{Agent, LLMClient, MemoryStore, SubAgentExecutor, Tool};
pub use verification::{
    GatePlan, GatePolicy, GateRequirement, GateSeverity, VerificationOutcome, VerificationReport,
    VerificationResult, VERIFICATION_SCHEMA_VERSION,
};
pub use vision::{
    attach_vision_images, vision_images_from_metadata, VisionImage,
    ANYCODE_VISION_IMAGES_METADATA_KEY,
};
pub use workflow::{
    webpage_default_workflow, workflow_ready_steps, workflow_topo_layers, PlanValidationIssue,
    PlanValidationResult, WorkflowCheckpoint, WorkflowDefinition, WorkflowHandoff, WorkflowRetry,
    WorkflowStep, WorkflowStepState, WorkflowStepStatus,
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
        attach_vision_images, current_chat_turn, current_dashboard_session_id,
        current_reply_language, current_user_turn_id, scope_chat_turn, vision_images_from_metadata,
        Agent, AgentLoopLimits, AgentType, ChatTurnContext, DiskTaskOutput, EmbeddingProvider,
        ExecutionTraceEvent, ExperienceCard, ExperiencePack, FeatureFlag, FeatureRegistry,
        GoalProgress, GoalSpec, LLMClient, LLMProvider, LLMResponse, LiveTraceEvent, Memory,
        MemoryKind, MemoryMetaV2, MemoryPipeline, MemoryPipelineSettings, MemoryScope, MemoryStore,
        MemoryType, Message, MessageContent, MessageRole, ModelConfig, ModelRouteProfile,
        NestedTaskInvoke, NestedTaskRun, PermissionMode, PlanLimits, PlanNode, PlanNodeKind,
        PlanPatch, PlanStatus, PlanTree, PlanValidationError, PlanValidationIssue,
        PlanValidationResult, PreSemanticFragment, RuntimeMode, RuntimeProfile, SecretRef,
        SecretResolver, SecurityPolicy, SessionNotificationSettings, SlashCommand,
        SlashCommandScope, StreamEvent, SubAgentExecutor, SurveyRating, Task, TaskBudget,
        TaskContext, TaskFamily, TaskId, TaskResult, TaskSpec, TerminationReason, Tool, ToolCall,
        ToolInput, ToolName, ToolOutput, ToolSchema, TurnOutput, TurnTokenUsage, Usage,
        VectorMemoryBackend, VisionImage, WorkflowCheckpoint, WorkflowDefinition, WorkflowHandoff,
        WorkflowRetry, WorkflowStep, WorkflowStepStatus, ANYCODE_COMPACT_SUMMARY_METADATA_KEY,
        ANYCODE_CONTEXT_USER_METADATA_KEY, ANYCODE_REASONING_CONTENT_METADATA_KEY,
        ANYCODE_TOOL_CALLS_METADATA_KEY, ANYCODE_VISION_IMAGES_METADATA_KEY,
        BUILTIN_SLASH_COMMANDS, DEFAULT_MAX_AGENT_TURNS, DEFAULT_MAX_TOOL_CALLS,
        EXECUTION_TRACE_SCHEMA_VERSION, MAX_AGENT_TURNS_CLAMP, MAX_TOOL_CALLS_CLAMP,
        NESTED_TASK_COOPERATIVE_CANCEL_ERROR, PLAN_TREE_CONTEXT_PREFIX, PLAN_TREE_MAX_DEPTH,
        PLAN_TREE_MAX_NODES,
    };
}
