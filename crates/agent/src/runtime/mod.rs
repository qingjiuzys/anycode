//! Agent 运行时（LLM + 工具循环、落盘、回执）。

#[cfg(feature = "harness-v1")]
pub mod harness_bridge;
#[cfg(feature = "harness-v1")]
mod harness_tools;
#[cfg(feature = "harness-v1")]
pub use harness_tools::{ComputerTicket, HubEnterGuard};
#[cfg(feature = "harness-v1")]
pub(crate) mod harness_unified;

mod agentic_loop;
mod agentic_turn;
mod artifacts;
mod automem;
mod budget;
mod compile_context;
mod completion_guard;
mod delivery_acceptance;
pub mod delivery_metrics;
mod discoverable_verification;
mod evidence;
mod execute_eval;
mod execute_goal;
mod execute_task;
mod execute_tool;
mod execute_turn;
mod execute_turn_finalize;
pub mod failover;
mod family_fallback;
mod grader;
mod guard_verdict;
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
mod sandbox_escape_nudge;
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
mod tool_result_render;
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
    /// auto-memory（LLM 驱动提取/巩固）配置；`None` 表示未启用或回退本地规则引擎。
    automem: Option<anycode_core::AutomemSettings>,
    /// auto-memory 根路径（`{base}/projects/{key}/memory/` 的 `{base}`）。
    automem_base: std::path::PathBuf,
    /// automem fork 任务的输入级工具门控：task_id → memory dir。
    automem_gates: Arc<StdMutex<HashMap<TaskId, std::path::PathBuf>>>,
    /// 后台 fork 需要的自引用（bootstrap 构造后 `attach_self`）。
    self_weak: StdMutex<Option<std::sync::Weak<AgentRuntime>>>,
    /// When true, execute_task / execute_turn are thin Kernel adapters (harness-v1).
    harness_unified_kernel: bool,
    #[cfg(feature = "harness-v1")]
    pub(crate) harness_hub: Arc<harness_tools::HarnessHub>,
}

fn canonical_agent_type(agent_type: &AgentType) -> AgentType {
    AgentType::new(crate::agent_profiles::normalize_agent_id(
        agent_type.as_str(),
    ))
}

pub(super) struct ParentToolSurfaceGuard {
    services: Arc<anycode_tools::ToolServices>,
    task_id: uuid::Uuid,
    previous: Option<(Vec<String>, Vec<String>)>,
    previous_budget: Option<anycode_core::TaskBudget>,
}

impl ParentToolSurfaceGuard {
    /// Record this task's deny list and budget so nested Agent inherits them.
    pub(super) fn attach(
        runtime: &AgentRuntime,
        task_id: uuid::Uuid,
        deny_names: Vec<String>,
        deny_prefixes: Vec<String>,
        budget: anycode_core::TaskBudget,
    ) -> Option<Self> {
        let services = runtime
            .tool_services
            .lock()
            .ok()
            .and_then(|g| g.as_ref().cloned())?;
        services.ensure_sub_agent_lineage(task_id);
        let previous = services.set_parent_task_tool_deny(task_id, deny_names, deny_prefixes);
        let previous_budget = services.set_task_budget(task_id, budget);
        Some(Self {
            services,
            task_id,
            previous,
            previous_budget,
        })
    }
}

impl Drop for ParentToolSurfaceGuard {
    fn drop(&mut self) {
        // Restore this task's surface only. Concurrent parents keep their own keys.
        self.services
            .restore_parent_task_tool_deny(self.task_id, self.previous.take());
        self.services
            .restore_task_budget(self.task_id, self.previous_budget.take());
    }
}

/// Step 3b：任务级 live trace 通道的注册守卫（`execute_task` / `execute_turn`
/// 共用）；drop 时从 ToolServices 键控 map 注销，避免任务结束后残留通道被
/// 嵌套工具误接线。
pub(super) struct LiveTraceRegistrationGuard {
    services: Arc<anycode_tools::ToolServices>,
    task_id: uuid::Uuid,
}

impl LiveTraceRegistrationGuard {
    /// 注册任务 live trace 通道；任一前置缺失（无 services / 无通道）时返回 None。
    pub(super) fn register(
        runtime: &AgentRuntime,
        task_id: uuid::Uuid,
        tx: Option<tokio::sync::mpsc::UnboundedSender<anycode_core::LiveTraceEvent>>,
    ) -> Option<Self> {
        let svc = runtime
            .tool_services
            .lock()
            .ok()
            .and_then(|g| g.as_ref().cloned())?;
        let tx = tx?;
        svc.set_live_trace_tx(task_id, tx);
        Some(Self {
            services: svc,
            task_id,
        })
    }
}

impl Drop for LiveTraceRegistrationGuard {
    fn drop(&mut self) {
        self.services.remove_live_trace_tx(self.task_id);
    }
}

