//! Aggregated constructor arguments for [`super::AgentRuntime::new`].
//! Keeps the composition root (`bootstrap`) and tests readable without changing runtime behavior.

use std::collections::HashMap;
use std::sync::Arc;

use anycode_core::prelude::*;
use anycode_core::{
    DiskTaskOutput, MemoryPipeline, MemoryPipelineSettings, SessionNotificationSettings,
};
use anycode_security::SecurityLayer;
use regex::Regex;

use crate::system_prompt::RuntimePromptConfig;

use super::tool_gating::AgentClaudeToolGating;

/// Core dependencies: LLM, tools, memory store, model config, security, prompts.
pub struct RuntimeCoreDeps {
    pub llm_client: Arc<dyn LLMClient>,
    pub tools: HashMap<ToolName, Box<dyn Tool>>,
    pub memory_store: Arc<dyn MemoryStore>,
    pub default_model_config: ModelConfig,
    pub model_overrides: HashMap<AgentType, ModelConfig>,
    pub disk_output: Option<DiskTaskOutput>,
    pub security: Arc<SecurityLayer>,
    pub sandbox_mode: bool,
    pub prompt_config: RuntimePromptConfig,
}

/// Optional memory pipeline and project autosave behavior.
pub struct RuntimeMemoryOptions {
    pub memory_pipeline: Option<Arc<dyn MemoryPipeline>>,
    pub memory_pipeline_settings: Option<MemoryPipelineSettings>,
    pub memory_project_autosave_enabled: bool,
    /// 外向会话通知（`config.notifications`）；与 `memory_pipeline` 无耦合。
    pub session_notifications: Option<SessionNotificationSettings>,
}

/// Tool listing policy: deny patterns, Claude permission rules, skill exposure on explore/plan agents.
pub struct RuntimeToolPolicy {
    pub tool_name_deny: Vec<Regex>,
    pub claude_gating: AgentClaudeToolGating,
    pub expose_skill_on_explore_plan: bool,
}
