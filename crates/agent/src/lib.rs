//! anyCode Agent Engine
//!
//! anyCode Agent 运行时：多轮工具循环、路由与内存

mod agents;
mod compact;
mod goal_engine;
mod prompt_assembler;
mod runtime;
mod system_prompt;
mod workspace_assistant;

pub use agents::{ExploreAgent, GeneralPurposeAgent, PlanAgent};
pub use compact::{
    CompactPolicy, CompactionHooks, CompactionPostContext, CompactionPreContext,
    DefaultCompactionHooks, FileReadSnippet, SessionCompactionState,
};
pub use goal_engine::GoalEngine;
pub use runtime::{AgentClaudeToolGating, AgentRuntime};
pub use system_prompt::RuntimePromptConfig;
pub use workspace_assistant::{GoalAgent, WorkspaceAssistantAgent};

#[cfg(test)]
include!("agent_test_mod.inc");
