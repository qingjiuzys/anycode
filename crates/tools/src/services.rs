//! 跨工具共享的运行时状态与 HTTP 客户端（装配自 `bootstrap` / `build_registry`）。

use crate::ask_user_question_host::AskUserQuestionHostArc;
use crate::skills::{SkillCatalog, SkillsGovernance};
use crate::wechat_outbound_host::WeChatOutboundHostArc;
use anycode_core::{
    plan_tree_all_completed, CoreError, NestedTaskRun, PlanTree, SubAgentExecutor, TaskResult,
    NESTED_TASK_COOPERATIVE_CANCEL_ERROR,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use uuid::Uuid;

/// Resolved LSP stdio settings (from `config.json` `lsp` + bootstrap).
#[derive(Clone)]
pub struct LspConnectionConfig {
    pub command: Option<String>,
    pub workspace_root: Option<std::path::PathBuf>,
    pub read_timeout: Duration,
}

impl Default for LspConnectionConfig {
    fn default() -> Self {
        Self {
            command: None,
            workspace_root: None,
            read_timeout: Duration::from_secs(60),
        }
    }
}

/// `Agent` / `Task` with `run_in_background: true` — process-local, not persisted in orchestration.json.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundAgentStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
    Partial,
}

impl BackgroundAgentStatus {
    pub fn as_json_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Partial => "partial",
        }
    }
}

pub struct BackgroundAgentJob {
    pub status: Mutex<BackgroundAgentStatus>,
    pub started_at: std::time::SystemTime,
    pub abort: Mutex<Option<tokio::task::AbortHandle>>,
    pub summary: Mutex<Option<String>>,
    /// Set by **`TaskStop`**; **`AgentRuntime::execute_task`** polls between turns/tools.
    pub coop_cancel: Arc<AtomicBool>,
}

impl BackgroundAgentJob {
    pub fn set_abort(&self, handle: tokio::task::AbortHandle) {
        *self.abort.lock().expect("abort mutex") = Some(handle);
    }
}

/// 装配默认工具注册表时的依赖（沙箱标志 + 可选共享服务）。
#[derive(Clone)]
pub struct ToolRegistryDeps {
    pub sandbox_mode: bool,
    pub services: Arc<ToolServices>,
}

impl ToolRegistryDeps {
    pub fn minimal(sandbox_mode: bool) -> Self {
        Self {
            sandbox_mode,
            services: Arc::new(ToolServices::default()),
        }
    }
}

