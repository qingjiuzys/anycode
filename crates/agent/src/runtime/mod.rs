//! Agent 运行时（LLM + 工具循环、落盘、回执）。

mod artifacts;
mod limits;
mod logging;
mod nested_worktree;
mod receipt;
mod session;
mod session_notify;
mod task_summary;
mod tool_gating;
mod tool_surface;

mod runtime_options;
pub use runtime_options::{RuntimeCoreDeps, RuntimeMemoryOptions, RuntimeToolPolicy};
pub use tool_gating::AgentClaudeToolGating;

use crate::compact::{CompactionHooks, DefaultCompactionHooks};
use crate::goal_engine::GoalEngine;
use crate::prompt_assembler::{
    relevant_memories_context_section, runtime_mode_context_section, slash_commands_context_section,
};
use crate::system_prompt::{compose_effective_system_prompt, RuntimePromptConfig};
use crate::{ExploreAgent, GeneralPurposeAgent, GoalAgent, PlanAgent, WorkspaceAssistantAgent};
use anycode_core::prelude::*;
use anycode_core::strip_llm_reasoning_xml_blocks;
use anycode_core::Artifact;
use anycode_core::{
    MemoryPipeline, MemoryPipelineSettings, SessionNotificationSettings,
    NESTED_TASK_COOPERATIVE_CANCEL_ERROR,
};
use anycode_security::SecurityLayer;
use artifacts::{extract_artifacts, truncate_text};
use async_trait::async_trait;
use limits::{
    MAX_AGENT_TURNS, MAX_TOOL_CALLS_TOTAL, TOOL_INPUT_LOG_MAX_BYTES, TOOL_RESULT_MAX_BYTES,
};
use logging::RunLogger;
use nested_worktree::NestedWorktreeGuard;
use receipt::ReceiptGenerator;
use regex::Regex;
use session_notify::{build_notification_value, spawn_dispatch};
use std::collections::HashMap;
use std::future::pending;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use task_summary::{last_assistant_plain_text, llm_summary_receipt};
use tokio::sync::{Mutex, RwLock};
use tracing::warn;
use uuid::Uuid;

fn nested_coop_cancelled(ctx: &TaskContext) -> bool {
    ctx.nested_cancel
        .as_ref()
        .is_some_and(|b| b.load(Ordering::Acquire))
}

fn opt_coop_cancelled(flag: &Option<Arc<AtomicBool>>) -> bool {
    flag.as_ref().is_some_and(|b| b.load(Ordering::Acquire))
}

fn channel_progress_send(ctx: &TaskContext, line: String) {
    if let Some(tx) = &ctx.channel_progress_tx {
        let _ = tx.send(line);
    }
}

fn task_cancelled_failure() -> TaskResult {
    TaskResult::Failure {
        error: NESTED_TASK_COOPERATIVE_CANCEL_ERROR.to_string(),
        details: Some("cooperative nested cancel".to_string()),
    }
}

/// Short-interval polling so `tokio::select!` can abort in-flight LLM I/O without `tokio-util`.
const COOP_CANCEL_POLL_INTERVAL: Duration = Duration::from_millis(20);

async fn coop_flag_wait(flag: Arc<AtomicBool>) {
    loop {
        if flag.load(Ordering::Acquire) {
            return;
        }
        tokio::time::sleep(COOP_CANCEL_POLL_INTERVAL).await;
    }
}

/// For `select!`: when `flag` is `None`, this future never completes (LLM branch always runs).
async fn coop_flag_wait_opt(flag: Option<Arc<AtomicBool>>) {
    match flag {
        Some(f) => coop_flag_wait(f).await,
        None => pending().await,
    }
}

async fn pop_assistant_placeholder(messages: &Arc<Mutex<Vec<Message>>>, assistant_id: Uuid) {
    let mut g = messages.lock().await;
    if g.last().is_some_and(|m| m.id == assistant_id) {
        g.pop();
    }
}

const MEMORY_AUTOSAVE_TITLE_MAX_CHARS: usize = 200;
const MEMORY_AUTOSAVE_CONTENT_MAX_BYTES: usize = 64 * 1024;

fn estimate_input_tokens_for_messages(messages: &[Message]) -> u32 {
    // Conservative fallback for streaming providers that do not report usage yet.
    // 4 chars/token is a common rough estimate for mixed English/JSON payloads.
    let chars: usize = messages
        .iter()
        .map(|m| match &m.content {
            MessageContent::Text(t) => t.chars().count(),
            MessageContent::ToolResult { content, .. } => content.chars().count(),
            _ => 0,
        })
        .sum();
    ((chars as u32).saturating_add(3)) / 4
}

fn last_user_plain_text_for_autosave(msgs: &[Message]) -> String {
    msgs.iter()
        .rev()
        .find_map(|m| {
            if m.role == MessageRole::User {
                match &m.content {
                    MessageContent::Text(t) if !t.trim().is_empty() => Some(t.clone()),
                    _ => None,
                }
            } else {
                None
            }
        })
        .unwrap_or_default()
}

/// 顶层 `"error": "…"` 为字符串时，排除助手正文里举例用的短 JSON（如 `{"error":"demo"}`）。
fn provider_error_string_smells_like_api(s: &str) -> bool {
    const NEEDLES: &[&str] = &[
        "User location",
        "not supported for the API",
        "FAILED_PRECONDITION",
        "generativelanguage",
        "Incorrect API key",
        "invalid_api_key",
        "invalid request",
        "rate limit",
        "quota",
        "exceeded",
    ];
    NEEDLES.iter().any(|n| s.contains(n))
}

/// OpenAI/Gemini 兼容：HTTP 200 但正文是 error JSON（流式 delta 或非流式 `choices.message.content`）。
/// 不识别时 assistant 会留下整段 JSON，流式 REPL 主区与 `Turn failed` 叠成两份。
fn summary_from_parsed_provider_error_value(err_body: &serde_json::Value) -> Option<String> {
    let err = err_body.get("error")?;
    match err {
        serde_json::Value::String(s) => {
            let s = s.trim();
            if s.is_empty() {
                None
            } else if provider_error_string_smells_like_api(s) {
                Some(s.to_string())
            } else {
                None
            }
        }
        serde_json::Value::Object(_) => {
            let msg = err
                .get("message")
                .and_then(|m| m.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty());
            let status = err.get("status").and_then(|s| s.as_str());
            let code = err.get("code");
            let looks_structured = msg.is_some()
                || status.is_some()
                || matches!(code, Some(serde_json::Value::Number(_)));
            if !looks_structured {
                return None;
            }
            Some(
                msg.map(|s| s.to_string())
                    .or_else(|| status.map(|s| s.to_string()))
                    .unwrap_or_else(|| "provider error object".to_string()),
            )
        }
        _ => None,
    }
}

