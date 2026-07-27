//! Agent 运行时（LLM + 工具循环、落盘、回执）。

mod agentic_loop;
mod agentic_turn;
mod artifacts;
mod budget;
mod compile_context;
mod completion_guard;
mod evidence;
mod execute_eval;
mod execute_goal;
mod execute_task;
mod execute_tool;
mod execute_turn;
mod execute_turn_finalize;
pub mod failover;
mod limits;
mod live_trace_emit;
mod llm_retry;
mod logging;
mod memory_hooks;
mod nested_task;
mod nested_worktree;
mod plan_tree_context;
mod progress_update;
mod provider_errors;
mod receipt;
mod session;
mod session_activity;
mod session_notify;
mod task_summary;
mod tool_audit;
mod tool_dispatch;
mod tool_gating;
mod tool_invocation;
mod tool_output_sanitize;
mod tool_result_injection;
mod tool_surface;

mod runtime_options;
pub use runtime_options::{RuntimeCoreDeps, RuntimeMemoryOptions, RuntimeToolPolicy};
pub use tool_gating::AgentClaudeToolGating;

use crate::compact::{CompactPolicy, CompactionHooks, DefaultCompactionHooks};
use crate::prompt_assembler::{
    relevant_memories_context_section, runtime_mode_context_section, slash_commands_context_section,
};
use crate::system_prompt::{compose_effective_system_prompt, RuntimePromptConfig};
use crate::{ExploreAgent, GeneralPurposeAgent, GoalAgent, PlanAgent, WorkspaceAssistantAgent};
use anycode_core::prelude::*;
use anycode_core::{MemoryPipeline, MemoryPipelineSettings, SessionNotificationSettings};
use anycode_security::SecurityLayer;
use logging::RunLogger;
use regex::Regex;
use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::RwLock;
use tracing::warn;
use uuid::Uuid;

/// Agent 运行时
pub struct AgentRuntime {
    agents: Arc<RwLock<HashMap<AgentType, Box<dyn Agent>>>>,
    llm_client: Arc<dyn LLMClient>,
    tools: Arc<RwLock<HashMap<ToolName, Box<dyn Tool>>>>,
    memory_store: Arc<dyn MemoryStore>,
    /// `backend=pipeline` 时用于归根通道 ingest（autosave 进虚态缓冲）；否则为 `None`。
    memory_pipeline: Option<Arc<dyn MemoryPipeline>>,
    /// 与 `memory_pipeline` 配套；用于钩子与限流配置。
    memory_pipeline_settings: Option<MemoryPipelineSettings>,
    /// 可选：工具结果 / 回合结束外向通知（HTTP、shell），与记忆管线钩子独立。
    session_notifications: Option<SessionNotificationSettings>,
    default_model_config: ModelConfig,
    model_overrides: HashMap<AgentType, ModelConfig>,
    /// Optional fallback chat model when primary fails (geo / rate limit / etc.).
    failover_chain: Vec<failover::FailoverPolicy>,
    disk_output: Option<DiskTaskOutput>,
    /// 权限与审批（策略、沙箱、工具确认）
    security: Arc<SecurityLayer>,
    /// 与 config.security.sandbox_mode 对齐：工具内路径/cwd 约束
    sandbox_mode: bool,
    /// `config.json` 的 system_prompt_override / append（已解析 `@path`）
    prompt_config: RuntimePromptConfig,
    /// `memory.auto_save` 且非 noop 后端时，任务成功结束后写入一条 Project 记忆。
    memory_project_autosave_enabled: bool,
    /// 在 LLM 请求前从工具名列表中剔除匹配项（如 `mcp__.*` deny 正则）。
    tool_name_deny: Vec<Regex>,
    claude_gating: AgentClaudeToolGating,
    compaction_hooks: Arc<dyn CompactionHooks>,
    auto_compact: bool,
    auto_compact_policy: CompactPolicy,
    /// When false and `session_context_window_tokens > 0`, compaction uses the manual window.
    session_context_window_auto: bool,
    session_context_window_tokens: u32,
    tool_services: StdMutex<Option<Arc<anycode_tools::ToolServices>>>,
    completion_guard: Arc<completion_guard::CompletionGuard>,
}