/// 会话级 todo（对齐 Claude Code `TodoWrite` 的简化模型）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: String,
    pub content: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskRecord {
    pub id: String,
    pub subject: String,
    pub description: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TeamRecord {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub member_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    pub id: String,
    pub schedule: String,
    pub command: String,
    /// Stable session id for all future runs of this cron job. The scheduler still
    /// executes independent task ids today; this id is the durable correlation key.
    #[serde(default)]
    pub session_id: Option<String>,
    /// Future failure routing: `log` (default), `same_channel`, `shell`, `http`.
    #[serde(default)]
    pub failure_destination: Option<String>,
    /// Future tool subset hint: `default`, `read_only`, `observability`, or `allowlist`.
    #[serde(default)]
    pub tool_profile: Option<String>,
    /// Explicit tool ids when `tool_profile` is `allowlist`.
    #[serde(default)]
    pub tool_allowlist: Option<Vec<String>>,
    /// Dashboard project scope. `None` (incl. legacy entries) = whole workspace.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
}

/// Optional production fields when creating cron jobs via `CronCreate` or scheduler APIs.
#[derive(Debug, Clone, Default)]
pub struct CronJobCreateOptions {
    pub session_id: Option<String>,
    pub failure_destination: Option<String>,
    pub tool_profile: Option<String>,
    pub tool_allowlist: Option<Vec<String>>,
    pub project_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuntimeModeState {
    pub plan_mode: bool,
    pub worktree_path: Option<String>,
    pub base_workdir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct OrchestrationSnapshotV1 {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    todos: Vec<TodoItem>,
    #[serde(default)]
    plan_tree: PlanTree,
    #[serde(default)]
    tasks: HashMap<String, TaskRecord>,
    #[serde(default)]
    teams: HashMap<String, TeamRecord>,
    #[serde(default)]
    crons: Vec<CronJob>,
    #[serde(default)]
    remote_hooks: Vec<String>,
    #[serde(default)]
    inter_messages: Vec<(String, String)>,
    #[serde(default)]
    mode: RuntimeModeState,
    #[serde(default)]
    deferred_tool_names: Vec<String>,
    #[serde(default)]
    config_overrides: HashMap<String, serde_json::Value>,
}

/// 共享服务：HTTP、会话 todo、编排任务等；可选将编排状态落盘 `~/.anycode/tasks/orchestration.json`。
pub struct ToolServices {
    pub http: Client,
    pub max_fetch_bytes: u64,
    /// WebSearch：可选 API（如 Brave）；未配置时走 DuckDuckGo 即时答案 JSON（无需 key）。
    pub web_search_api_key: Option<String>,
    pub web_search_endpoint: Option<String>,
    orchestration_path: Option<PathBuf>,
    todos: Mutex<Vec<TodoItem>>,
    plan_tree: Mutex<PlanTree>,
    tasks: Mutex<HashMap<String, TaskRecord>>,
    teams: Mutex<HashMap<String, TeamRecord>>,
    crons: Mutex<Vec<CronJob>>,
    remote_hooks: Mutex<Vec<String>>,
    inter_messages: Mutex<Vec<(String, String)>>,
    mode: Mutex<RuntimeModeState>,
    /// `ToolSearch` 登记的延后工具名（演示用）。
    deferred_tool_names: Mutex<Vec<String>>,
    /// `Config` 工具内存覆盖（不直接写盘；真实持久化由 CLI 负责）。
    config_overrides: Mutex<HashMap<String, serde_json::Value>>,
    /// 装配后由 CLI 注入，供 `Agent` / `Task` 工具嵌套 `execute_task`。
    sub_agent_executor: Mutex<Option<Arc<dyn SubAgentExecutor>>>,
    /// REPL/TUI 注入：`AskUserQuestion` 主机侧选题。
    ask_user_question_host: Mutex<Option<AskUserQuestionHostArc>>,
    /// CLI 注入：`SendWeChatMessage` 真实 iLink 出站。
    wechat_outbound_host: Mutex<Option<WeChatOutboundHostArc>>,
    /// `LSP` 工具：`tools-lsp` 下读此配置（CLI bootstrap 写入）。
    lsp: Mutex<LspConnectionConfig>,
    sub_agent_depth: AtomicU32,
    /// `run_in_background` nested agents: keyed by `nested_task_id` / execution UUID.
    background_agents: Mutex<HashMap<Uuid, Arc<BackgroundAgentJob>>>,
    /// 长驻 MCP 会话：stdio 与 Streamable HTTP（`ANYCODE_MCP_*`）。
    #[cfg(feature = "tools-mcp")]
    mcp_sessions: Mutex<Vec<Arc<dyn crate::mcp_connected::McpConnected>>>,
    /// `defer_mcp_tools` 时，经 `ToolSearch` 登记后可出现在首轮 LLM 工具列表中的 `mcp__*` 名。
    mcp_defer_allowlist: Option<Arc<Mutex<HashSet<String>>>>,
    /// Startup scan of `SKILL.md` skills + resolution rules for the `Skill` tool.
    pub skill_catalog: Arc<SkillCatalog>,
    /// Runtime skill governance (global / per-agent / project allowlists).
    pub skills_governance: Mutex<SkillsGovernance>,
    /// Active agent id for the current tool loop (Skill governance).
    active_agent_type: Mutex<Option<String>>,
    /// Parent `execute_task` tool surface for nested Agent/Task inheritance.
    parent_task_tool_deny: Mutex<Option<(Vec<String>, Vec<String>)>>,
    /// Injected at bootstrap; avoids per-execute disk reads in media tools.
    media_registry: Mutex<Option<Arc<anycode_llm::media::MediaClientRegistry>>>,
    /// Local WeChat chat history query settings (`wechatHistory` in config.json).
    wechat_history_config: Mutex<anycode_wechat_history::WechatHistoryConfig>,
}

impl Default for ToolServices {
    fn default() -> Self {
        Self {
            http: Client::builder()
                .user_agent(anycode_core::user_agent("anycode-tools"))
                .build()
                .expect("reqwest client"),
            max_fetch_bytes: 2 * 1024 * 1024,
            web_search_api_key: std::env::var("ANYCODE_WEB_SEARCH_API_KEY").ok(),
            web_search_endpoint: std::env::var("ANYCODE_WEB_SEARCH_URL").ok(),
            orchestration_path: None,
            todos: Mutex::new(vec![]),
            plan_tree: Mutex::new(PlanTree::default()),
            tasks: Mutex::new(HashMap::new()),
            teams: Mutex::new(HashMap::new()),
            crons: Mutex::new(vec![]),
            remote_hooks: Mutex::new(vec![]),
            inter_messages: Mutex::new(vec![]),
            mode: Mutex::new(RuntimeModeState::default()),
            deferred_tool_names: Mutex::new(vec![]),
            config_overrides: Mutex::new(HashMap::new()),
            sub_agent_executor: Mutex::new(None),
            ask_user_question_host: Mutex::new(None),
            wechat_outbound_host: Mutex::new(None),
            lsp: Mutex::new(LspConnectionConfig::default()),
            sub_agent_depth: AtomicU32::new(0),
            background_agents: Mutex::new(HashMap::new()),
            #[cfg(feature = "tools-mcp")]
            mcp_sessions: Mutex::new(vec![]),
            mcp_defer_allowlist: None,
            skill_catalog: Arc::new(SkillCatalog::empty()),
            skills_governance: Mutex::new(SkillsGovernance::default()),
            active_agent_type: Mutex::new(None),
            parent_task_tool_deny: Mutex::new(None),
            media_registry: Mutex::new(None),
            wechat_history_config: Mutex::new(
                anycode_wechat_history::WechatHistoryConfig::default(),
            ),
        }
    }
}

impl ToolServices {
    fn background_state_path(id: Uuid) -> Option<PathBuf> {
        dirs::home_dir().map(|h| {
            h.join(".anycode/tasks")
                .join(id.to_string())
                .join("state.json")
        })
    }

    fn persist_background_state(id: Uuid, status: BackgroundAgentStatus, summary: Option<&str>) {
        let Some(path) = Self::background_state_path(id) else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let body = serde_json::json!({
            "version": 1,
            "task_id": id,
            "kind": "background_agent",
            "status": status.as_json_str(),
            "summary": summary.unwrap_or(""),
            "updated_at": chrono::Utc::now().to_rfc3339(),
            "diagnostic_only": true,
        });
        if let Ok(text) = serde_json::to_string_pretty(&body) {
            let _ = fs::write(path, text);
        }
    }

    /// 无编排文件路径（如无 HOME），与 `default()` 相同字段，但可挂接 MCP 延迟门控。
    pub fn new_ephemeral(mcp_defer_allowlist: Option<Arc<Mutex<HashSet<String>>>>) -> Self {
        Self::new_ephemeral_with_skills(mcp_defer_allowlist, Arc::new(SkillCatalog::empty()))
    }

    pub fn new_ephemeral_with_skills(
        mcp_defer_allowlist: Option<Arc<Mutex<HashSet<String>>>>,
        skill_catalog: Arc<SkillCatalog>,
    ) -> Self {
        Self {
            mcp_defer_allowlist,
            skill_catalog,
            ..Default::default()
        }
    }

    /// 绑定 `orchestration.json` 路径；若文件已存在则恢复编排状态（P6 持久化 v1）。
    pub fn load_or_new(orchestration_file: PathBuf) -> anyhow::Result<Self> {
        Self::load_or_new_with_mcp_defer(orchestration_file, None, Arc::new(SkillCatalog::empty()))
    }

    pub fn load_or_new_with_mcp_defer(
        orchestration_file: PathBuf,
        mcp_defer_allowlist: Option<Arc<Mutex<HashSet<String>>>>,
        skill_catalog: Arc<SkillCatalog>,
    ) -> anyhow::Result<Self> {
        let s = Self {
            mcp_defer_allowlist,
            skill_catalog,
            orchestration_path: Some(orchestration_file.clone()),
            ..Default::default()
        };
        if orchestration_file.is_file() {
            match fs::read_to_string(&orchestration_file) {
                Ok(text) => match serde_json::from_str::<OrchestrationSnapshotV1>(&text) {
                    Ok(snap) => s.apply_snapshot(snap),
                    Err(e) => {
                        tracing::warn!(
                            target: "anycode_tools",
                            "orchestration.json 无法解析，已忽略并保留备份: {}",
                            e
                        );
                        let bak = orchestration_file.with_extension("json.corrupt");
                        let _ = fs::copy(&orchestration_file, &bak);
                    }
                },
                Err(e) => {
                    tracing::warn!(
                        target: "anycode_tools",
                        "读取 orchestration.json 失败，以空状态启动: {}",
                        e
                    );
                }
            }
        }
        Ok(s)
    }

    /// `ToolSearch` / 会话逻辑：将 `mcp__*` 工具名加入延迟加载白名单（与 `AgentRuntime` 共用同一 `Arc`）。
    pub fn register_mcp_tool_for_llm_session(&self, tool_api_name: &str) {
        let Some(g) = &self.mcp_defer_allowlist else {
            return;
        };
        if let Ok(mut set) = g.lock() {
            set.insert(tool_api_name.to_string());
        }
    }

    pub fn attach_sub_agent_executor(&self, ex: Arc<dyn SubAgentExecutor>) {
        *self.sub_agent_executor.lock().expect("sub_agent_executor") = Some(ex);
    }

    /// Set while a parent [`anycode_core::Task`] is executing so nested agents inherit tool denies.
    pub fn set_parent_task_tool_deny(&self, names: Vec<String>, prefixes: Vec<String>) {
        *self
            .parent_task_tool_deny
            .lock()
            .expect("parent_task_tool_deny") = Some((names, prefixes));
    }

    pub fn clear_parent_task_tool_deny(&self) {
        *self
            .parent_task_tool_deny
            .lock()
            .expect("parent_task_tool_deny") = None;
    }

    pub fn parent_task_tool_deny(&self) -> (Vec<String>, Vec<String>) {
        self.parent_task_tool_deny
            .lock()
            .expect("parent_task_tool_deny")
            .clone()
            .unwrap_or_default()
    }

    pub fn set_skills_governance(&self, gov: SkillsGovernance) {
        *self.skills_governance.lock().expect("skills_governance") = gov;
    }

    pub fn set_media_registry(&self, reg: Arc<anycode_llm::media::MediaClientRegistry>) {
        *self.media_registry.lock().expect("media_registry") = Some(reg);
    }

    pub fn set_wechat_history_config(&self, config: anycode_wechat_history::WechatHistoryConfig) {
        *self
            .wechat_history_config
            .lock()
            .expect("wechat_history_config") = config;
    }

    pub fn wechat_history_config(&self) -> anycode_wechat_history::WechatHistoryConfig {
        self.wechat_history_config
            .lock()
            .expect("wechat_history_config")
            .clone()
    }

    pub fn media_registry(&self) -> Result<anycode_llm::media::MediaClientRegistry, String> {
        if let Some(reg) = self.media_registry.lock().expect("media_registry").clone() {
            return Ok((*reg).clone());
        }
        let (_, cfg) =
            anycode_llm::config_file::read_config_value(None).map_err(|e| e.to_string())?;
        Ok(anycode_llm::media::MediaClientRegistry::from_config(&cfg))
    }

    pub fn set_active_agent_type(&self, agent_type: Option<String>) {
        *self.active_agent_type.lock().expect("active_agent_type") = agent_type;
    }

    pub fn active_agent_type(&self) -> Option<String> {
        self.active_agent_type
            .lock()
            .expect("active_agent_type")
            .clone()
    }

    pub fn is_skill_allowed(&self, skill_id: &str) -> bool {
        let agent = self.active_agent_type().unwrap_or_default();
        let gov = self.skills_governance.lock().expect("skills_governance");
        if agent.is_empty() {
            return true;
        }
        gov.is_allowed(&agent, skill_id)
    }

    pub fn attach_ask_user_question_host(&self, host: AskUserQuestionHostArc) {
        *self
            .ask_user_question_host
            .lock()
            .expect("ask_user_question_host") = Some(host);
    }

    pub fn ask_user_question_host(&self) -> Option<AskUserQuestionHostArc> {
        self.ask_user_question_host
            .lock()
            .expect("ask_user_question_host")
            .clone()
    }

    pub fn attach_wechat_outbound_host(&self, host: WeChatOutboundHostArc) {
        *self
            .wechat_outbound_host
            .lock()
            .expect("wechat_outbound_host") = Some(host);
    }

    pub fn wechat_outbound_host(&self) -> Option<WeChatOutboundHostArc> {
        self.wechat_outbound_host
            .lock()
            .expect("wechat_outbound_host")
            .clone()
    }

    pub fn set_lsp_connection_config(&self, c: LspConnectionConfig) {
        *self.lsp.lock().expect("lsp mutex") = c;
    }

    pub fn lsp_connection_config(&self) -> LspConnectionConfig {
        self.lsp.lock().expect("lsp mutex").clone()
    }

    pub fn sub_agent_executor(&self) -> Option<Arc<dyn SubAgentExecutor>> {
        self.sub_agent_executor
            .lock()
            .expect("sub_agent_executor")
            .clone()
    }

    /// 进入子 Agent 嵌套；超过深度返回 `false`（建议 ≤6 层）。
    pub fn try_enter_sub_agent_depth(&self) -> bool {
        const MAX: u32 = 6;
        loop {
            let cur = self.sub_agent_depth.load(Ordering::Acquire);
            if cur >= MAX {
                return false;
            }
            if self
                .sub_agent_depth
                .compare_exchange_weak(cur, cur + 1, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return true;
            }
        }
    }

    pub fn leave_sub_agent_depth(&self) {
        self.sub_agent_depth.fetch_sub(1, Ordering::AcqRel);
    }

    pub fn insert_background_agent_job(&self, id: Uuid) -> Arc<BackgroundAgentJob> {
        let coop_cancel = Arc::new(AtomicBool::new(false));
        let job = Arc::new(BackgroundAgentJob {
            status: Mutex::new(BackgroundAgentStatus::Running),
            started_at: std::time::SystemTime::now(),
            abort: Mutex::new(None),
            summary: Mutex::new(None),
            coop_cancel: coop_cancel.clone(),
        });
        self.background_agents
            .lock()
            .expect("background_agents")
            .insert(id, job.clone());
        Self::persist_background_state(id, BackgroundAgentStatus::Running, Some("running"));
        job
    }

    /// When the spawned nested task is dropped (e.g. `AbortHandle::abort`), depth and registry must still converge.
    pub fn finalize_background_if_still_running(&self, id: Uuid) {
        let map = self.background_agents.lock().expect("background_agents");
        let Some(j) = map.get(&id) else {
            return;
        };
        let mut st = j.status.lock().expect("bg status");
        if *st == BackgroundAgentStatus::Running {
            *st = BackgroundAgentStatus::Cancelled;
            *j.summary.lock().expect("bg summary") = Some("aborted".into());
            Self::persist_background_state(id, BackgroundAgentStatus::Cancelled, Some("aborted"));
        }
    }

    pub fn finish_background_agent(&self, id: Uuid, run: Result<NestedTaskRun, CoreError>) {
        let map = self.background_agents.lock().expect("background_agents");
        let Some(job) = map.get(&id) else {
            return;
        };
        {
            let st = job.status.lock().expect("bg status");
            if *st == BackgroundAgentStatus::Cancelled {
                return;
            }
        }
        match run {
            Ok(NestedTaskRun { result, .. }) => {
                let new_status = match &result {
                    TaskResult::Success { .. } => BackgroundAgentStatus::Completed,
                    TaskResult::Failure { error, .. }
                        if error == NESTED_TASK_COOPERATIVE_CANCEL_ERROR =>
                    {
                        BackgroundAgentStatus::Cancelled
                    }
                    TaskResult::Failure { .. } => BackgroundAgentStatus::Failed,
                    TaskResult::Partial { .. } => BackgroundAgentStatus::Partial,
                };
                let summary = match result {
                    TaskResult::Success { output, .. } => output.chars().take(500).collect(),
                    TaskResult::Failure { error, .. } => error,
                    TaskResult::Partial { success, remaining } => {
                        format!("{success} / remaining: {remaining}")
                    }
                };
                let mut st = job.status.lock().expect("bg status");
                *st = new_status;
                drop(st);
                *job.summary.lock().expect("bg summary") = Some(summary);
                let persisted_summary = job.summary.lock().expect("bg summary").clone();
                Self::persist_background_state(id, new_status, persisted_summary.as_deref());
            }
            Err(e) => {
                let mut st = job.status.lock().expect("bg status");
                *st = BackgroundAgentStatus::Failed;
                drop(st);
                let summary = e.to_string();
                *job.summary.lock().expect("bg summary") = Some(summary.clone());
                Self::persist_background_state(id, BackgroundAgentStatus::Failed, Some(&summary));
            }
        }
    }

    /// Best-effort: marks cancelled and aborts the tokio task running `run_nested_task`.
    pub fn cancel_background_agent(&self, id: Uuid) -> bool {
        let map = self.background_agents.lock().expect("background_agents");
        let Some(job) = map.get(&id) else {
            return false;
        };
        job.coop_cancel.store(true, Ordering::Release);
        let mut st = job.status.lock().expect("bg status");
        if *st != BackgroundAgentStatus::Running {
            return false;
        }
        *st = BackgroundAgentStatus::Cancelled;
        drop(st);
        *job.summary.lock().expect("bg summary") = Some("cancelled".into());
        Self::persist_background_state(id, BackgroundAgentStatus::Cancelled, Some("cancelled"));
        if let Some(a) = job.abort.lock().expect("abort").as_ref() {
            a.abort();
        }
        true
    }

    /// For [`crate::orchestration::TaskOutputTool`]: status + optional short summary.
    pub fn background_agent_tool_view(
        &self,
        id: Uuid,
    ) -> Option<(BackgroundAgentStatus, Option<String>)> {
        let job = {
            let map = self.background_agents.lock().expect("background_agents");
            map.get(&id).cloned()?
        };
        let st = *job.status.lock().expect("bg status");
        let sum = job.summary.lock().expect("bg summary").clone();
        Some((st, sum))
    }

    #[cfg(feature = "tools-mcp")]
    pub fn attach_mcp_session(&self, session: Arc<dyn crate::mcp_connected::McpConnected>) {
        self.mcp_sessions
            .lock()
            .expect("mcp_sessions")
            .push(session);
    }

    #[cfg(feature = "tools-mcp")]
    pub fn attach_mcp_stdio(&self, session: Arc<crate::mcp_session::McpStdioSession>) {
        let s: Arc<dyn crate::mcp_connected::McpConnected> = session;
        self.attach_mcp_session(s);
    }

    /// 已连接的 MCP 会话（顺序与连接顺序一致）。
    #[cfg(feature = "tools-mcp")]
    pub fn mcp_sessions(&self) -> Vec<Arc<dyn crate::mcp_connected::McpConnected>> {
        self.mcp_sessions.lock().expect("mcp_sessions").clone()
    }

    /// 兼容旧逻辑：仅首个会话（单 MCP 时与历史行为一致）。
    #[cfg(feature = "tools-mcp")]
    pub fn mcp_stdio(&self) -> Option<Arc<dyn crate::mcp_connected::McpConnected>> {
        self.mcp_sessions
            .lock()
            .expect("mcp_sessions")
            .first()
            .cloned()
    }

    fn apply_snapshot(&self, snap: OrchestrationSnapshotV1) {
        *self.todos.lock().expect("todos mutex") = snap.todos;
        *self.plan_tree.lock().expect("plan_tree mutex") = snap.plan_tree;
        *self.tasks.lock().expect("tasks mutex") = snap.tasks;
        *self.teams.lock().expect("teams mutex") = snap.teams;
        *self.crons.lock().expect("crons mutex") = snap.crons;
        *self.remote_hooks.lock().expect("remote mutex") = snap.remote_hooks;
        *self.inter_messages.lock().expect("msg mutex") = snap.inter_messages;
        *self.mode.lock().expect("mode mutex") = snap.mode;
        *self.deferred_tool_names.lock().expect("defer mutex") = snap.deferred_tool_names;
        *self.config_overrides.lock().expect("cfg mutex") = snap.config_overrides;
    }

    fn collect_snapshot(&self) -> OrchestrationSnapshotV1 {
        OrchestrationSnapshotV1 {
            version: 1,
            todos: self.todos.lock().expect("todos mutex").clone(),
            plan_tree: self.plan_tree.lock().expect("plan_tree mutex").clone(),
            tasks: self.tasks.lock().expect("tasks mutex").clone(),
            teams: self.teams.lock().expect("teams mutex").clone(),
            crons: self.crons.lock().expect("crons mutex").clone(),
            remote_hooks: self.remote_hooks.lock().expect("remote mutex").clone(),
            inter_messages: self.inter_messages.lock().expect("msg mutex").clone(),
            mode: self.mode.lock().expect("mode mutex").clone(),
            deferred_tool_names: self
                .deferred_tool_names
                .lock()
                .expect("defer mutex")
                .clone(),
            config_overrides: self.config_overrides.lock().expect("cfg mutex").clone(),
        }
    }

    fn try_persist(&self) {
        let Some(ref path) = self.orchestration_path else {
            return;
        };
        if let Err(e) = self.persist_to_path(path) {
            tracing::warn!(target: "anycode_tools", "orchestration persist failed: {}", e);
        }
    }

    fn persist_to_path(&self, path: &Path) -> anyhow::Result<()> {
        let snap = self.collect_snapshot();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("json.tmp");
        let data = serde_json::to_string_pretty(&snap)?;
        fs::write(&tmp, data)?;
        fs::rename(&tmp, path)?;
        Ok(())
    }

    pub fn replace_todos(&self, new: Vec<TodoItem>) -> (Vec<TodoItem>, Vec<TodoItem>) {
        let mut guard = self.todos.lock().expect("todos mutex");
        let old = std::mem::take(&mut *guard);
        let all_done = new.iter().all(|t| t.status == "completed");
        *guard = if all_done { vec![] } else { new.clone() };
        let cur = guard.clone();
        drop(guard);
        self.try_persist();
        (old, cur)
    }

    pub fn plan_tree(&self) -> PlanTree {
        self.plan_tree.lock().expect("plan_tree mutex").clone()
    }

    pub fn replace_plan_tree(&self, new: PlanTree) -> (PlanTree, PlanTree) {
        let mut guard = self.plan_tree.lock().expect("plan_tree mutex");
        let old = std::mem::take(&mut *guard);
        *guard = if plan_tree_all_completed(&new) {
            PlanTree::default()
        } else {
            new.clone()
        };
        let cur = guard.clone();
        drop(guard);
        self.try_persist();
        (old, cur)
    }

    pub fn insert_task(
        &self,
        subject: String,
        description: String,
        metadata: serde_json::Value,
    ) -> TaskRecord {
        let id = Uuid::new_v4().to_string();
        let t = TaskRecord {
            id: id.clone(),
            subject,
            description,
            status: "pending".to_string(),
            metadata,
        };
        self.tasks
            .lock()
            .expect("tasks mutex")
            .insert(id.clone(), t.clone());
        self.try_persist();
        t
    }

    pub fn get_task(&self, id: &str) -> Option<TaskRecord> {
        self.tasks.lock().expect("tasks mutex").get(id).cloned()
    }

    pub fn list_tasks(&self) -> Vec<TaskRecord> {
        self.tasks
            .lock()
            .expect("tasks mutex")
            .values()
            .cloned()
            .collect()
    }

    pub fn update_task(&self, id: &str, patch: TaskRecord) -> Option<TaskRecord> {
        let mut m = self.tasks.lock().expect("tasks mutex");
        let out = m.get_mut(id).map(|existing| {
            if !patch.subject.is_empty() {
                existing.subject = patch.subject;
            }
            if !patch.description.is_empty() {
                existing.description = patch.description;
            }
            if !patch.status.is_empty() {
                existing.status = patch.status;
            }
            if patch.metadata != serde_json::Value::Null {
                existing.metadata = patch.metadata;
            }
            existing.clone()
        });
        drop(m);
        if out.is_some() {
            self.try_persist();
        }
        out
    }

    pub fn remove_task(&self, id: &str) -> bool {
        let removed = self.tasks.lock().expect("tasks mutex").remove(id).is_some();
        if removed {
            self.try_persist();
        }
        removed
    }

    pub fn insert_team(&self, name: String) -> TeamRecord {
        let id = Uuid::new_v4().to_string();
        let t = TeamRecord {
            id: id.clone(),
            name,
            member_ids: vec![],
        };
        self.teams
            .lock()
            .expect("teams mutex")
            .insert(id.clone(), t.clone());
        self.try_persist();
        t
    }

    pub fn remove_team(&self, id: &str) -> bool {
        let removed = self.teams.lock().expect("teams mutex").remove(id).is_some();
        if removed {
            self.try_persist();
        }
        removed
    }

    pub fn list_teams(&self) -> Vec<TeamRecord> {
        self.teams
            .lock()
            .expect("teams mutex")
            .values()
            .cloned()
            .collect()
    }

    /// Optional production fields for new cron jobs (`CronCreate` / scheduler).
    pub fn push_cron(&self, schedule: String, command: String) -> String {
        self.push_cron_with_options(schedule, command, CronJobCreateOptions::default())
            .id
    }

    pub fn push_cron_with_options(
        &self,
        schedule: String,
        command: String,
        opts: CronJobCreateOptions,
    ) -> CronJob {
        let id = Uuid::new_v4().to_string();
        let job = CronJob {
            id: id.clone(),
            schedule,
            command,
            session_id: opts
                .session_id
                .filter(|s| !s.trim().is_empty())
                .or_else(|| Some(Uuid::new_v4().to_string())),
            failure_destination: Some(
                opts.failure_destination
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| "log".to_string()),
            ),
            tool_profile: Some(
                opts.tool_profile
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| "default".to_string()),
            ),
            tool_allowlist: opts
                .tool_allowlist
                .filter(|list| !list.is_empty())
                .map(|list| {
                    list.into_iter()
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                }),
            project_id: opts.project_id.filter(|s| !s.trim().is_empty()),
        };
        self.crons.lock().expect("crons mutex").push(job.clone());
        self.try_persist();
        job
    }

    pub fn remove_cron(&self, id: &str) -> bool {
        let mut g = self.crons.lock().expect("crons mutex");
        let len = g.len();
        g.retain(|c| c.id != id);
        let removed = g.len() < len;
        drop(g);
        if removed {
            self.try_persist();
        }
        removed
    }

    pub fn list_crons(&self) -> Vec<CronJob> {
        self.crons.lock().expect("crons mutex").clone()
    }

    pub fn push_remote_hook(&self, url: String) {
        self.remote_hooks.lock().expect("remote mutex").push(url);
        self.try_persist();
    }

    pub fn push_message(&self, from: String, body: String) {
        self.inter_messages
            .lock()
            .expect("msg mutex")
            .push((from, body));
        self.try_persist();
    }

    pub fn list_messages(&self) -> Vec<(String, String)> {
        self.inter_messages.lock().expect("msg mutex").clone()
    }

    pub fn set_plan_mode(&self, v: bool) {
        self.mode.lock().expect("mode mutex").plan_mode = v;
        self.try_persist();
    }

    pub fn plan_mode(&self) -> bool {
        self.mode.lock().expect("mode mutex").plan_mode
    }

    pub fn set_worktree(&self, path: Option<String>) {
        self.mode.lock().expect("mode mutex").worktree_path = path;
        self.try_persist();
    }

    pub fn worktree_path(&self) -> Option<String> {
        self.mode.lock().expect("mode mutex").worktree_path.clone()
    }

    pub fn defer_tool(&self, name: String) {
        self.deferred_tool_names
            .lock()
            .expect("defer mutex")
            .push(name);
        self.try_persist();
    }

    pub fn deferred_tools(&self) -> Vec<String> {
        self.deferred_tool_names
            .lock()
            .expect("defer mutex")
            .clone()
    }

    pub fn config_set(&self, key: String, value: serde_json::Value) {
        self.config_overrides
            .lock()
            .expect("cfg mutex")
            .insert(key, value);
        self.try_persist();
    }

    pub fn config_get(&self, key: &str) -> Option<serde_json::Value> {
        self.config_overrides
            .lock()
            .expect("cfg mutex")
            .get(key)
            .cloned()
    }

    pub fn config_snapshot(&self) -> HashMap<String, serde_json::Value> {
        self.config_overrides.lock().expect("cfg mutex").clone()
    }
}

/// Read [`CronJob`] rows from a persisted orchestration file (same JSON as [`ToolServices::load_or_new`]).
/// Returns an empty list if the path is missing; returns an error if the file exists but is not valid JSON.
pub fn read_cron_jobs_from_orchestration_file(path: &Path) -> anyhow::Result<Vec<CronJob>> {
    if !path.is_file() {
        return Ok(vec![]);
    }
    let text = fs::read_to_string(path)?;
    #[derive(Deserialize)]
    struct OrchestrationCronsOnly {
        #[serde(default)]
        crons: Vec<CronJob>,
    }
    let snap: OrchestrationCronsOnly = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("invalid orchestration JSON: {e}"))?;
    Ok(snap.crons)
}