fn try_parse_provider_error_top_json(t: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(t).ok()?;
    let err_body = match &v {
        serde_json::Value::Array(a) => a.first()?,
        serde_json::Value::Object(_) => &v,
        _ => return None,
    };
    summary_from_parsed_provider_error_value(err_body)
}

/// 去掉 BOM 后再解析（不跳过正文中的 `{`，避免把举例 JSON 当成接口错误）。
fn try_parse_provider_error_after_bom(t: &str) -> Option<String> {
    let u = t.trim().trim_start_matches('\u{feff}');
    try_parse_provider_error_top_json(u)
}

/// 明显是接口错误 JSON、但无法整段 `serde_json::parse` 时的兜底（半段、转义异常等）。
fn heuristic_openai_compat_error_blob(t: &str) -> bool {
    let u = t.trim().trim_start_matches('\u{feff}');
    if u.len() < 25 || u.len() > 512 * 1024 {
        return false;
    }
    if u.contains("generativelanguage.googleapis.com") {
        return true;
    }
    if u.contains("googleapis.com") && u.contains("\"error\"") {
        return true;
    }
    if !(u.starts_with('{') || u.starts_with('[')) {
        return false;
    }
    if !u.contains("\"error\"") {
        return false;
    }
    u.contains("User location is not supported")
        || u.contains("FAILED_PRECONDITION")
        || (u.contains("\"message\"") && u.contains("\"code\""))
}