fn canonical_agent_type(agent_type: &AgentType) -> AgentType {
    AgentType::new(crate::agent_profiles::normalize_agent_id(
        agent_type.as_str(),
    ))
}

pub(super) struct ParentToolSurfaceGuard {
    services: Arc<anycode_tools::ToolServices>,
    previous: Option<(Vec<String>, Vec<String>)>,
}

impl Drop for ParentToolSurfaceGuard {
    fn drop(&mut self) {
        // Restore the caller's surface instead of clearing: nested/concurrent
        // tasks must not wipe the parent's deny propagation.
        self.services
            .restore_parent_task_tool_deny(self.previous.take());
    }
}

impl AgentRuntime {
    fn context_messages_from_sections(&self, sections: Vec<String>) -> Vec<Message> {
        sections
            .into_iter()
            .filter(|section| !section.trim().is_empty())
            .map(|section| {
                let mut metadata = HashMap::new();
                metadata.insert(
                    ANYCODE_CONTEXT_USER_METADATA_KEY.to_string(),
                    serde_json::Value::Bool(true),
                );
                Message {
                    id: Uuid::new_v4(),
                    role: MessageRole::User,
                    content: MessageContent::Text(section),
                    timestamp: chrono::Utc::now(),
                    metadata,
                }
            })
            .collect()
    }

    fn build_context_sections(
        &self,
        mode: RuntimeMode,
        memories: &[Memory],
        extra_sections: &[String],
    ) -> Vec<String> {
        let mut sections = vec![
            runtime_mode_context_section(mode),
            slash_commands_context_section(),
        ];
        if let Some(section) = self.prompt_config.workspace_section.as_deref() {
            let t = section.trim();
            if !t.is_empty() {
                sections.push(t.to_string());
            }
        }
        if let Some(section) = self.prompt_config.channel_section.as_deref() {
            let t = section.trim();
            if !t.is_empty() {
                sections.push(t.to_string());
            }
        }
        if let Some(section) = self.prompt_config.workflow_section.as_deref() {
            let t = section.trim();
            if !t.is_empty() {
                sections.push(t.to_string());
            }
        }
        if let Some(section) = self.prompt_config.goal_section.as_deref() {
            let t = section.trim();
            if !t.is_empty() {
                sections.push(t.to_string());
            }
        }
        if let Some(section) = relevant_memories_context_section(memories) {
            sections.push(section);
        }
        if !self.prompt_config.prompt_fragments.is_empty() {
            sections.push(self.prompt_config.prompt_fragments.join("\n\n"));
        }
        sections.extend(
            extra_sections
                .iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
        );
        sections
    }

    pub fn new(
        core: RuntimeCoreDeps,
        memory: RuntimeMemoryOptions,
        tool_policy: RuntimeToolPolicy,
    ) -> Self {
        let RuntimeCoreDeps {
            llm_client,
            tools,
            memory_store,
            default_model_config,
            model_overrides,
            failover_chain,
            disk_output,
            security,
            sandbox_mode,
            prompt_config,
        } = core;

        let RuntimeMemoryOptions {
            memory_pipeline,
            memory_pipeline_settings,
            memory_project_autosave_enabled,
            session_notifications,
        } = memory;

        let RuntimeToolPolicy {
            tool_name_deny,
            claude_gating,
            expose_skill_on_explore_plan,
        } = tool_policy;

        let mut agents = HashMap::new();

        // 注册内置 agents
        let gp_agent =
            Box::new(GeneralPurposeAgent::new(default_model_config.clone())) as Box<dyn Agent>;
        agents.insert(AgentType::new("general-purpose"), gp_agent);

        let explore_agent = Box::new(ExploreAgent::new(
            default_model_config.clone(),
            expose_skill_on_explore_plan,
        )) as Box<dyn Agent>;
        agents.insert(AgentType::new("explore"), explore_agent);

        let plan_agent = Box::new(PlanAgent::new(
            default_model_config.clone(),
            expose_skill_on_explore_plan,
        )) as Box<dyn Agent>;
        agents.insert(AgentType::new("plan"), plan_agent);

        let workspace_agent = Box::new(WorkspaceAssistantAgent::new(
            default_model_config.clone(),
            expose_skill_on_explore_plan,
        )) as Box<dyn Agent>;
        agents.insert(AgentType::new("workspace-assistant"), workspace_agent);

        let goal_agent = Box::new(GoalAgent::new(default_model_config.clone())) as Box<dyn Agent>;
        agents.insert(AgentType::new("goal"), goal_agent);

        Self {
            agents: Arc::new(RwLock::new(agents)),
            llm_client,
            tools: Arc::new(RwLock::new(tools)),
            memory_store,
            memory_pipeline,
            memory_pipeline_settings,
            session_notifications,
            default_model_config,
            model_overrides,
            failover_chain,
            disk_output,
            security,
            sandbox_mode,
            prompt_config,
            memory_project_autosave_enabled,
            tool_name_deny,
            claude_gating,
            compaction_hooks: Arc::new(DefaultCompactionHooks::new()),
            auto_compact: false,
            auto_compact_policy: CompactPolicy::default(),
            session_context_window_auto: true,
            session_context_window_tokens: 0,
            tool_services: StdMutex::new(None),
            completion_guard: Arc::new(completion_guard::CompletionGuard::new(
                Arc::new(anycode_tools::ValidatorRegistry::new()),
                completion_guard::CompletionGuardPolicy::default(),
            )),
        }
    }