/// 构造一条注入上下文的 user 消息（`ANYCODE_CONTEXT_USER_METADATA_KEY=true`），
/// 供编译上下文注入、修复请求、evidence repair 等所有"系统侧 user 消息"统一使用。
pub(super) fn context_user_message(text: String) -> Message {
    let mut metadata = HashMap::new();
    metadata.insert(
        ANYCODE_CONTEXT_USER_METADATA_KEY.to_string(),
        serde_json::Value::Bool(true),
    );
    Message {
        id: Uuid::new_v4(),
        role: MessageRole::User,
        content: MessageContent::Text(text),
        timestamp: chrono::Utc::now(),
        metadata,
    }
}

impl AgentRuntime {
    fn context_messages_from_sections(&self, sections: Vec<String>) -> Vec<Message> {
        sections
            .into_iter()
            .filter(|section| !section.trim().is_empty())
            .map(context_user_message)
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
            automem,
            automem_base_path,
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

        // auto-memory 后台 fork（提取 / 巩固）：复用 GeneralPurposeAgent，
        // 工具面由 automem 白名单 + 输入级门控收紧。
        for automem_type in [
            automem::AUTOMEM_EXTRACT_AGENT_TYPE,
            automem::AUTOMEM_DREAM_AGENT_TYPE,
        ] {
            agents.insert(
                AgentType::new(automem_type),
                Box::new(GeneralPurposeAgent::new(default_model_config.clone())) as Box<dyn Agent>,
            );
        }

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
            automem,
            automem_base: automem_base_path
                .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".anycode")),
            automem_gates: Arc::new(StdMutex::new(HashMap::new())),
            self_weak: StdMutex::new(None),
            harness_unified_kernel: false,
            #[cfg(feature = "harness-v1")]
            harness_hub: Arc::new(harness_tools::HarnessHub::default()),
        }
    }

    /// 供后台 auto-memory fork 借用 `Arc<AgentRuntime>`（bootstrap 在 `Arc::new` 后调用一次）。
    pub fn attach_self(self: &Arc<Self>) {
        if let Ok(mut g) = self.self_weak.lock() {
            *g = Some(Arc::downgrade(self));
        }
    }

    /// Sandboxed Host/graph helper. The caller supplies the LLM; this does not
    /// mint provider usage or skip `execute_tool_call`.
    pub fn sandboxed_scripted(
        llm: Arc<dyn LLMClient>,
        extra: HashMap<ToolName, Box<dyn Tool>>,
    ) -> Arc<Self> {
        struct NoopMemory;
        #[async_trait::async_trait]
        impl MemoryStore for NoopMemory {
            async fn save(&self, _memory: Memory) -> Result<(), CoreError> {
                Ok(())
            }
            async fn recall(
                &self,
                _query: &str,
                _mem_type: MemoryType,
            ) -> Result<Vec<Memory>, CoreError> {
                Ok(vec![])
            }
            async fn update(&self, _id: &str, _memory: Memory) -> Result<(), CoreError> {
                Ok(())
            }
            async fn delete(&self, _id: &str) -> Result<(), CoreError> {
                Ok(())
            }
        }

        let mut tools = extra;
        tools
            .entry("FileRead".into())
            .or_insert_with(|| Box::new(anycode_tools::FileReadTool::new(true)));
        let runtime = Arc::new(Self::new(
            RuntimeCoreDeps {
                llm_client: llm,
                tools,
                memory_store: Arc::new(NoopMemory),
                default_model_config: ModelConfig {
                    provider: LLMProvider::Custom("scripted".into()),
                    model: "scripted-native".into(),
                    ..Default::default()
                },
                model_overrides: HashMap::new(),
                failover_chain: vec![],
                disk_output: None,
                security: Arc::new(SecurityLayer::new(PermissionMode::BypassPermissions)),
                sandbox_mode: true,
                prompt_config: RuntimePromptConfig::default(),
            },
            RuntimeMemoryOptions {
                memory_pipeline: None,
                memory_pipeline_settings: None,
                memory_project_autosave_enabled: false,
                session_notifications: None,
                automem: None,
                automem_base_path: None,
            },
            RuntimeToolPolicy {
                tool_name_deny: vec![],
                claude_gating: AgentClaudeToolGating::default(),
                expose_skill_on_explore_plan: false,
            },
        ));
        runtime.attach_self();
        runtime
    }

    pub async fn attach_harness_tools(self: &Arc<Self>) {
        #[cfg(feature = "harness-v1")]
        harness_tools::register_harness_tools(self).await;
    }

    /// Host-only dispatch through the same security pipeline Kernel uses.
    /// Outside this crate, `execute_tool_call` stays crate-private.
    #[cfg(feature = "harness-v1")]
    pub async fn host_execute_tool_call(
        &self,
        task_id: anycode_core::TaskId,
        agent_type: &anycode_core::AgentType,
        working_directory: &str,
        tool_call: &anycode_core::ToolCall,
    ) -> Result<anycode_core::ToolOutput, anycode_core::CoreError> {
        self.execute_tool_call(task_id, agent_type, working_directory, tool_call)
            .await
    }

    /// Host-only computer lease. The caller must already hold a live
    /// `RunContext` whose scope includes `device`. Observe/Act still fail
    /// closed until a backend is enrolled.
    #[cfg(feature = "harness-v1")]
    pub fn set_harness_write_isolation(&self, isolation: harness_bridge::WriteIsolation) {
        self.harness_hub.set_write_isolation(isolation);
    }

    #[cfg(feature = "harness-v1")]
    pub fn enroll_macos_screencapture(
        &self,
        device: uuid::Uuid,
        private_cwd: &std::path::Path,
    ) -> Result<(), anycode_harness_core::Error> {
        let backend = anycode_harness_extensions::computer::MacScreenCaptureBackend::host_default(
            private_cwd.to_path_buf(),
        )?;
        self.harness_hub.enroll_computer(
            device,
            "macos-screencapture",
            std::sync::Arc::new(backend),
        )
    }

    #[cfg(feature = "harness-v1")]
    pub fn computer_backend_kind(&self) -> Option<String> {
        self.harness_hub.computer_backend_kind()
    }

    #[cfg(feature = "harness-v1")]
    pub fn bind_harness_product_run(
        &self,
        run: uuid::Uuid,
    ) -> Result<(), anycode_harness_core::Error> {
        self.harness_hub.bind_product_run(run)
    }

    #[cfg(feature = "harness-v1")]
    pub fn unbind_harness_product_run(&self, run: uuid::Uuid) {
        self.harness_hub.unbind_product_run(run);
    }

    /// Bind harness tools to this Host run. Drop the guard to leave.
    #[cfg(feature = "harness-v1")]
    pub fn enter_harness_run(
        &self,
        ctx: anycode_harness_core::RunContext,
        working_directory: &std::path::Path,
    ) -> harness_tools::HubEnterGuard {
        self.harness_hub.enter(ctx, working_directory.to_path_buf())
    }

    #[cfg(feature = "harness-v1")]
    fn with_harness_run<T>(
        &self,
        ctx: &anycode_harness_core::RunContext,
        working_directory: &std::path::Path,
        f: impl FnOnce() -> T,
    ) -> T {
        let _guard = self.enter_harness_run(ctx.clone(), working_directory);
        f()
    }

    /// HOST ONLY. The model cannot mint this approval or pass an ApprovalTicket.
    #[cfg(feature = "harness-v1")]
    pub fn approve_harness_computer_action(
        &self,
        ctx: &anycode_harness_core::RunContext,
        working_directory: &std::path::Path,
        frame_id: uuid::Uuid,
        action: anycode_harness_extensions::computer::Action,
    ) -> Result<(), anycode_harness_core::Error> {
        self.with_harness_run(ctx, working_directory, || {
            self.harness_hub.approve_computer_action(frame_id, action)
        })
    }

    #[cfg(feature = "harness-v1")]
    pub fn issue_harness_computer_ticket(
        &self,
        ctx: &anycode_harness_core::RunContext,
        working_directory: &std::path::Path,
        device: uuid::Uuid,
        ttl_secs: u64,
    ) -> Result<uuid::Uuid, anycode_harness_core::Error> {
        self.with_harness_run(ctx, working_directory, || {
            Ok(self
                .harness_hub
                .issue_computer_ticket(device, ttl_secs)?
                .ticket)
        })
    }

    #[cfg(feature = "harness-v1")]
    pub fn issue_harness_computer_ticket_for_run(
        &self,
        ctx: &anycode_harness_core::RunContext,
        working_directory: &std::path::Path,
        device: uuid::Uuid,
        ttl_secs: u64,
        run: uuid::Uuid,
    ) -> Result<harness_tools::ComputerTicket, anycode_harness_core::Error> {
        self.with_harness_run(ctx, working_directory, || {
            self.harness_hub
                .issue_computer_ticket_for_run(device, ttl_secs, run)
        })
    }

    #[must_use]
    pub fn with_auto_compact(mut self, enabled: bool, policy: CompactPolicy) -> Self {
        self.auto_compact = enabled;
        self.auto_compact_policy = policy;
        self
    }

    #[must_use]
    pub fn with_harness_unified_kernel(mut self, enabled: bool) -> Self {
        self.harness_unified_kernel = enabled;
        self
    }

    #[must_use]
    pub fn harness_unified_kernel_enabled(&self) -> bool {
        self.harness_unified_kernel
    }

    #[must_use]
    pub fn sandbox_mode(&self) -> bool {
        self.sandbox_mode
    }

    #[cfg(feature = "harness-v1")]
    pub(super) fn upgraded_self(&self) -> Result<Arc<Self>, CoreError> {
        self.self_weak
            .lock()
            .ok()
            .and_then(|g| g.as_ref()?.upgrade())
            .ok_or_else(|| {
                CoreError::Other(anyhow::anyhow!(
                    "harness unified kernel requires AgentRuntime::attach_self"
                ))
            })
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

    pub fn tool_services(&self) -> Option<Arc<anycode_tools::ToolServices>> {
        self.tool_services.lock().ok().and_then(|g| g.clone())
    }

    pub fn attach_tool_services(&self, services: Arc<anycode_tools::ToolServices>) {
        if self.harness_unified_kernel {
            services.set_forbid_detached_nested(true);
        }
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
        let mut messages = messages.to_vec();
        let _ = crate::compact::prepare_messages_for_llm_hop(&mut messages);
        let messages = crate::reply_language::inject_ephemeral_reply_language_reminder(&messages);
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

    pub(super) fn logger(&self) -> RunLogger {
        RunLogger::new(self.disk_output.clone())
    }

    pub(super) fn model_for_task(&self, agent_type: &AgentType) -> &ModelConfig {
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

    /// 已注册 agents 的（id, description）摘要：过滤 automem 内部 fork、按 id 排序、上限
    /// `MAX_AGENT_CATALOG_ENTRIES`。供 `Agent`/`Task` 工具面暴露给父模型（可发现性）。
    pub async fn list_agent_summaries(&self) -> Vec<(String, String)> {
        let agents = self.agents.read().await;
        summaries_from_agents(&agents)
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

/// 子代理目录条目上限（防 schema 膨胀，弱本地模型友好）。
pub(crate) const MAX_AGENT_CATALOG_ENTRIES: usize = 64;

/// 从 agents 注册表生成（id, description）摘要：过滤 automem 内部 fork、按 id 排序、
/// 上限 [`MAX_AGENT_CATALOG_ENTRIES`]。`list_agent_summaries`（异步读锁）与
/// `SubAgentExecutor::agent_catalog`（同步 `try_read`）共用。
pub(crate) fn summaries_from_agents(
    agents: &HashMap<AgentType, Box<dyn Agent>>,
) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = agents
        .values()
        .filter(|a| !automem::is_automem_agent_type(a.agent_type().as_str()))
        .map(|a| {
            (
                a.agent_type().as_str().to_string(),
                a.description().to_string(),
            )
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out.truncate(MAX_AGENT_CATALOG_ENTRIES);
    out
}

#[cfg(test)]
mod agent_catalog_tests {
    use super::*;

    struct StubAgent {
        agent_type: AgentType,
        description: &'static str,
    }

    #[async_trait::async_trait]
    impl Agent for StubAgent {
        fn agent_type(&self) -> &AgentType {
            &self.agent_type
        }
        fn description(&self) -> &str {
            self.description
        }
        fn tools(&self) -> Vec<ToolName> {
            vec![]
        }
        async fn execute(&mut self, _task: Task) -> Result<TaskResult, CoreError> {
            Ok(TaskResult::Failure {
                error: "unused".into(),
                details: None,
            })
        }
    }

    #[test]
    fn summaries_filter_automem_sort() {
        let mut agents: HashMap<AgentType, Box<dyn Agent>> = HashMap::new();
        for (id, desc) in [
            ("plan", "Plan agent"),
            ("explore", "Explore agent"),
            ("automem-extract", "internal extract fork"),
            ("automem-dream", "internal dream fork"),
            ("sql-reviewer", "Reviews SQL"),
        ] {
            agents.insert(
                AgentType::new(id),
                Box::new(StubAgent {
                    agent_type: AgentType::new(id),
                    description: desc,
                }),
            );
        }
        let out = summaries_from_agents(&agents);
        let ids: Vec<&str> = out.iter().map(|(id, _)| id.as_str()).collect();
        assert_eq!(ids, vec!["explore", "plan", "sql-reviewer"]);
    }

    #[test]
    fn summaries_capped_at_max_entries() {
        let mut agents: HashMap<AgentType, Box<dyn Agent>> = HashMap::new();
        for i in 0..(MAX_AGENT_CATALOG_ENTRIES + 8) {
            let id = format!("agent-{i:03}");
            agents.insert(
                AgentType::new(&id),
                Box::new(StubAgent {
                    agent_type: AgentType::new(&id),
                    description: "d",
                }),
            );
        }
        let out = summaries_from_agents(&agents);
        assert_eq!(out.len(), MAX_AGENT_CATALOG_ENTRIES);
    }
}