fn provider_error_from_streamed_assistant_text(text: &str) -> Option<String> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }

    if let Some(s) = try_parse_provider_error_top_json(t) {
        return Some(format!("streamed provider error: {s}"));
    }
    if let Some(s) = try_parse_provider_error_after_bom(t) {
        return Some(format!("streamed provider error: {s}"));
    }

    if heuristic_openai_compat_error_blob(t) {
        return Some(
            "streamed provider error: response body looks like API error JSON (details on stderr)"
                .to_string(),
        );
    }

    if t.contains("\"error\"") && t.contains("User location is not supported") {
        return Some(format!(
            "streamed provider error: {}",
            t.chars().take(600).collect::<String>()
        ));
    }
    if t.contains("FAILED_PRECONDITION") && t.contains("\"error\"") {
        return Some(format!(
            "streamed provider error: {}",
            t.chars().take(600).collect::<String>()
        ));
    }
    None
}

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
            disk_output,
            security,
            sandbox_mode,
            prompt_config,
            memory_project_autosave_enabled,
            tool_name_deny,
            claude_gating,
            compaction_hooks: Arc::new(DefaultCompactionHooks::new()),
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

    async fn pipeline_memory_hook_tool_result(
        &self,
        session_label: &str,
        task_id: TaskId,
        tool_name: &str,
        tool_text: &str,
    ) {
        let Some(ref pipe) = self.memory_pipeline else {
            return;
        };
        let Some(ref s) = self.memory_pipeline_settings else {
            return;
        };
        if !s.hook_after_tool_result {
            return;
        }
        if s.hook_tool_deny_prefixes
            .iter()
            .any(|p| tool_name.starts_with(p.as_str()))
        {
            return;
        }
        let (body, _) = truncate_text(tool_text.to_string(), s.hook_max_bytes);
        let text = format!("[tool:{}]\n{}", tool_name, body);
        let sess = format!("{}:{}", session_label, task_id);
        if let Err(e) = pipe
            .ingest_fragment(&sess, &text, MemoryType::Project)
            .await
        {
            warn!(target: "anycode_agent", "memory pipeline hook (tool): {}", e);
        }
    }

    async fn pipeline_memory_hook_agent_turn(
        &self,
        session_label: &str,
        task_id: TaskId,
        turn: usize,
        assistant_excerpt: &str,
    ) {
        let Some(ref pipe) = self.memory_pipeline else {
            return;
        };
        let Some(ref s) = self.memory_pipeline_settings else {
            return;
        };
        if !s.hook_after_agent_turn {
            return;
        }
        let t = assistant_excerpt.trim();
        if t.is_empty() {
            return;
        }
        let (body, _) = truncate_text(t.to_string(), s.hook_max_bytes);
        let text = format!("[turn {}]\n{}", turn, body);
        let sess = format!("{}:{}", session_label, task_id);
        if let Err(e) = pipe
            .ingest_fragment(&sess, &text, MemoryType::Project)
            .await
        {
            warn!(target: "anycode_agent", "memory pipeline hook (turn): {}", e);
        }
    }

    fn maybe_session_notify_tool_result(
        &self,
        session_label: &str,
        task_id: TaskId,
        turn: usize,
        tool_name: &str,
        tool_text: &str,
        cwd: Option<&str>,
    ) {
        let Some(ref cfg) = self.session_notifications else {
            return;
        };
        if !cfg.after_tool_result || !cfg.is_configured() {
            return;
        }
        if cfg
            .tool_deny_prefixes
            .iter()
            .any(|p| tool_name.starts_with(p.as_str()))
        {
            return;
        }
        let payload = build_notification_value(
            "tool_result",
            session_label,
            task_id,
            turn,
            Some(tool_name),
            tool_text,
            cwd,
            cfg.max_body_bytes,
        );
        spawn_dispatch(cfg.clone(), payload);
    }

    fn maybe_session_notify_agent_turn(
        &self,
        session_label: &str,
        task_id: TaskId,
        turn: usize,
        assistant_excerpt: &str,
        cwd: Option<&str>,
    ) {
        let Some(ref cfg) = self.session_notifications else {
            return;
        };
        if !cfg.after_agent_turn || !cfg.is_configured() {
            return;
        }
        let t = assistant_excerpt.trim();
        if t.is_empty() {
            return;
        }
        let payload = build_notification_value(
            "agent_turn",
            session_label,
            task_id,
            turn,
            None,
            t,
            cwd,
            cfg.max_body_bytes,
        );
        spawn_dispatch(cfg.clone(), payload);
    }

    async fn maybe_autosave_memory(&self, task_id: TaskId, prompt: &str, output: &str) {
        if !self.memory_project_autosave_enabled {
            return;
        }
        let line0 = prompt.lines().next().unwrap_or("").trim();
        let title = if line0.chars().count() > MEMORY_AUTOSAVE_TITLE_MAX_CHARS {
            line0
                .chars()
                .take(MEMORY_AUTOSAVE_TITLE_MAX_CHARS)
                .collect::<String>()
        } else {
            line0.to_string()
        };
        let title = if title.is_empty() {
            "(empty prompt)".to_string()
        } else {
            title
        };
        let (content, _) = truncate_text(output.to_string(), MEMORY_AUTOSAVE_CONTENT_MAX_BYTES);
        if let Some(ref pipe) = self.memory_pipeline {
            let session = task_id.to_string();
            let text = format!("{}\n\n{}", title, content);
            if let Err(e) = pipe
                .ingest_fragment(&session, &text, MemoryType::Project)
                .await
            {
                warn!(target: "anycode_agent", "memory pipeline ingest (auto_save) failed: {}", e);
            }
            return;
        }
        let now = chrono::Utc::now();
        let memory = Memory {
            id: task_id.to_string(),
            mem_type: MemoryType::Project,
            title,
            content,
            tags: vec![],
            scope: MemoryScope::Project,
            created_at: now,
            updated_at: now,
        };
        if let Err(e) = self.memory_store.save(memory).await {
            warn!(target: "anycode_agent", "memory auto_save failed: {}", e);
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
        self.model_overrides
            .get(agent_type)
            .unwrap_or(&self.default_model_config)
    }

    fn model_for_summary(&self) -> &ModelConfig {
        // 优先 routing.agents.summary，其次复用 plan，再回退 default
        self.model_overrides
            .get(&AgentType::new("summary"))
            .or_else(|| self.model_overrides.get(&AgentType::new("plan")))
            .unwrap_or(&self.default_model_config)
    }

    fn runtime_mode_for_agent(agent_type: &AgentType) -> RuntimeMode {
        match agent_type.as_str() {
            "plan" => RuntimeMode::Plan,
            "explore" => RuntimeMode::Explore,
            "workspace-assistant" | "channel" => RuntimeMode::Channel,
            "goal" => RuntimeMode::Goal,
            _ => RuntimeMode::Code,
        }
    }

    /// 构建 TUI 会话使用的初始 `system` 消息（不注入 memory，避免引入额外不确定性）。
    pub async fn build_system_message(
        &self,
        agent_type: &AgentType,
        working_directory: &str,
    ) -> Result<Message, CoreError> {
        let agents = self.agents.read().await;
        let agent = agents
            .get(agent_type)
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
        let mode = Self::runtime_mode_for_agent(agent_type);
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

    /// 执行一次“连续会话”的 agentic turn：从传入的 `messages` 继续跑同一轮工具循环，
    /// 并在结束后返回 `TurnOutput`：最终 assistant 文本、artifacts、以及聚合的 `TurnTokenUsage`（max input、sum output、cache 累计等）。
    ///
    /// 关键点：
    /// - 不重建 system/user：由调用方（TUI）维护 messages 历史。
    /// - 工具循环与 `execute_task` 相同：assistant → tool_calls → 执行工具 → tool_result 回注。
    /// - 优先展示最终 assistant 文本；若收尾无可用正文，则退化为 summary 回执，避免 TUI 出现“无总结”空白。
    /// - `messages` 使用 `Arc<Mutex<_>>`：仅在快照/追加时短暂加锁，便于 UI 在 LLM/工具执行中读取增量。
    pub async fn execute_turn_from_messages(
        &self,
        task_id: TaskId,
        agent_type: &AgentType,
        messages: Arc<Mutex<Vec<Message>>>,
        working_directory: &str,
        coop_cancel: Option<Arc<AtomicBool>>,
    ) -> Result<TurnOutput, CoreError> {
        let logger = self.logger();
        logger.ensure_initialized(task_id);
        logger.line(
            task_id,
            &format!("[task_start] agent_type={}", agent_type.as_str()),
        );

        // 1) 工具名与 schema（与 `execute_task` 共用 tool_surface，避免漂移）
        let agent_tools = {
            let agents = self.agents.read().await;
            let agent = agents
                .get(agent_type)
                .ok_or_else(|| CoreError::AgentNotFound(Uuid::new_v4()))?;
            agent.tools()
        };

        let tools = self.tools.read().await;
        let raw = tool_surface::resolve_agent_tool_names(agent_type.as_str(), agent_tools, &tools);
        let names = tool_surface::prepare_tool_names_for_llm(
            raw,
            &self.tool_name_deny,
            &self.claude_gating,
        );
        let tool_schemas = tool_surface::build_tool_schemas(&names, &tools);
        drop(tools);

        // 2) agentic loop：保持与 execute_task 的语义一致
        let model_config = self.model_for_task(agent_type).clone();
        let mut total_tool_calls: usize = 0;
        let mut artifacts: Vec<Artifact> = vec![];
        let mut last_assistant_text = String::new();
        let mut turn_usage = TurnTokenUsage::default();

        for turn in 1..=MAX_AGENT_TURNS {
            logger.line(
                task_id,
                &format!("[turn_start] turn={}/{}", turn, MAX_AGENT_TURNS),
            );
            if opt_coop_cancelled(&coop_cancel) {
                logger.line(task_id, "[task_end] status=cancelled reason=cooperative");
                return Err(CoreError::CooperativeCancel);
            }
            logger.line(
                task_id,
                &format!(
                    "[llm_request_start] turn={} model={} base_url={}",
                    turn,
                    model_config.model,
                    model_config
                        .base_url
                        .clone()
                        .unwrap_or_else(|| "<default>".to_string())
                ),
            );

            let t0 = std::time::Instant::now();
            let messages_snapshot = {
                let g = messages.lock().await;
                g.clone()
            };
            // Prefer streaming: TUI can render deltas incrementally via shared `messages`.
            // Fallback to non-stream chat if streaming is not supported / fails.
            let mut tool_calls: Vec<ToolCall> = vec![];
            let mut streamed = false;
            let assistant_id = Uuid::new_v4();
            let mut stream_usage: Option<Usage> = None;

            // Insert an empty assistant message first so UI can show deltas as they arrive.
            {
                let mut g = messages.lock().await;
                g.push(Message {
                    id: assistant_id,
                    role: MessageRole::Assistant,
                    content: MessageContent::Text(String::new()),
                    timestamp: chrono::Utc::now(),
                    metadata: HashMap::new(),
                });
            }

            let stream_open = self.llm_client.chat_stream(
                messages_snapshot.clone(),
                tool_schemas.clone(),
                &model_config,
            );
            let stream_open = tokio::select! {
                biased;
                () = coop_flag_wait_opt(coop_cancel.clone()) => {
                    pop_assistant_placeholder(&messages, assistant_id).await;
                    logger.line(
                        task_id,
                        "[llm_response_end] status=cancelled reason=cooperative_in_flight",
                    );
                    logger.line(task_id, "[task_end] status=cancelled reason=cooperative");
                    return Err(CoreError::CooperativeCancel);
                }
                r = stream_open => r,
            };

            if let Ok(mut rx) = stream_open {
                streamed = true;
                let mut received_any = false;
                let mut stream_cancelled = false;
                loop {
                    tokio::select! {
                        biased;
                        () = coop_flag_wait_opt(coop_cancel.clone()) => {
                            stream_cancelled = true;
                            break;
                        }
                        ev = rx.recv() => {
                            match ev {
                                None => break,
                                Some(ev) => match ev {
                                    StreamEvent::Delta(d) => {
                                        if !d.is_empty() {
                                            received_any = true;
                                            let mut g = messages.lock().await;
                                            if let Some(last) = g.last_mut() {
                                                if last.id == assistant_id {
                                                    if let MessageContent::Text(t) = &mut last.content {
                                                        t.push_str(&d);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    StreamEvent::ToolCall(tc) => {
                                        received_any = true;
                                        tool_calls.push(tc)
                                    }
                                    StreamEvent::Usage(u) => {
                                        received_any = true;
                                        stream_usage = Some(u);
                                    }
                                    StreamEvent::Done => break,
                                },
                            }
                        }
                    }
                }
                if stream_cancelled {
                    pop_assistant_placeholder(&messages, assistant_id).await;
                    logger.line(
                        task_id,
                        "[llm_response_end] status=cancelled reason=cooperative_in_flight",
                    );
                    logger.line(task_id, "[task_end] status=cancelled reason=cooperative");
                    return Err(CoreError::CooperativeCancel);
                }
                if !received_any {
                    streamed = false;
                }
            }

            // If streaming didn't work, do the normal one-shot request and replace the placeholder assistant message.
            let response = if streamed {
                // Rehydrate a response-like tuple from current message + tool_calls.
                let assistant_msg = {
                    let g = messages.lock().await;
                    g.iter()
                        .rev()
                        .find(|m| m.id == assistant_id)
                        .cloned()
                        .unwrap_or(Message {
                            id: assistant_id,
                            role: MessageRole::Assistant,
                            content: MessageContent::Text(String::new()),
                            timestamp: chrono::Utc::now(),
                            metadata: HashMap::new(),
                        })
                };
                LLMResponse {
                    message: assistant_msg,
                    tool_calls,
                    usage: stream_usage.unwrap_or_else(|| Usage {
                        input_tokens: estimate_input_tokens_for_messages(&messages_snapshot),
                        output_tokens: 0,
                        cache_creation_tokens: None,
                        cache_read_tokens: None,
                    }),
                }
            } else {
                let chat_fut =
                    self.llm_client
                        .chat(messages_snapshot, tool_schemas.clone(), &model_config);
                let r = tokio::select! {
                    biased;
                    () = coop_flag_wait_opt(coop_cancel.clone()) => {
                        pop_assistant_placeholder(&messages, assistant_id).await;
                        logger.line(
                            task_id,
                            "[llm_response_end] status=cancelled reason=cooperative_in_flight",
                        );
                        logger.line(task_id, "[task_end] status=cancelled reason=cooperative");
                        return Err(CoreError::CooperativeCancel);
                    }
                    res = chat_fut => res?,
                };
                // Replace placeholder assistant with final assistant message.
                {
                    let mut g = messages.lock().await;
                    if let Some(last) = g.last_mut() {
                        if last.id == assistant_id {
                            *last = r.message.clone();
                        }
                    }
                }
                r
            };

            turn_usage.max_input_tokens =
                turn_usage.max_input_tokens.max(response.usage.input_tokens);
            turn_usage.total_output_tokens += response.usage.output_tokens;
            turn_usage.total_cache_read_tokens += response.usage.cache_read_tokens.unwrap_or(0);
            turn_usage.total_cache_creation_tokens +=
                response.usage.cache_creation_tokens.unwrap_or(0);

            logger.line(
                task_id,
                &format!(
                    "[llm_response_end] turn={} elapsed_ms={} input_tokens={} output_tokens={} streamed={}",
                    turn,
                    t0.elapsed().as_millis(),
                    response.usage.input_tokens,
                    response.usage.output_tokens,
                    streamed
                ),
            );

            // 若本轮有 tool_calls，写入 metadata 供 OpenAI 兼容 provider 重建历史
            let mut assistant_msg = response.message.clone();
            if !response.tool_calls.is_empty() {
                assistant_msg.metadata.insert(
                    ANYCODE_TOOL_CALLS_METADATA_KEY.to_string(),
                    serde_json::to_value(&response.tool_calls)?,
                );
                // Also update the in-place message in history with metadata.
                let mut g = messages.lock().await;
                if let Some(last) = g.last_mut() {
                    if last.id == assistant_msg.id {
                        last.metadata = assistant_msg.metadata.clone();
                    }
                }
            }

            // 保留「最后一条非空」正文：部分 API 在收尾会再给一条空 assistant，避免覆盖掉仍应作为 turn 摘要的上一段文字。
            let raw_assistant = match &assistant_msg.content {
                MessageContent::Text(t) => t.as_str(),
                _ => "",
            };
            let text = strip_llm_reasoning_xml_blocks(raw_assistant);
            if response.tool_calls.is_empty() {
                let stream_err = provider_error_from_streamed_assistant_text(&text)
                    .or_else(|| provider_error_from_streamed_assistant_text(raw_assistant));
                if let Some(err) = stream_err {
                    logger.line(
                        task_id,
                        &format!(
                            "[llm_response_end] status=stream_error_as_body turn={} detail={}",
                            turn, err
                        ),
                    );
                    {
                        let mut g = messages.lock().await;
                        if g.last().is_some_and(|m| m.role == MessageRole::Assistant) {
                            g.pop();
                        }
                    }
                    logger.line(task_id, "[task_end] status=failed");
                    return Err(CoreError::LLMError(err));
                }
            }
            if !text.trim().is_empty() {
                last_assistant_text = text;
            }
            // If we streamed, assistant message is already in `messages`; no need to push again.
            // If we didn't stream, we already replaced placeholder with `r.message` above.

            let session_label = format!("tui_{}", task_id);

            if response.tool_calls.is_empty() {
                self.pipeline_memory_hook_agent_turn(
                    &session_label,
                    task_id,
                    turn,
                    &last_assistant_text,
                )
                .await;
                self.maybe_session_notify_agent_turn(
                    &session_label,
                    task_id,
                    turn,
                    &last_assistant_text,
                    Some(working_directory),
                );
                logger.line(task_id, &format!("[turn_end] turn={} tool_calls=0", turn));
                break;
            }

            logger.line(
                task_id,
                &format!(
                    "[turn_end] turn={} tool_calls={}",
                    turn,
                    response.tool_calls.len()
                ),
            );

            for tool_call in response.tool_calls {
                if opt_coop_cancelled(&coop_cancel) {
                    logger.line(task_id, "[task_end] status=cancelled reason=cooperative");
                    return Err(CoreError::CooperativeCancel);
                }
                total_tool_calls += 1;
                if total_tool_calls > MAX_TOOL_CALLS_TOTAL {
                    logger.line(
                        task_id,
                        &format!(
                            "[task_end] status=failed reason=max_tool_calls({})",
                            MAX_TOOL_CALLS_TOTAL
                        ),
                    );
                    return Ok(TurnOutput {
                        final_text: last_assistant_text,
                        artifacts,
                        usage: turn_usage,
                    });
                }

                // 记录工具入参（截断避免日志/上下文过大）
                let tool_input_json = serde_json::to_string(&tool_call.input)
                    .unwrap_or_else(|_| "<unserializable>".to_string());
                let (tool_input_json, truncated) =
                    truncate_text(tool_input_json, TOOL_INPUT_LOG_MAX_BYTES);
                logger.line(
                    task_id,
                    &format!(
                        "[tool_call_input] turn={} idx={} name={} truncated={}",
                        turn, total_tool_calls, tool_call.name, truncated
                    ),
                );
                logger.line(task_id, &tool_input_json);

                logger.line(
                    task_id,
                    &format!(
                        "[tool_call_start] turn={} idx={} name={}",
                        turn, total_tool_calls, tool_call.name
                    ),
                );
                let t0 = std::time::Instant::now();
                let tool_result = self
                    .execute_tool_call(task_id, working_directory, &tool_call)
                    .await?;
                logger.line(
                    task_id,
                    &format!(
                        "[tool_call_end] turn={} idx={} name={} elapsed_ms={} error={}",
                        turn,
                        total_tool_calls,
                        tool_call.name,
                        t0.elapsed().as_millis(),
                        tool_result
                            .error
                            .clone()
                            .unwrap_or_else(|| "<none>".to_string())
                    ),
                );

                // 回注 tool_result（截断以防爆上下文）
                let tool_text = if let Some(err) = tool_result.error.clone() {
                    format!("ERROR: {}\nRESULT: {}", err, tool_result.result)
                } else {
                    format!("{}", tool_result.result)
                };
                let (tool_text, truncated) = truncate_text(tool_text, TOOL_RESULT_MAX_BYTES);
                if truncated {
                    logger.line(
                        task_id,
                        &format!(
                            "[tool_result] truncated=true max_bytes={}",
                            TOOL_RESULT_MAX_BYTES
                        ),
                    );
                }

                let for_hook = tool_text.clone();
                let mut metadata = HashMap::new();
                metadata.insert(
                    "tool_name".to_string(),
                    serde_json::Value::String(tool_call.name.clone()),
                );

                {
                    let mut g = messages.lock().await;
                    g.push(Message {
                        id: Uuid::new_v4(),
                        role: MessageRole::Tool,
                        content: MessageContent::ToolResult {
                            tool_use_id: tool_call.id.clone(),
                            content: tool_text,
                            is_error: tool_result.error.is_some(),
                        },
                        timestamp: chrono::Utc::now(),
                        metadata,
                    });
                }

                self.pipeline_memory_hook_tool_result(
                    &session_label,
                    task_id,
                    &tool_call.name,
                    &for_hook,
                )
                .await;
                self.maybe_session_notify_tool_result(
                    &session_label,
                    task_id,
                    turn,
                    &tool_call.name,
                    &for_hook,
                    Some(working_directory),
                );

                artifacts.extend(extract_artifacts(&tool_call, &tool_result));
                if opt_coop_cancelled(&coop_cancel) {
                    logger.line(task_id, "[task_end] status=cancelled reason=cooperative");
                    return Err(CoreError::CooperativeCancel);
                }
            }
        }

        logger.line(task_id, "[task_end] status=completed");
        let user_line = {
            let g = messages.lock().await;
            last_user_plain_text_for_autosave(&g)
        };
        if !last_assistant_text.trim().is_empty() {
            self.maybe_autosave_memory(task_id, &user_line, &last_assistant_text)
                .await;
            return Ok(TurnOutput {
                final_text: last_assistant_text,
                artifacts,
                usage: turn_usage,
            });
        }

        let output_tail = logger.tail(task_id, 24 * 1024);
        let artifacts_brief = ReceiptGenerator::artifacts_brief(&artifacts);
        let summary_model = self.model_for_summary().clone();
        let summary_task = Task {
            id: task_id,
            agent_type: agent_type.clone(),
            prompt: user_line.clone(),
            context: TaskContext {
                session_id: Uuid::new_v4(),
                working_directory: working_directory.to_string(),
                environment: HashMap::new(),
                user_id: None,
                system_prompt_append: None,
                context_injections: vec![],
                nested_model_override: None,
                nested_worktree_path: None,
                nested_worktree_repo_root: None,
                nested_cancel: None,
                channel_progress_tx: None,
            },
            created_at: chrono::Utc::now(),
        };
        let summary_text = llm_summary_receipt(
            &self.llm_client,
            &summary_model,
            &summary_task,
            total_tool_calls,
            MAX_AGENT_TURNS,
            MAX_TOOL_CALLS_TOTAL,
            &artifacts_brief,
            &output_tail,
        )
        .await;
        self.maybe_autosave_memory(task_id, &user_line, &summary_text)
            .await;
        Ok(TurnOutput {
            final_text: summary_text,
            artifacts,
            usage: turn_usage,
        })
    }

    /// 执行任务
    pub async fn execute_task(&self, task: Task) -> Result<TaskResult, CoreError> {
        let _nested_wt = NestedWorktreeGuard(
            match (
                &task.context.nested_worktree_repo_root,
                &task.context.nested_worktree_path,
            ) {
                (Some(r), Some(p)) if !r.is_empty() && !p.is_empty() => {
                    Some((r.clone(), p.clone()))
                }
                _ => None,
            },
        );

        let logger = self.logger();
        logger.ensure_initialized(task.id);
        logger.line(
            task.id,
            &format!("[task_start] agent_type={}", task.agent_type.as_str()),
        );

        // 1. 获取 Agent
        let agents = self.agents.read().await;
        let agent = agents
            .get(&task.agent_type)
            .ok_or_else(|| CoreError::AgentNotFound(task.id))?;

        // 2. 加载相关记忆
        let memories = self
            .memory_store
            .recall(&task.prompt, MemoryType::Project)
            .await?;

        // 3. 构建消息（system + context status + user）
        let mode = Self::runtime_mode_for_agent(agent.agent_type());
        let mut messages: Vec<Message> = vec![Message {
            id: Uuid::new_v4(),
            role: MessageRole::System,
            content: MessageContent::Text(self.build_system_prompt(
                agent,
                task.context.working_directory.as_str(),
                task.context.system_prompt_append.as_deref(),
            )?),
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        }];
        messages.extend(
            self.context_messages_from_sections(self.build_context_sections(
                mode,
                &memories,
                &task.context.context_injections,
            )),
        );

        // 用户消息
        messages.push(Message {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: MessageContent::Text(task.prompt.clone()),
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        });

        // 4. 工具名与 schema（与 TUI turn 共用 tool_surface）
        let tools = self.tools.read().await;
        let raw =
            tool_surface::resolve_agent_tool_names(task.agent_type.as_str(), agent.tools(), &tools);
        let names = tool_surface::prepare_tool_names_for_llm(
            raw,
            &self.tool_name_deny,
            &self.claude_gating,
        );
        let tool_schemas = tool_surface::build_tool_schemas(&names, &tools);
        drop(tools);

        // 5. 多轮 tool loop（assistant → tool_calls → 执行 → tool_result）
        let mut model_config = self.model_for_task(&task.agent_type).clone();
        if let Some(ref hint) = task.context.nested_model_override {
            model_config = crate::nested_model::resolve_nested_model_hint(&model_config, hint);
        }
        let mut total_tool_calls: usize = 0;
        let mut artifacts: Vec<Artifact> = vec![];

        for turn in 1..=MAX_AGENT_TURNS {
            logger.line(
                task.id,
                &format!("[turn_start] turn={}/{}", turn, MAX_AGENT_TURNS),
            );
            if nested_coop_cancelled(&task.context) {
                logger.line(task.id, "[task_end] status=cancelled reason=cooperative");
                return Ok(task_cancelled_failure());
            }
            logger.line(
                task.id,
                &format!(
                    "[llm_request_start] turn={} model={} base_url={}",
                    turn,
                    model_config.model,
                    model_config
                        .base_url
                        .clone()
                        .unwrap_or_else(|| "<default>".to_string())
                ),
            );

            let t0 = std::time::Instant::now();
            let response_result = match task.context.nested_cancel.clone() {
                Some(flag) => {
                    tokio::select! {
                        biased;
                        () = coop_flag_wait(flag) => {
                            logger.line(
                                task.id,
                                "[llm_response_end] status=cancelled reason=cooperative_in_flight",
                            );
                            logger.line(task.id, "[task_end] status=cancelled reason=cooperative");
                            return Ok(task_cancelled_failure());
                        }
                        res = self.llm_client.chat(
                            messages.clone(),
                            tool_schemas.clone(),
                            &model_config,
                        ) => res,
                    }
                }
                None => {
                    self.llm_client
                        .chat(messages.clone(), tool_schemas.clone(), &model_config)
                        .await
                }
            };

            let response = match response_result {
                Ok(r) => r,
                Err(e) => {
                    logger.line(
                        task.id,
                        &format!(
                            "[llm_response_end] status=error turn={} elapsed_ms={} error={}",
                            turn,
                            t0.elapsed().as_millis(),
                            e
                        ),
                    );
                    logger.line(task.id, "[task_end] status=failed");
                    return Ok(TaskResult::Failure {
                        error: "LLM 调用失败".to_string(),
                        details: Some(e.to_string()),
                    });
                }
            };

            logger.line(
                task.id,
                &format!(
                    "[llm_response_end] turn={} elapsed_ms={} input_tokens={} output_tokens={}",
                    turn,
                    t0.elapsed().as_millis(),
                    response.usage.input_tokens,
                    response.usage.output_tokens
                ),
            );

            // 先把 assistant 消息追加回上下文；若本轮有 tool_calls，写入 metadata 供 OpenAI 兼容 provider 重建历史
            let mut assistant_msg = response.message.clone();
            if !response.tool_calls.is_empty() {
                if let Ok(v) = serde_json::to_value(&response.tool_calls) {
                    assistant_msg
                        .metadata
                        .insert(ANYCODE_TOOL_CALLS_METADATA_KEY.to_string(), v);
                }
            }
            messages.push(assistant_msg);

            let session_label = task.context.session_id.to_string();
            let turn_plain = messages
                .last()
                .and_then(|m| match &m.content {
                    MessageContent::Text(t) => Some(strip_llm_reasoning_xml_blocks(t)),
                    _ => None,
                })
                .unwrap_or_default();

            if response.tool_calls.is_empty() {
                self.pipeline_memory_hook_agent_turn(&session_label, task.id, turn, &turn_plain)
                    .await;
                self.maybe_session_notify_agent_turn(
                    &session_label,
                    task.id,
                    turn,
                    &turn_plain,
                    Some(task.context.working_directory.as_str()),
                );
                logger.line(task.id, &format!("[turn_end] turn={} tool_calls=0", turn));
                break;
            }

            logger.line(
                task.id,
                &format!(
                    "[turn_end] turn={} tool_calls={}",
                    turn,
                    response.tool_calls.len()
                ),
            );

            for tool_call in response.tool_calls {
                if nested_coop_cancelled(&task.context) {
                    logger.line(task.id, "[task_end] status=cancelled reason=cooperative");
                    return Ok(task_cancelled_failure());
                }
                total_tool_calls += 1;
                if total_tool_calls > MAX_TOOL_CALLS_TOTAL {
                    logger.line(
                        task.id,
                        &format!(
                            "[task_end] status=failed reason=max_tool_calls({})",
                            MAX_TOOL_CALLS_TOTAL
                        ),
                    );
                    return Ok(TaskResult::Failure {
                        error: "达到最大工具调用次数，已停止".to_string(),
                        details: Some(format!("max_tool_calls={}", MAX_TOOL_CALLS_TOTAL)),
                    });
                }

                // 记录工具入参（截断避免日志/上下文过大）
                let tool_input_json = serde_json::to_string(&tool_call.input)
                    .unwrap_or_else(|_| "<unserializable>".to_string());
                let (tool_input_json, truncated) =
                    truncate_text(tool_input_json, TOOL_INPUT_LOG_MAX_BYTES);
                logger.line(
                    task.id,
                    &format!(
                        "[tool_call_input] turn={} idx={} name={} truncated={}",
                        turn, total_tool_calls, tool_call.name, truncated
                    ),
                );
                logger.line(task.id, &tool_input_json);

                logger.line(
                    task.id,
                    &format!(
                        "[tool_call_start] turn={} idx={} name={}",
                        turn, total_tool_calls, tool_call.name
                    ),
                );
                channel_progress_send(&task.context, format!("🔧 {}", tool_call.name));
                let t0 = std::time::Instant::now();
                let tool_result = self
                    .execute_tool_call(task.id, &task.context.working_directory, &tool_call)
                    .await?;
                logger.line(
                    task.id,
                    &format!(
                        "[tool_call_end] turn={} idx={} name={} elapsed_ms={} error={}",
                        turn,
                        total_tool_calls,
                        tool_call.name,
                        t0.elapsed().as_millis(),
                        tool_result
                            .error
                            .clone()
                            .unwrap_or_else(|| "<none>".to_string())
                    ),
                );

                // 回注 tool_result（截断以防爆上下文）
                let tool_text = if let Some(err) = tool_result.error.clone() {
                    format!("ERROR: {}\nRESULT: {}", err, tool_result.result)
                } else {
                    format!("{}", tool_result.result)
                };
                let (tool_text, truncated) = truncate_text(tool_text, TOOL_RESULT_MAX_BYTES);
                if truncated {
                    logger.line(
                        task.id,
                        &format!(
                            "[tool_result] truncated=true max_bytes={}",
                            TOOL_RESULT_MAX_BYTES
                        ),
                    );
                }
                let for_hook = tool_text.clone();
                let mut tool_meta = HashMap::new();
                tool_meta.insert(
                    "tool_name".to_string(),
                    serde_json::Value::String(tool_call.name.clone()),
                );
                messages.push(Message {
                    id: Uuid::new_v4(),
                    role: MessageRole::Tool,
                    content: MessageContent::ToolResult {
                        tool_use_id: tool_call.id.clone(),
                        content: tool_text,
                        is_error: tool_result.error.is_some(),
                    },
                    timestamp: chrono::Utc::now(),
                    metadata: tool_meta,
                });

                if tool_result.error.is_some() {
                    let e_short: String = tool_result
                        .error
                        .as_deref()
                        .unwrap_or("error")
                        .chars()
                        .take(120)
                        .collect();
                    channel_progress_send(
                        &task.context,
                        format!("✗ {} {}", tool_call.name, e_short),
                    );
                } else {
                    channel_progress_send(&task.context, format!("✓ {}", tool_call.name));
                }

                self.pipeline_memory_hook_tool_result(
                    &session_label,
                    task.id,
                    &tool_call.name,
                    &for_hook,
                )
                .await;
                self.maybe_session_notify_tool_result(
                    &session_label,
                    task.id,
                    turn,
                    &tool_call.name,
                    &for_hook,
                    Some(task.context.working_directory.as_str()),
                );

                // 基础 artifacts（V1）
                artifacts.extend(extract_artifacts(&tool_call, &tool_result));
                if nested_coop_cancelled(&task.context) {
                    logger.line(task.id, "[task_end] status=cancelled reason=cooperative");
                    return Ok(task_cancelled_failure());
                }
            }
        }

        // 正常收尾：最后一跳无 tool_calls，故末条消息即本轮 assistant。打满 MAX_AGENT_TURNS 且末尾为 Tool 时不走此路径，保留 summary。
        if let Some(fast) = last_assistant_plain_text(&messages) {
            logger.line(task.id, "[task_end] status=completed");
            logger.line(
                task.id,
                &format!(
                    "[final_output] source=assistant reply_chars={}",
                    fast.chars().count()
                ),
            );
            logger.line(task.id, "== assistant_final ==");
            for line in fast.lines() {
                logger.line(task.id, line);
            }
            self.maybe_autosave_memory(task.id, &task.prompt, &fast)
                .await;
            return Ok(TaskResult::Success {
                output: fast,
                artifacts,
            });
        }

        // 7. 生成总结（末条非 assistant、assistant 正文为空、或仅 tool_calls 等）
        let output_tail = logger.tail(task.id, 24 * 1024);
        let artifacts_brief = ReceiptGenerator::artifacts_brief(&artifacts);

        let summary_model = self.model_for_summary().clone();
        let summary_text = llm_summary_receipt(
            &self.llm_client,
            &summary_model,
            &task,
            total_tool_calls,
            MAX_AGENT_TURNS,
            MAX_TOOL_CALLS_TOTAL,
            &artifacts_brief,
            &output_tail,
        )
        .await;

        logger.line(task.id, "[task_end] status=completed");
        logger.line(task.id, "== summary ==");
        for line in summary_text.lines() {
            logger.line(task.id, line);
        }

        self.maybe_autosave_memory(task.id, &task.prompt, &summary_text)
            .await;

        Ok(TaskResult::Success {
            output: summary_text,
            artifacts,
        })
    }

    pub async fn execute_goal_task(
        &self,
        task: Task,
        spec: GoalSpec,
    ) -> Result<(TaskResult, GoalProgress), CoreError> {
        let engine = GoalEngine::new(spec);
        let mut progress = GoalProgress::default();
        let mut current_task = engine.prime_task(task);
        let mut last_result = TaskResult::Failure {
            error: "goal did not run".to_string(),
            details: None,
        };

        while engine.should_continue(&progress) {
            let result = self.execute_task(current_task.clone()).await?;
            engine.update(&mut progress, &result);
            last_result = result.clone();
            if progress.completed {
                break;
            }
            current_task.context.context_injections = vec![format!(
                "## Goal Retry Context\nPrevious attempt count: {}.\nLast error: {:?}\nLast output: {:?}",
                progress.attempts, progress.last_error, progress.last_output
            )];
        }

        Ok((last_result, progress))
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

    async fn execute_tool_call(
        &self,
        task_id: TaskId,
        working_directory: &str,
        tool_call: &ToolCall,
    ) -> Result<ToolOutput, CoreError> {
        let tools = self.tools.read().await;
        let tool = tools
            .get(&tool_call.name)
            .ok_or_else(|| CoreError::ToolNotFound(tool_call.name.clone()))?;

        match self
            .security
            .check_tool_call(&tool_call.name, &tool_call.input)
            .await
        {
            Ok(_) => {}
            Err(CoreError::PermissionDenied(reason)) => {
                self.log_task_line(
                    task_id,
                    &format!("[tool_denied] name={} reason={}", tool_call.name, reason),
                );
                return Ok(ToolOutput {
                    result: serde_json::json!({ "error": reason.clone() }),
                    error: Some(reason),
                    duration_ms: 0,
                });
            }
            Err(e) => return Err(e),
        }

        if !self.security.is_bypass_permissions().await {
            if let Some(rules) = &self.claude_gating.rules {
                let args_json =
                    serde_json::to_string(&tool_call.input).unwrap_or_else(|_| "{}".into());
                if rules.content_denies(&tool_call.name, &args_json)
                    && !rules.content_allows(&tool_call.name, &args_json)
                {
                    let reason =
                        "Permission deny rule matched (tool arguments matched ruleContent)"
                            .to_string();
                    self.log_task_line(
                        task_id,
                        &format!("[tool_denied] name={} reason={}", tool_call.name, reason),
                    );
                    return Ok(ToolOutput {
                        result: serde_json::json!({ "error": reason.clone() }),
                        error: Some(reason),
                        duration_ms: 0,
                    });
                }
                if rules.needs_ask(&tool_call.name, &args_json) {
                    let skip_second_prompt = self
                        .security
                        .skip_redundant_claude_ask_after_tool_check(&tool_call.name)
                        .await;
                    if !skip_second_prompt {
                        match self
                            .security
                            .confirm_claude_ask_or_deny(&tool_call.name, &tool_call.input)
                            .await
                        {
                            Ok(()) => {}
                            Err(CoreError::PermissionDenied(reason)) => {
                                self.log_task_line(
                                    task_id,
                                    &format!(
                                        "[tool_denied] name={} reason={}",
                                        tool_call.name, reason
                                    ),
                                );
                                return Ok(ToolOutput {
                                    result: serde_json::json!({ "error": reason.clone() }),
                                    error: Some(reason),
                                    duration_ms: 0,
                                });
                            }
                            Err(e) => return Err(e),
                        }
                    }
                }
            }
        }

        let input = ToolInput {
            name: tool_call.name.clone(),
            input: tool_call.input.clone(),
            working_directory: if working_directory.is_empty() {
                None
            } else {
                Some(working_directory.to_string())
            },
            sandbox_mode: self.sandbox_mode,
        };

        tool.execute(input).await
    }
}

#[async_trait]
impl SubAgentExecutor for AgentRuntime {
    async fn run_nested_task(&self, invoke: NestedTaskInvoke) -> Result<NestedTaskRun, CoreError> {
        let mut wd = invoke.working_directory;
        let wt_roots = {
            let iso = invoke
                .isolation
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty());
            if iso.is_some_and(|s| s.eq_ignore_ascii_case("worktree")) {
                let (repo, wt) = nested_worktree::create_nested_worktree(&wd).await?;
                wd = wt.clone();
                Some((repo, wt))
            } else {
                None
            }
        };

        let task = Task {
            id: invoke.task_id.unwrap_or_else(Uuid::new_v4),
            agent_type: invoke.agent_type,
            prompt: invoke.prompt,
            context: TaskContext {
                session_id: Uuid::new_v4(),
                working_directory: wd,
                environment: HashMap::new(),
                user_id: None,
                system_prompt_append: None,
                context_injections: vec![],
                nested_model_override: invoke.model.clone(),
                nested_worktree_repo_root: wt_roots.as_ref().map(|(r, _)| r.clone()),
                nested_worktree_path: wt_roots.as_ref().map(|(_, p)| p.clone()),
                nested_cancel: invoke.cancel.clone(),
                channel_progress_tx: None,
            },
            created_at: chrono::Utc::now(),
        };
        let task_id = task.id;
        let result = self.execute_task(task).await?;
        Ok(NestedTaskRun { task_id, result })
    }
}

#[cfg(test)]
mod streamed_provider_error_tests {
    #[test]
    fn detects_array_wrapped_google_json() {
        let j = r#"[{"error":{"code":400,"message":"User location is not supported for the API use.","status":"FAILED_PRECONDITION"}}]"#;
        let e = super::provider_error_from_streamed_assistant_text(j).expect("should detect");
        assert!(
            e.contains("User location") || e.contains("streamed provider error"),
            "{e}"
        );
    }

    #[test]
    fn ignores_normal_assistant_prose() {
        let j = "Here is an example JSON: {\"error\": \"not a provider envelope\"}";
        assert!(super::provider_error_from_streamed_assistant_text(j).is_none());
    }

    #[test]
    fn detects_top_level_error_string() {
        let j = r#"{"error":"User location is not supported for the API use."}"#;
        let e = super::provider_error_from_streamed_assistant_text(j).expect("detect");
        assert!(e.contains("User location") || e.contains("streamed"), "{e}");
    }

    #[test]
    fn bom_before_json_still_parses() {
        let j =
            "\u{feff}[{\"error\":{\"code\":400,\"message\":\"User location is not supported\"}}]";
        assert!(super::provider_error_from_streamed_assistant_text(j).is_some());
    }

    #[test]
    fn heuristic_truncated_geo_json() {
        let j = r#"[{"error":{"code":400,"message":"User location is not supported","status":"FAILED_PRECONDITION""#;
        assert!(super::provider_error_from_streamed_assistant_text(j).is_some());
    }
}