    #[must_use]
    pub fn with_auto_compact(mut self, enabled: bool, policy: CompactPolicy) -> Self {
        self.auto_compact = enabled;
        self.auto_compact_policy = policy;
        self
    }

    #[must_use]
    pub fn with_session_context(
        mut self,
        context_window_auto: bool,
        context_window_tokens: u32,
    ) -> Self {
        self.session_context_window_auto = context_window_auto;
        self.session_context_window_tokens = context_window_tokens;
        self
    }

    pub(super) fn effective_context_window_tokens(&self, model: &ModelConfig) -> u32 {
        if !self.session_context_window_auto && self.session_context_window_tokens > 0 {
            self.session_context_window_tokens
        } else {
            anycode_llm::capabilities_for_model_config(model).context_tokens
        }
    }

    pub fn attach_tool_services(&self, services: Arc<anycode_tools::ToolServices>) {
        if let Ok(mut g) = self.tool_services.lock() {
            *g = Some(services);
        }
    }

    pub(super) async fn chat_with_failover(
        &self,
        messages: &[Message],
        tools: Vec<ToolSchema>,
        primary: &ModelConfig,
        task_id: TaskId,
        logger: &RunLogger,
    ) -> Result<LLMResponse, CoreError> {
        let messages = crate::reply_language::inject_ephemeral_reply_language_reminder(messages);
        if self.failover_chain.is_empty() {
            return self.llm_client.chat(messages, tools, primary).await;
        }
        match self
            .llm_client
            .chat(messages.clone(), tools.clone(), primary)
            .await
        {
            Ok(r) => Ok(r),
            Err(e) => {
                let mut last_err = e;
                for policy in &self.failover_chain {
                    if !failover::error_triggers_failover(&last_err, policy.trigger) {
                        return Err(last_err);
                    }
                    logger.line(
                        task_id,
                        &format!(
                            "[model_failover] from={}/{} to={}/{} reason={}",
                            Self::provider_label(primary),
                            primary.model,
                            Self::provider_label(&policy.fallback),
                            policy.fallback.model,
                            last_err
                        ),
                    );
                    match self
                        .llm_client
                        .chat(messages.clone(), tools.clone(), &policy.fallback)
                        .await
                    {
                        Ok(r) => return Ok(r),
                        Err(next) => last_err = next,
                    }
                }
                Err(last_err)
            }
        }
    }

    pub(super) async fn try_failover_on_provider_body_error(
        &self,
        messages: &[Message],
        tools: Vec<ToolSchema>,
        primary: &ModelConfig,
        task_id: TaskId,
        logger: &RunLogger,
        err: &str,
    ) -> Result<Option<LLMResponse>, CoreError> {
        if self.failover_chain.is_empty() {
            return Ok(None);
        }
        let synthetic = CoreError::LLMError(err.to_string());
        let mut last_err = synthetic;
        for policy in &self.failover_chain {
            if !failover::error_triggers_failover(&last_err, policy.trigger) {
                return Ok(None);
            }
            logger.line(
                task_id,
                &format!(
                    "[model_failover] stream_error from={}/{} to={}/{}",
                    Self::provider_label(primary),
                    primary.model,
                    Self::provider_label(&policy.fallback),
                    policy.fallback.model
                ),
            );
            match self
                .llm_client
                .chat(messages.to_vec(), tools.clone(), &policy.fallback)
                .await
            {
                Ok(r) => return Ok(Some(r)),
                Err(next) => last_err = next,
            }
        }
        Err(last_err)
    }