/// Append a cron job to `~/.anycode/tasks/orchestration.json` (or `path`), creating the file if needed.
pub fn append_cron_job_to_orchestration_file(
    path: &Path,
    schedule: String,
    command: String,
    opts: CronJobCreateOptions,
) -> anyhow::Result<CronJob> {
    use uuid::Uuid;
    let mut snap = if path.is_file() {
        let text = fs::read_to_string(path)?;
        serde_json::from_str::<OrchestrationSnapshotV1>(&text)
            .unwrap_or_else(|_| OrchestrationSnapshotV1::default())
    } else {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        OrchestrationSnapshotV1::default()
    };
    let job = CronJob {
        id: Uuid::new_v4().to_string(),
        schedule,
        command,
        session_id: opts
            .session_id
            .filter(|s| !s.trim().is_empty())
            .or_else(|| Some(Uuid::new_v4().to_string())),
        failure_destination: Some(
            opts.failure_destination
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "log".to_string()),
        ),
        tool_profile: Some(
            opts.tool_profile
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "default".to_string()),
        ),
        tool_allowlist: opts.tool_allowlist.filter(|list| !list.is_empty()),
        project_id: opts.project_id.filter(|s| !s.trim().is_empty()),
    };
    snap.crons.push(job.clone());
    let text = serde_json::to_string_pretty(&snap)?;
    fs::write(path, text)?;
    Ok(job)
}

