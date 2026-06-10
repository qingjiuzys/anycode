//! anyCode Agent Engine
//!
//! anyCode Agent 运行时：多轮工具循环、路由与内存

mod agent_profiles;
mod agents;
mod compact;
mod declarative_agent;
mod goal_engine;
mod model_instructions;
mod nested_model;
mod prompt_assembler;
mod runtime;
mod system_prompt;
mod workspace_assistant;

pub use agent_profiles::{
    apply_tool_filters, base_tools_for_extends, is_builtin_extends, profile_spec_for_builtin,
    resolve_profile, runtime_mode_for_extends, AgentProfileSpec, BuiltinAgentSeed,
    ResolvedAgentProfile, BUILTIN_AGENT_SEED, BUILTIN_EXTENDS, SHIPPED_ROLE_IDS,
};
pub use agents::{ExploreAgent, GeneralPurposeAgent, PlanAgent};
pub use compact::{
    CompactPolicy, CompactionHooks, CompactionPostContext, CompactionPreContext,
    DefaultCompactionHooks, FileReadSnippet, SessionCompactionState,
};
pub use declarative_agent::ProfileAgent;
pub use goal_engine::GoalEngine;
pub use model_instructions::{
    discover_model_instructions, ModelInstructionsConfig, ModelInstructionsFile,
    DEFAULT_MODEL_INSTRUCTIONS_FILENAME, MODEL_INSTRUCTIONS_FILENAMES,
};
pub use prompt_assembler::{render_system_prompt_segments, PromptAssembler, SystemPromptSegment};
pub use runtime::{
    failover::{error_triggers_failover, FailoverPolicy},
    AgentClaudeToolGating, AgentRuntime, RuntimeCoreDeps, RuntimeMemoryOptions, RuntimeToolPolicy,
};
pub use system_prompt::RuntimePromptConfig;
pub use workspace_assistant::{GoalAgent, WorkspaceAssistantAgent};

#[cfg(test)]
mod tests;