    fn provider_label(cfg: &ModelConfig) -> String {
        match &cfg.provider {
            LLMProvider::Custom(s) => s.clone(),
            LLMProvider::Anthropic => "anthropic".into(),
            LLMProvider::OpenAI => "openai".into(),
            LLMProvider::Local => "local".into(),
        }
    }

    /// 将记忆管线的易失层（如虚态缓冲 WAL）刷盘。进程正常退出时 pipeline 也会在 drop 时 best-effort 刷盘。
    pub fn sync_memory_durability(&self) {
        if let Some(ref pipe) = self.memory_pipeline {
            if let Err(e) = pipe.sync_durability() {
                warn!(target: "anycode_agent", "memory pipeline durability sync: {}", e);
            }
        }
    }

    fn log_task_line(&self, task_id: TaskId, line: &str) {
        if let Some(out) = &self.disk_output {
            let _ = out.append_line(task_id, line);
        }
    }

    fn logger(&self) -> RunLogger {
        RunLogger::new(self.disk_output.clone())
    }

    fn model_for_task(&self, agent_type: &AgentType) -> &ModelConfig {
        let canonical = canonical_agent_type(agent_type);
        self.model_overrides
            .get(agent_type)
            .or_else(|| self.model_overrides.get(&canonical))
            .unwrap_or(&self.default_model_config)
    }

    fn model_for_summary(&self) -> &ModelConfig {
        // 优先 routing.agents.summary，其次复用 plan，再回退 default
        self.model_overrides
            .get(&AgentType::new("summary"))
            .or_else(|| self.model_overrides.get(&AgentType::new("plan")))
            .unwrap_or(&self.default_model_config)
    }

    /// 构建 TUI 会话使用的初始 `system` 消息（不注入 memory，避免引入额外不确定性）。
    pub async fn build_system_message(
        &self,
        agent_type: &AgentType,
        working_directory: &str,
    ) -> Result<Message, CoreError> {
        let agents = self.agents.read().await;
        let canonical = canonical_agent_type(agent_type);
        let agent = agents
            .get(&canonical)
            .or_else(|| agents.get(agent_type))
            .ok_or_else(|| CoreError::AgentNotFound(Uuid::new_v4()))?;

        let prompt = self.build_system_prompt(agent, working_directory, None)?;

        Ok(Message {
            id: Uuid::new_v4(),
            role: MessageRole::System,
            content: MessageContent::Text(prompt),
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        })
    }

    /// 构建 TUI 会话初始消息（system + 上下文状态消息；不注入 memory）。
    pub async fn build_session_messages(
        &self,
        agent_type: &AgentType,
        working_directory: &str,
    ) -> Result<Vec<Message>, CoreError> {
        let system = self
            .build_system_message(agent_type, working_directory)
            .await?;
        let mode = {
            let agents = self.agents.read().await;
            let canonical = canonical_agent_type(agent_type);
            let agent = agents
                .get(&canonical)
                .or_else(|| agents.get(agent_type))
                .ok_or_else(|| CoreError::AgentNotFound(Uuid::new_v4()))?;
            agent.runtime_mode()
        };
        let mut messages = vec![system];
        messages.extend(
            self.context_messages_from_sections(self.build_context_sections(mode, &[], &[])),
        );
        Ok(messages)
    }

    /// 注册自定义 Agent
    pub async fn register_agent(&self, agent: Box<dyn Agent>) {
        let mut agents = self.agents.write().await;
        agents.insert(agent.agent_type().clone(), agent);
    }

    fn build_system_prompt(
        &self,
        agent: &Box<dyn Agent>,
        working_directory: &str,
        task_append: Option<&str>,
    ) -> Result<String, CoreError> {
        Ok(compose_effective_system_prompt(
            &self.prompt_config,
            agent.as_ref(),
            working_directory,
            task_append,
        ))
    }
}