#[cfg(test)]
mod orchestration_persist_tests {
    use super::*;
    use anycode_core::{plan_tree_is_empty, PlanNode, PlanStatus};
    use std::fs;

    #[test]
    fn load_or_new_corrupt_json_backup_and_empty_state() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("orchestration.json");
        let bad = "{ not valid json";
        fs::write(&path, bad).unwrap();
        let s = ToolServices::load_or_new(path.clone()).unwrap();
        assert!(s.list_tasks().is_empty(), "损坏文件应以空编排启动");
        let bak = path.with_extension("json.corrupt");
        assert!(bak.is_file(), "应写入 .json.corrupt 备份");
        assert_eq!(fs::read_to_string(&bak).unwrap(), bad);
    }

    #[test]
    fn load_or_new_roundtrip_task() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("orchestration.json");
        {
            let s = ToolServices::load_or_new(path.clone()).unwrap();
            s.insert_task("subj".into(), "desc".into(), serde_json::json!({}));
        }
        let s2 = ToolServices::load_or_new(path).unwrap();
        let list = s2.list_tasks();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].subject, "subj");
    }

    #[test]
    fn push_cron_with_options_persists_production_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("orchestration.json");
        let s = ToolServices::load_or_new(path).unwrap();
        let job = s.push_cron_with_options(
            "0 0 12 * * *".into(),
            "check health".into(),
            CronJobCreateOptions {
                session_id: Some("sess-abc".into()),
                failure_destination: Some("http".into()),
                tool_profile: Some("allowlist".into()),
                tool_allowlist: Some(vec!["FileRead".into(), "Glob".into()]),
                project_id: None,
            },
        );
        assert_eq!(job.session_id.as_deref(), Some("sess-abc"));
        assert_eq!(job.failure_destination.as_deref(), Some("http"));
        assert_eq!(job.tool_profile.as_deref(), Some("allowlist"));
        assert_eq!(
            job.tool_allowlist,
            Some(vec!["FileRead".into(), "Glob".into()])
        );
        let listed = s.list_crons();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, job.id);
    }

    #[test]
    fn read_cron_jobs_from_orchestration_file_reads_crons_field() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("orchestration.json");
        fs::write(
            &path,
            r#"{"version":1,"crons":[{"id":"j1","schedule":"0 0 12 * * *","command":"ping"}]}"#,
        )
        .unwrap();
        let jobs = super::read_cron_jobs_from_orchestration_file(&path).unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, "j1");
        assert_eq!(jobs[0].command, "ping");
    }

    #[test]
    fn load_or_new_roundtrip_plan_tree() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("orchestration.json");
        {
            let s = ToolServices::load_or_new(path.clone()).unwrap();
            s.replace_plan_tree(PlanTree {
                roots: vec![PlanNode {
                    id: "root".into(),
                    title: "Plan".into(),
                    status: PlanStatus::Pending,
                    children: vec![],
                    detail: None,
                    kind: None,
                }],
            });
        }
        let s2 = ToolServices::load_or_new(path).unwrap();
        let tree = s2.plan_tree();
        assert_eq!(tree.roots.len(), 1);
        assert_eq!(tree.roots[0].title, "Plan");
    }

    #[test]
    fn load_or_new_legacy_snapshot_without_plan_tree_field() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("orchestration.json");
        fs::write(&path, r#"{"version":1,"todos":[]}"#).unwrap();
        let s = ToolServices::load_or_new(path).unwrap();
        assert!(plan_tree_is_empty(&s.plan_tree()));
    }
}
