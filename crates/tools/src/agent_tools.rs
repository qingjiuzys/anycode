//! `Agent`、`Skill`、`SendMessage`、旧版子代理名 `Task`。
//!
//! **Claude Code `Agent` 对齐**：`subagent_type`（同 `agent_type`）、可选 `description`、可选 `cwd`（相对则相对工具工作目录，再 canonical 为绝对路径）、
//! 可选 `model`（`sonnet`/`opus`/`haiku` 或裸 id）、可选 `isolation: "worktree"`（临时 git worktree）、`run_in_background: true` 时 `tokio::spawn` 嵌套执行并立即返回 `status: started`（见 `ToolServices` 注册表与 **`TaskStop`** / **`TaskOutput`**）。
//! 成功/失败 JSON 含 `status`、`agent_id`（= `nested_task_id`）、`output_file`、`model`/`isolation` 回显、类 Claude 的 `content: [{type,text}]`。
//! `SendMessage` 接受 `to` 作为 `recipient` 别名。

use crate::services::ToolServices;
use crate::skills::{
    acceptance as skill_acceptance, extract_markdown_sections, extract_skill_body,
    load_skill_instructions, parse_skill_manifest_file, parse_skill_manifest_text,
    truncate_skill_output, SkillCatalog, SkillPermissions, MAX_SKILL_OUTPUT_BYTES,
};
use anycode_core::prelude::*;
use anycode_core::DiskTaskOutput;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// 嵌套 Agent 未指定类型时的默认子类型（与常见 `Agent`/`Task` 工具约定一致）。
const DEFAULT_SUBAGENT_AGENT_TYPE: &str = "general-purpose";

fn nested_output_log_path(task_id: Uuid) -> Option<String> {
    dirs::home_dir().map(|h| {
        DiskTaskOutput::new(h.join(".anycode").join("tasks"))
            .output_path(task_id)
            .to_string_lossy()
            .into_owned()
    })
}

/// Map Claude Code `subagent_type` strings (`Explore`, `Plan`, …) to anyCode `AgentType` ids.
/// Resolve `cwd` to an absolute path (Claude Code); relative paths are relative to the tool-call working directory.
fn resolve_agent_working_directory(base_tool_wd: &str, cwd: Option<&str>) -> String {
    let base = Path::new(base_tool_wd);
    let base_path = if base.as_os_str().is_empty() {
        std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf())
    } else {
        base.to_path_buf()
    };
    let joined = match cwd.map(str::trim).filter(|s| !s.is_empty()) {
        Some(c) => {
            let p = Path::new(c);
            if p.is_absolute() {
                p.to_path_buf()
            } else {
                base_path.join(p)
            }
        }
        None => base_path,
    };
    std::fs::canonicalize(&joined)
        .unwrap_or(joined)
        .to_string_lossy()
        .into_owned()
}

fn normalize_subagent_type_name(raw: &str) -> String {
    let t = raw.trim();
    if t.is_empty() {
        return String::new();
    }
    let lower = t.to_ascii_lowercase();
    match lower.as_str() {
        "explore" | "explorer" => "explore".to_string(),
        "plan" | "planner" => "plan".to_string(),
        "general-purpose" | "general_purpose" | "builder" => "general-purpose".to_string(),
        "goal" | "goal-runner" => "goal".to_string(),
        "workspace-assistant" | "channel-ops" | "channel" => "workspace-assistant".to_string(),
        "verification" | "claude-code-guide" | "statusline-setup" => "general-purpose".to_string(),
        _ => t.to_string(),
    }
}

/// 工具描述中追加的子代理条目上限（schema 体积控制，弱本地模型友好）。
const AGENT_CATALOG_DESCRIPTION_LIMIT: usize = 20;
/// 单条子代理描述字符上限。
const AGENT_CATALOG_DESC_CHAR_LIMIT: usize = 120;

/// `Agent`/`Task` 工具基础 schema（两个线名共用）。
fn agent_tool_base_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "prompt": { "type": "string", "description": "Task for the agent (Claude Code primary field)" },
            "task": { "type": "string", "description": "Alias of prompt" },
            "description": { "type": "string", "description": "Short human-readable summary (Claude Code style, optional)" },
            "agent_type": { "type": "string", "description": "anyCode: explore | plan | general-purpose" },
            "subagent_type": { "type": "string", "description": "Claude Code: same as agent_type (Explore, Plan, …)" },
            "cwd": { "type": "string", "description": "Working directory for the nested agent; overrides tool-call cwd when set" },
            "model": { "type": "string", "description": "Claude: sonnet | opus | haiku or explicit model id" },
            "isolation": { "type": "string", "description": "worktree — isolated git worktree under system temp" },
            "run_in_background": { "type": "boolean", "description": "When true, nested agent runs in background; use TaskOutput/TaskStop with nested_task_id" }
        },
        "anyOf": [
            { "required": ["prompt"] },
            { "required": ["task"] }
        ]
    })
}

/// 动态工具描述：基础描述 + 已注册子代理清单（可发现性）。
/// 目录为空时逐字节返回基础描述（未接 runtime 的工具测试不受影响）。
fn agent_tool_api_description(base: &str, catalog: &[(String, String)]) -> String {
    if catalog.is_empty() {
        return base.to_string();
    }
    let mut out = String::from(base);
    out.push_str("\nRegistered subagent types (pass one as `subagent_type`/`agent_type`):");
    for (id, desc) in catalog.iter().take(AGENT_CATALOG_DESCRIPTION_LIMIT) {
        let trimmed: String = desc.chars().take(AGENT_CATALOG_DESC_CHAR_LIMIT).collect();
        out.push_str(&format!("\n- {id}: {trimmed}"));
    }
    out
}

/// 动态 schema：把已注册子代理 id 清单注入 `agent_type`/`subagent_type` 的描述。
///
/// 刻意**不加** JSON-schema `enum`：`normalize_subagent_type_name` 故意接受 Claude 别名
/// （`Explore`/`Builder`/大小写变体），严格 enum 会让 strict-mode provider 拒绝这些别名，
/// 且对新注册 agent 的陈旧 schema 硬失败。描述清单兼顾发现性与别名容忍。
fn agent_tool_schema_with_catalog(
    mut base: serde_json::Value,
    catalog: &[(String, String)],
) -> serde_json::Value {
    if catalog.is_empty() {
        return base;
    }
    let id_list = catalog
        .iter()
        .map(|(id, _)| id.as_str())
        .collect::<Vec<_>>()
        .join(" | ");
    if let Some(props) = base.get_mut("properties").and_then(|p| p.as_object_mut()) {
        if let Some(at) = props.get_mut("agent_type").and_then(|v| v.as_object_mut()) {
            at.insert(
                "description".into(),
                json!(format!("anyCode agent id: {id_list}")),
            );
        }
        if let Some(st) = props
            .get_mut("subagent_type")
            .and_then(|v| v.as_object_mut())
        {
            st.insert(
                "description".into(),
                json!(format!(
                    "Claude Code alias accepted; anyCode agent id: {id_list}"
                )),
            );
        }
    }
    base
}

struct SubAgentDepthGuard<'a> {
    services: &'a ToolServices,
    child_id: Uuid,
    disarmed: bool,
}

/// 前台嵌套调用的协作取消守卫（ADR 010）：所在 tool future 被 drop（父任务
/// 取消时 Cancel-policy select 抢占）即置位，子代理在下一边界退出。
struct ForegroundCancelGuard(Arc<AtomicBool>);

impl Drop for ForegroundCancelGuard {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

impl<'a> SubAgentDepthGuard<'a> {
    fn new(services: &'a ToolServices, child_id: Uuid) -> Self {
        Self {
            services,
            child_id,
            disarmed: false,
        }
    }

    fn disarm(&mut self) {
        self.disarmed = true;
    }
}

impl Drop for SubAgentDepthGuard<'_> {
    fn drop(&mut self) {
        if !self.disarmed {
            self.services.leave_sub_agent_depth(self.child_id);
        }
    }
}

#[derive(Deserialize)]
struct AgentToolIn {
    #[serde(default)]
    prompt: Option<String>,
    #[serde(default)]
    task: Option<String>,
    /// anyCode: `agent_type`. Claude Code: `subagent_type` (e.g. Explore, Plan, general-purpose).
    #[serde(default, alias = "subagent_type")]
    agent_type: Option<String>,
    /// Claude Code: short human-readable summary of what the sub-agent will do (optional here for compatibility).
    #[serde(default)]
    description: Option<String>,
    /// Claude Code: working directory for the nested agent (overrides tool-call `working_directory` when set).
    #[serde(default)]
    cwd: Option<String>,
    /// Claude Code: `sonnet` | `opus` | `haiku` or a raw model id.
    #[serde(default)]
    model: Option<String>,
    /// Claude Code: `worktree` for isolated git worktree under the temp directory.
    #[serde(default)]
    isolation: Option<String>,
    /// Claude Code: when `true`, nested run continues in a background task; use `TaskOutput` / `TaskStop` with `nested_task_id`.
    #[serde(default)]
    run_in_background: Option<bool>,
    /// 可选 JSON schema：子代理须以 `StructuredOutput` 收尾，父结果 JSON 带 `structured_output`。
    #[serde(default)]
    output_schema: Option<serde_json::Value>,
}

/// 结构化输出 schema 注入字符上限（防 schema 膨胀占满上下文）。
const STRUCTURED_OUTPUT_SCHEMA_CAP: usize = 16 * 1024;

/// 结构化输出契约注入文本（经 `context_injections` 以 User 上下文消息进入嵌套会话）。
pub const STRUCTURED_OUTPUT_INSTRUCTION: &str = "## Structured output contract\n\
You must finish this task by calling the StructuredOutput tool exactly once with a JSON object \
that matches this schema:\n";

pub struct AgentTool {
    services: Arc<ToolServices>,
    policy: SecurityPolicy,
}

impl AgentTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            services,
            policy: SecurityPolicy::sensitive_mutation(),
        }
    }

    async fn run_sub_agent(
        &self,
        input: ToolInput,
        default_agent_type: &str,
    ) -> Result<ToolOutput, CoreError> {
        let start = Instant::now();
        let exe = match self.services.sub_agent_executor() {
            Some(e) => e,
            None => {
                return Ok(ToolOutput {
                    result: serde_json::json!({
                        "error": "Sub-agent runner not attached (internal bootstrap order)"
                    }),
                    error: Some("no sub-agent runner".into()),
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        };

        let v: AgentToolIn = serde_json::from_value(input.input.clone())?;

        let prompt = v
            .prompt
            .or(v.task)
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| {
                CoreError::LLMError(
                    "non-empty `prompt` or `task` is required (Claude Code: `prompt`)".into(),
                )
            })?;
        if v.run_in_background == Some(true) && self.services.forbid_detached_nested() {
            return Ok(ToolOutput {
                result: serde_json::json!({
                    "error": "unified kernel refuses detached background Agent; use HarnessAgentSpawn/Join/Cancel"
                }),
                error: Some("detached background Agent is not allowed".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        let agent_type_owned = v
            .agent_type
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(normalize_subagent_type_name)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| default_agent_type.to_string());

        let base_wd = input
            .working_directory
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| ".".to_string());
        let wd = resolve_agent_working_directory(&base_wd, v.cwd.as_deref());

        let desc = v
            .description
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let model = v
            .model
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let isolation = v
            .isolation
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let model_echo = model.clone();
        let isolation_echo = isolation.clone();

        let child_id = Uuid::new_v4();
        if !self.services.try_spawn_sub_agent(input.task_id, child_id) {
            return Ok(ToolOutput {
                result: serde_json::json!({ "error": "sub-agent nesting depth exceeded" }),
                error: Some("max sub-agent depth".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
        let mut depth_guard = SubAgentDepthGuard::new(self.services.as_ref(), child_id);
        let (parent_deny_names, parent_deny_prefixes) =
            self.services.parent_task_tool_deny(input.task_id);
        let mut invoke = NestedTaskInvoke {
            agent_type: AgentType::new(agent_type_owned.clone()),
            prompt: prompt.clone(),
            working_directory: wd.clone(),
            model,
            isolation,
            // 前后台共用同一个预分配 task_id：结构化输出 schema 槽 / 捕获按 task 键控，
            // 后台 job 直接复用此 id（不再覆盖），schema 槽键始终一致。
            task_id: Some(child_id),
            cancel: None,
            tool_deny_names: parent_deny_names,
            tool_deny_prefixes: parent_deny_prefixes,
            context_injections: vec![],
            // Step 3b 嵌套可观测性：父任务的 live trace 通道经 ToolServices 键控 map 取得，
            // 子任务事件由 nested_task 的 forwarder 包成 Subagent 后转发。
            live_trace_tx: input
                .task_id
                .and_then(|id| self.services.live_trace_tx_for(id)),
            parent_task_id: input.task_id,
            budget: self.services.task_budget(input.task_id),
        };

        // 结构化输出契约：schema 槽 + 指令注入（子代理以 StructuredOutput 收尾）。
        let schema_task_id: Option<Uuid> = if let Some(schema) = v.output_schema.clone() {
            let tid = invoke.task_id.expect("task_id pre-assigned");
            self.services
                .set_structured_output_schema(tid, schema.clone());
            let schema_text = serde_json::to_string_pretty(&schema).unwrap_or_default();
            let capped: String = schema_text
                .chars()
                .take(STRUCTURED_OUTPUT_SCHEMA_CAP)
                .collect();
            invoke
                .context_injections
                .push(format!("{STRUCTURED_OUTPUT_INSTRUCTION}{capped}"));
            Some(tid)
        } else {
            None
        };

        if v.run_in_background == Some(true) {
            // 后台 job 直接复用预分配的 task_id：结构化输出 schema 槽 / 捕获已按此 id 键控，无需重键。
            let task_id = invoke.task_id.expect("task_id pre-assigned");
            let job = self.services.insert_background_agent_job(task_id);
            let mut invoke_bg = invoke;
            invoke_bg.cancel = Some(job.coop_cancel.clone());
            let output_file = nested_output_log_path(task_id);
            let nested_task_id = task_id.to_string();
            let exe_c = exe.clone();
            let services_c = self.services.clone();
            let desc_c = desc.clone();
            let agent_type_c = agent_type_owned.clone();
            let prompt_c = prompt.clone();
            let model_echo_c = model_echo.clone();
            let isolation_echo_c = isolation_echo.clone();
            let wd_c = wd.clone();

            depth_guard.disarm();
            let handle = tokio::spawn(async move {
                struct BgDepthGuard {
                    services: Arc<ToolServices>,
                    task_id: Uuid,
                }
                impl Drop for BgDepthGuard {
                    fn drop(&mut self) {
                        self.services.leave_sub_agent_depth(self.task_id);
                        self.services
                            .finalize_background_if_still_running(self.task_id);
                    }
                }
                let _g = BgDepthGuard {
                    services: services_c.clone(),
                    task_id,
                };
                let res = exe_c.run_nested_task(invoke_bg).await;
                // 结构化输出捕获保留给 TaskOutput 一次性取用（不在这里丢弃）；
                // schema 槽始终回收。未轮询的残留以进程内后台任务数为界。
                services_c.clear_structured_output_schema(task_id);
                services_c.finish_background_agent(task_id, res);
            });
            job.set_abort(handle.abort_handle());

            return Ok(ToolOutput {
                result: json!({
                    "status": "started",
                    "nested_task_id": &nested_task_id,
                    "agent_id": &nested_task_id,
                    "output_file": output_file,
                    "background": true,
                    "hint": "Poll TaskOutput with id=nested_task_id; cancel with TaskStop on the same id (best-effort abort).",
                    "agent_type": &agent_type_c,
                    "subagent_type_resolved": &agent_type_c,
                    "working_directory": &wd_c,
                    "prompt": &prompt_c,
                    "description": desc_c.clone(),
                    "model": model_echo_c.clone(),
                    "isolation": isolation_echo_c.clone(),
                }),
                error: None,
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        let invoke_task_id = invoke.task_id;
        // 前台嵌套取消（ADR 010）：per-call 取消标志。父任务取消时 tool_dispatch 的
        // Cancel-policy select 抢占并 drop 本 future，guard 置位 → 子代理在下一个
        // 协作取消边界（turn/工具）经 nested_coop_cancelled 退出为 cancelled。
        let fg_cancel = Arc::new(AtomicBool::new(false));
        invoke.cancel = Some(Arc::clone(&fg_cancel));
        let _fg_cancel_guard = ForegroundCancelGuard(fg_cancel);
        let run_res = exe.run_nested_task(invoke).await;
        // 无论成败都回收结构化输出槽（schema + 捕获），避免泄漏到后续任务。
        let structured_out =
            invoke_task_id.and_then(|tid| self.services.take_structured_output(tid));
        if let Some(tid) = schema_task_id {
            self.services.clear_structured_output_schema(tid);
        }
        let NestedTaskRun { task_id, result } = match run_res {
            Ok(run) => run,
            Err(CoreError::AgentNotFound(_)) => {
                // type-miss 遥测 + 自愈提示：列出已知子代理，模型下一轮可纠正。
                tracing::warn!(event = "subagent_type_miss", agent_type = %agent_type_owned);
                let known_agents: Vec<String> =
                    exe.agent_catalog().into_iter().map(|(id, _)| id).collect();
                return Ok(ToolOutput {
                    result: json!({
                        "status": "failed",
                        "error": format!("unknown subagent_type: {agent_type_owned}"),
                        "known_agents": known_agents,
                        "structured_output": structured_out,
                    }),
                    error: Some(format!("unknown subagent_type: {agent_type_owned}")),
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
            Err(e) => return Err(e),
        };
        let nested_task_id = task_id.to_string();
        let output_file = nested_output_log_path(task_id);

        match result {
            TaskResult::Success { output, artifacts } => {
                let content_text = output.clone();
                // artifacts 元数据透传（不含内容）：父模型可感知子代理产出了哪些文件。
                let artifacts_meta: Vec<serde_json::Value> = artifacts
                    .iter()
                    .map(|a| {
                        json!({
                            "name": a.name,
                            "kind": a.kind,
                            "path": a.path,
                            "title": a.title,
                            "bytes": a.bytes,
                            "mime": a.mime,
                        })
                    })
                    .collect();
                Ok(ToolOutput {
                    result: json!({
                        "status": "completed",
                        "output": output,
                        "content": [{ "type": "text", "text": content_text }],
                        "artifacts_count": artifacts.len(),
                        "artifacts": artifacts_meta,
                        "structured_output": structured_out,
                        "nested_task_id": &nested_task_id,
                        "agent_id": &nested_task_id,
                        "output_file": output_file,
                        "agent_type": &agent_type_owned,
                        "subagent_type_resolved": &agent_type_owned,
                        "working_directory": &wd,
                        "prompt": &prompt,
                        "description": desc.clone(),
                        "model": model_echo.clone(),
                        "isolation": isolation_echo.clone(),
                    }),
                    error: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                })
            }
            TaskResult::Failure { error, details } => Ok(ToolOutput {
                result: json!({
                    "status": "failed",
                    "error": error,
                    "details": details,
                    "structured_output": structured_out,
                    "nested_task_id": &nested_task_id,
                    "agent_id": &nested_task_id,
                    "output_file": output_file,
                    "agent_type": &agent_type_owned,
                    "subagent_type_resolved": &agent_type_owned,
                    "working_directory": &wd,
                    "prompt": &prompt,
                    "description": desc.clone(),
                    "model": model_echo.clone(),
                    "isolation": isolation_echo.clone(),
                }),
                error: Some("subtask failed".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            }),
            TaskResult::Partial { success, remaining } => Ok(ToolOutput {
                result: json!({
                    "status": "partial",
                    "partial_success": success,
                    "remaining": remaining,
                    "structured_output": structured_out,
                    "nested_task_id": &nested_task_id,
                    "agent_id": &nested_task_id,
                    "output_file": output_file,
                    "agent_type": &agent_type_owned,
                    "subagent_type_resolved": &agent_type_owned,
                    "working_directory": &wd,
                    "prompt": &prompt,
                    "description": desc.clone(),
                    "model": model_echo.clone(),
                    "isolation": isolation_echo.clone(),
                }),
                error: Some("subtask partial".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            }),
        }
    }
}

#[async_trait]
impl Tool for AgentTool {
    fn name(&self) -> &str {
        "Agent"
    }
    fn description(&self) -> &str {
        "Nested agent run (same AgentRuntime as the host). Claude Code parity: `prompt`/`task`, `subagent_type`/`agent_type`, `description`, `cwd` (absolute path after resolve), `model` (sonnet|opus|haiku or raw id), `isolation: \"worktree\"` (temp git worktree, auto-removed). `run_in_background: true` returns immediately with `status: started` and the same `nested_task_id`/`output_file` hints; poll `TaskOutput`, cancel with `TaskStop` (best-effort). Sync runs: `status` completed/failed/partial, `content` on success. Depth ~6 max."
    }
    fn api_tool_description(&self) -> String {
        agent_tool_api_description(self.description(), &self.services.agent_catalog())
    }
    fn schema(&self) -> serde_json::Value {
        agent_tool_schema_with_catalog(agent_tool_base_schema(), &self.services.agent_catalog())
    }
    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Default
    }
    fn security_policy(&self) -> Option<&SecurityPolicy> {
        Some(&self.policy)
    }
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        self.run_sub_agent(input, DEFAULT_SUBAGENT_AGENT_TYPE).await
    }
}

#[derive(Deserialize)]
struct SkillIn {
    #[serde(default)]
    name: String,
    #[serde(default)]
    args: Vec<String>,
}

#[derive(Deserialize)]
struct SkillSearchIn {
    #[serde(default)]
    query: String,
    #[serde(default)]
    id: String,
}

pub struct SkillSearchTool {
    services: Arc<ToolServices>,
}

impl SkillSearchTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self { services }
    }
}

#[async_trait]
impl Tool for SkillSearchTool {
    fn name(&self) -> &str {
        "SkillSearch"
    }
    fn description(&self) -> &str {
        "Search discoverable skills allowed by project and agent governance. Pass `query` for substring match on id/description, or `id` for exact lookup."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Substring match on skill id or description" },
                "id": { "type": "string", "description": "Exact skill id lookup" }
            }
        })
    }
    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Auto
    }
    fn security_policy(&self) -> Option<&SecurityPolicy> {
        None
    }
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let start = Instant::now();
        let v: SkillSearchIn = serde_json::from_value(input.input).unwrap_or(SkillSearchIn {
            query: String::new(),
            id: String::new(),
        });
        let id_exact = v.id.trim();
        let query = v.query.trim().to_ascii_lowercase();
        if id_exact.is_empty() && query.is_empty() {
            return Ok(ToolOutput {
                result: serde_json::json!({ "error": "query or id required" }),
                error: Some("query or id required".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
        let cat = &self.services.skill_catalog;
        let mut matches: Vec<serde_json::Value> = Vec::new();
        for skill in cat.metas() {
            if !self.services.is_skill_allowed(&skill.id) {
                continue;
            }
            let hit = if !id_exact.is_empty() {
                skill.id == id_exact
            } else {
                skill.id.to_ascii_lowercase().contains(&query)
                    || skill.description.to_ascii_lowercase().contains(&query)
            };
            if !hit {
                continue;
            }
            matches.push(serde_json::json!({
                "id": skill.id,
                "description": skill.description,
                "has_run": skill.has_run,
            }));
        }
        matches.sort_by(|a, b| {
            a.get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .cmp(b.get("id").and_then(|v| v.as_str()).unwrap_or(""))
        });
        Ok(ToolOutput {
            result: serde_json::json!({ "skills": matches, "count": matches.len() }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

pub struct SkillTool {
    services: Arc<ToolServices>,
    policy: SecurityPolicy,
}

impl SkillTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            services,
            policy: SecurityPolicy::sensitive_mutation(),
        }
    }
}

#[async_trait]
impl Tool for SkillTool {
    fn name(&self) -> &str {
        "Skill"
    }
    fn description(&self) -> &str {
        "Run a skill from a discovered skill directory. Use SkillSearch first when unsure. With a `run` script: pass `name` and optional `args`. Documentation-only skills: pass `name` only to load `SKILL.md` instructions."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "args": { "type": "array", "items": { "type": "string" } }
            },
            "required": ["name"]
        })
    }
    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Default
    }
    fn security_policy(&self) -> Option<&SecurityPolicy> {
        Some(&self.policy)
    }
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let start = Instant::now();
        let v: SkillIn = serde_json::from_value(input.input)?;
        let skill_name = v.name.trim();
        if skill_name.is_empty() {
            return Ok(ToolOutput {
                result: serde_json::json!({ "error": "name required" }),
                error: Some("name required".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
        if !SkillCatalog::is_valid_skill_id(skill_name) {
            return Ok(ToolOutput {
                result: serde_json::json!({
                    "error": "invalid skill id",
                    "hint": "use only letters, digits, . _ -"
                }),
                error: Some("invalid skill id".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
        if !self.services.is_skill_allowed(skill_name) {
            return Ok(ToolOutput {
                result: serde_json::json!({
                    "error": "skill blocked by governance",
                    "skill": skill_name,
                    "hint": "Enable this skill for the project or agent profile in Dashboard / config.json"
                }),
                error: Some("skill blocked by governance".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
        let cat = &self.services.skill_catalog;
        let task_cwd = input
            .working_directory
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(Path::new);
        let Some(root) = cat.resolve_skill_root(skill_name, task_cwd) else {
            return Ok(ToolOutput {
                result: serde_json::json!({
                    "error": "skill not found",
                    "hint": "Add SKILL.md under ~/.anycode/skills/<name>/ or <cwd>/skills/<name>/; optional executable `run`."
                }),
                error: Some("skill not found".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        };
        // Skill App HITL is host-driven (auto-present on ppt prompts). Do not
        // re-present here — that re-opens the studio after the user already locked a brief.
        let runner = root.join("run");
        if !runner.is_file() {
            let Some(body) = load_skill_instructions(&root) else {
                return Ok(ToolOutput {
                    result: serde_json::json!({
                        "error": "skill run script not found and SKILL.md body empty",
                        "expected_path": runner.to_string_lossy(),
                    }),
                    error: Some("skill has no run script".into()),
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            };
            return Ok(ToolOutput {
                result: serde_json::json!({
                    "skill": skill_name,
                    "mode": "instructions",
                    "instructions": body,
                }),
                error: None,
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
        if v.args.is_empty() {
            if let Some(body) = load_skill_instructions(&root) {
                return Ok(ToolOutput {
                    result: serde_json::json!({
                        "skill": skill_name,
                        "mode": "usage",
                        "hint": "This skill has a run script; pass required paths in args (string array). Working directory is the task cwd (ANYCODE_WORKING_DIR), not the skill directory.",
                        "instructions": body,
                    }),
                    error: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        }
        let manifest_pre = parse_skill_manifest_file(&root.join("SKILL.md"));
        let perms = SkillPermissions::from_value(
            manifest_pre.as_ref().and_then(|m| m.permissions.as_ref()),
        );
        if perms.network_denied() {
            return Ok(ToolOutput {
                result: serde_json::json!({
                    "error": "skill run blocked: permissions.network=false",
                    "skill": skill_name,
                    "hint": "This skill forbids network. Load instructions without args, or set permissions.network=true in SKILL.md."
                }),
                error: Some("skill run blocked: permissions.network=false".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
        // Task working directory (not skill root) — documented contract.
        let skill_root = std::fs::canonicalize(&root).unwrap_or_else(|_| root.clone());
        let cwd = task_cwd
            .map(|s| std::fs::canonicalize(s).unwrap_or_else(|_| PathBuf::from(s)))
            .unwrap_or_else(|| skill_root.clone());
        if let Err(e) = crate::sandbox::resolve_under_workdir(cwd.to_string_lossy().as_ref(), ".") {
            return Ok(ToolOutput {
                result: serde_json::json!({
                    "error": format!("skill cwd outside sandbox: {e}"),
                    "cwd": cwd.to_string_lossy(),
                }),
                error: Some("skill cwd outside sandbox".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
        let timeout = Duration::from_millis(cat.run_timeout_ms.max(1_000));
        let mut cmd = tokio::process::Command::new(&runner);
        cmd.current_dir(&cwd);
        cmd.env("ANYCODE_SKILL_DIR", &skill_root);
        cmd.env("ANYCODE_WORKING_DIR", &cwd);
        cmd.args(&v.args);
        cmd.kill_on_drop(true);
        // Always scrub env unless catalog explicitly disables minimal_env.
        let use_minimal = cat.minimal_env;
        if use_minimal {
            cmd.env_clear();
            for k in ["PATH", "HOME", "USER", "TMPDIR", "SYSTEMROOT", "LANG"] {
                if let Ok(val) = std::env::var(k) {
                    cmd.env(k, val);
                }
            }
            cmd.env("ANYCODE_SKILL_DIR", &skill_root);
            cmd.env("ANYCODE_WORKING_DIR", &cwd);
        }
        let run = cmd.output();
        let out = match tokio::time::timeout(timeout, run).await {
            Ok(Ok(o)) => o,
            Ok(Err(e)) => {
                return Err(CoreError::LLMError(format!("skill run: {e}")));
            }
            Err(_) => {
                return Ok(ToolOutput {
                    result: serde_json::json!({ "error": "skill timed out", "timeout_ms": cat.run_timeout_ms }),
                    error: Some("skill timed out".into()),
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        };
        let ok = out.status.success();
        let mut stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        let mut stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        stdout = truncate_skill_output(stdout, MAX_SKILL_OUTPUT_BYTES);
        stderr = truncate_skill_output(stderr, MAX_SKILL_OUTPUT_BYTES);
        let exit_code = out.status.code();

        // P0.4 skill contract: re-attach the SOP input/output/acceptance
        // contract and run declarative acceptance checks. Weak chat models
        // follow checklists, not vibes — failed checks become a tool error so
        // the agent loop turns them into a repair turn.
        let manifest = parse_skill_manifest_file(&root.join("SKILL.md"));
        let sop_contract = std::fs::read_to_string(root.join("SKILL.md"))
            .ok()
            .and_then(|text| {
                extract_markdown_sections(
                    &extract_skill_body(&text),
                    &["## Inputs", "## Output", "## Acceptance Checks"],
                    4 * 1024,
                )
            });
        let acceptance: Vec<skill_acceptance::AcceptanceOutcome> = manifest
            .as_ref()
            .map(|m| skill_acceptance::run_acceptance_checks(&m.acceptance, &cwd))
            .unwrap_or_default();
        let acceptance_failure = skill_acceptance::summarize_failures(&acceptance);

        let error = if !ok {
            let stderr_line = stderr
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .unwrap_or("");
            let msg = if !stderr_line.is_empty() {
                format!(
                    "skill exited with code {}: {}",
                    exit_code
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "?".into()),
                    stderr_line
                )
            } else {
                format!(
                    "skill exited with code {}",
                    exit_code
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "?".into())
                )
            };
            Some(msg)
        } else {
            acceptance_failure
                .as_ref()
                .map(|f| format!("skill acceptance checks failed: {f}"))
        };
        Ok(ToolOutput {
            result: serde_json::json!({
                "stdout": stdout,
                "stderr": stderr,
                "code": exit_code,
                "sop_contract": sop_contract,
                "acceptance": acceptance,
            }),
            error,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[derive(Deserialize)]
struct ProposeSkillItem {
    #[serde(default)]
    name: String,
    #[serde(default)]
    kind: String, // "new" | "improvement"
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    description: String,
    #[serde(default)]
    evidence: Option<String>,
    #[serde(default, alias = "skillMd")]
    skill_md: Option<String>,
}

#[derive(Deserialize)]
struct ProposeSkillsIn {
    #[serde(default)]
    proposals: Vec<ProposeSkillItem>,
}

/// 技能提案工具：校验 1-3 个技能提案（新增 / 改进），返回结构化评审结果。
/// 只读——不安装、不写盘；对齐 Claude Code `ProposeSkills` 语义。
pub struct ProposeSkillsTool {
    services: Arc<ToolServices>,
}

impl ProposeSkillsTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self { services }
    }
}

#[async_trait]
impl Tool for ProposeSkillsTool {
    fn name(&self) -> &str {
        "ProposeSkills"
    }
    fn description(&self) -> &str {
        "Validate 1-3 skill proposals (kind=new or kind=improvement). Each proposal: `name`, `kind`, optional `target` (existing skill id for improvement), `description`, optional `evidence`, optional `skillMd` draft. Returns per-proposal acceptance, issues (invalid id, unknown target, existing-name conflict, unparsable SKILL.md frontmatter), and a count summary. Read-only; it does not install skills."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "proposals": {
                    "type": "array",
                    "minItems": 1,
                    "maxItems": 3,
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string", "description": "Proposed skill id (letters, digits, dot, underscore, dash)" },
                            "kind": { "type": "string", "enum": ["new", "improvement"], "description": "new = create a skill; improvement = change an existing one" },
                            "target": { "type": "string", "description": "Existing skill id when kind=improvement" },
                            "description": { "type": "string" },
                            "evidence": { "type": "string", "description": "Why this skill is needed" },
                            "skillMd": { "type": "string", "description": "Optional draft SKILL.md content" }
                        },
                        "required": ["name", "kind", "description"]
                    }
                }
            },
            "required": ["proposals"]
        })
    }
    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Auto
    }
    fn security_policy(&self) -> Option<&SecurityPolicy> {
        None
    }
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let start = Instant::now();
        let v: ProposeSkillsIn =
            serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;
        if v.proposals.is_empty() {
            return Ok(ToolOutput {
                result: serde_json::json!({ "error": "proposals required (1-3)" }),
                error: Some("proposals required (1-3)".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
        if v.proposals.len() > 3 {
            return Ok(ToolOutput {
                result: serde_json::json!({ "error": "at most 3 proposals per call" }),
                error: Some("at most 3 proposals per call".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
        let cat = &self.services.skill_catalog;
        let mut reviews: Vec<serde_json::Value> = Vec::new();
        for p in &v.proposals {
            let name = p.name.trim();
            let kind = p.kind.trim();
            let mut issues: Vec<serde_json::Value> = Vec::new();
            if name.is_empty() {
                issues.push(json!({ "severity": "error", "message": "name is required" }));
            } else if !SkillCatalog::is_valid_skill_id(name) {
                issues.push(json!({
                    "severity": "error",
                    "message": "invalid skill id (letters, digits, . _ - only)"
                }));
            }
            if kind != "new" && kind != "improvement" {
                issues.push(json!({
                    "severity": "error",
                    "message": "kind must be new or improvement"
                }));
            }
            let target = p.target.as_deref().unwrap_or("").trim();
            if kind == "improvement" && target.is_empty() {
                issues.push(json!({
                    "severity": "error",
                    "message": "target required when kind=improvement"
                }));
            }
            if p.description.trim().is_empty() {
                issues.push(json!({
                    "severity": "warn",
                    "message": "description is empty"
                }));
            }
            // 名称冲突 / target 存在性
            let name_exists = cat.metas().iter().any(|m| m.id == name);
            let target_exists = !target.is_empty() && cat.metas().iter().any(|m| m.id == target);
            if kind == "new" && name_exists {
                issues.push(json!({
                    "severity": "warn",
                    "message": "skill with this name already exists; consider kind=improvement"
                }));
            }
            if kind == "improvement" && !target.is_empty() && !target_exists {
                issues.push(json!({
                    "severity": "error",
                    "message": format!("target skill not found: {target}")
                }));
            }
            // skillMd 草案可解析性（frontmatter）
            let skill_md_valid = match p.skill_md.as_deref() {
                Some(md) if md.trim().is_empty() => {
                    issues.push(json!({
                        "severity": "warn",
                        "message": "skillMd is empty"
                    }));
                    false
                }
                Some(md) => match parse_skill_manifest_text(md) {
                    Some(_) => true,
                    None => {
                        issues.push(json!({
                            "severity": "warn",
                            "message": "skillMd frontmatter is not parseable"
                        }));
                        false
                    }
                },
                None => false,
            };
            let accepted = !issues.iter().any(|i| i["severity"] == "error");
            reviews.push(json!({
                "name": name,
                "kind": kind,
                "target": target,
                "description": p.description,
                "evidence": p.evidence,
                "accepted": accepted,
                "issues": issues,
                "nameExists": name_exists,
                "targetExists": target_exists,
                "skillMdValid": skill_md_valid,
            }));
        }
        let accepted_count = reviews
            .iter()
            .filter(|r| r.get("accepted").and_then(|v| v.as_bool()).unwrap_or(false))
            .count();
        Ok(ToolOutput {
            result: serde_json::json!({
                "reviews": reviews,
                "count": reviews.len(),
                "accepted": accepted_count,
            }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[derive(Deserialize)]
struct MsgIn {
    /// anyCode: `recipient`. Claude Code swarm: `to`.
    #[serde(default, alias = "to")]
    recipient: String,
    #[serde(default)]
    message: String,
    #[serde(default)]
    body: String,
}

pub struct SendMessageTool {
    services: Arc<ToolServices>,
    policy: SecurityPolicy,
}

impl SendMessageTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            services,
            policy: SecurityPolicy::sensitive_mutation(),
        }
    }
}

#[async_trait]
impl Tool for SendMessageTool {
    fn name(&self) -> &str {
        "SendMessage"
    }

    fn description(&self) -> &str {
        "Queue a message for another agent/recipient key (`recipient` or Claude-style `to`). Body: `message` or `body`. Persists with orchestration state when ~/.anycode is available."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "recipient": { "type": "string", "description": "Recipient key / agent name" },
                "to": { "type": "string", "description": "Claude Code alias for recipient" },
                "message": { "type": "string" },
                "body": { "type": "string" }
            }
        })
    }

    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Default
    }

    fn security_policy(&self) -> Option<&SecurityPolicy> {
        Some(&self.policy)
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let start = Instant::now();
        let m: MsgIn = serde_json::from_value(input.input).unwrap_or(MsgIn {
            recipient: String::new(),
            message: String::new(),
            body: String::new(),
        });
        let recipient = m.recipient.trim().to_string();
        if recipient.is_empty() {
            return Ok(ToolOutput {
                result: json!({
                    "error": "recipient or `to` must be a non-empty string (Claude Code: `to`)"
                }),
                error: Some("invalid SendMessage input".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
        let text = if !m.body.is_empty() {
            m.body
        } else {
            m.message
        };
        self.services.push_message(recipient.clone(), text.clone());
        Ok(ToolOutput {
            result: serde_json::json!({ "queued": true, "preview": text.chars().take(200).collect::<String>() }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

pub struct LegacyTaskAgentTool {
    inner: AgentTool,
    policy: SecurityPolicy,
}

impl LegacyTaskAgentTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            inner: AgentTool::new(services),
            policy: SecurityPolicy::sensitive_mutation(),
        }
    }
}

#[async_trait]
impl Tool for LegacyTaskAgentTool {
    fn name(&self) -> &str {
        "Task"
    }
    fn description(&self) -> &str {
        "Legacy wire name `Task` (Claude Code): same as `Agent` — `prompt`/`task`, optional `subagent_type`, `description`, `cwd` (absolute after resolve), `model`, `isolation` (`worktree`); `run_in_background: true` spawns async nested run (TaskOutput / TaskStop)."
    }
    fn api_tool_description(&self) -> String {
        agent_tool_api_description(self.description(), &self.inner.services.agent_catalog())
    }
    fn schema(&self) -> serde_json::Value {
        agent_tool_schema_with_catalog(
            agent_tool_base_schema(),
            &self.inner.services.agent_catalog(),
        )
    }
    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Default
    }
    fn security_policy(&self) -> Option<&SecurityPolicy> {
        Some(&self.policy)
    }
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        self.inner
            .run_sub_agent(input, DEFAULT_SUBAGENT_AGENT_TYPE)
            .await
    }
}

#[cfg(test)]
mod claude_compat_tests {
    use super::normalize_subagent_type_name;

    #[test]
    fn normalizes_claude_builtin_casing() {
        assert_eq!(normalize_subagent_type_name("Explore"), "explore");
        assert_eq!(normalize_subagent_type_name("Plan"), "plan");
        assert_eq!(
            normalize_subagent_type_name("general-purpose"),
            "general-purpose"
        );
        assert_eq!(normalize_subagent_type_name("Builder"), "general-purpose");
        assert_eq!(
            normalize_subagent_type_name("Verification"),
            "general-purpose"
        );
    }
}

#[cfg(test)]
mod agent_catalog_tests {
    use super::{AgentTool, LegacyTaskAgentTool};
    use crate::services::ToolServices;
    use anycode_core::prelude::*;
    use async_trait::async_trait;
    use std::sync::Arc;

    struct CatalogEx {
        catalog: Vec<(String, String)>,
    }

    #[async_trait]
    impl SubAgentExecutor for CatalogEx {
        async fn run_nested_task(
            &self,
            _invoke: NestedTaskInvoke,
        ) -> Result<NestedTaskRun, CoreError> {
            Err(CoreError::LLMError("unused in catalog tests".into()))
        }
        fn agent_catalog(&self) -> Vec<(String, String)> {
            self.catalog.clone()
        }
    }

    fn services_with_catalog(catalog: Vec<(String, String)>) -> Arc<ToolServices> {
        let services = Arc::new(ToolServices::default());
        services.attach_sub_agent_executor(Arc::new(CatalogEx { catalog }));
        services
    }

    #[test]
    fn agent_tool_description_and_schema_expose_registered_agents() {
        let services = services_with_catalog(vec![(
            "sql-reviewer".into(),
            "Reviews SQL migrations".into(),
        )]);
        let tool = AgentTool::new(services);
        let desc = tool.api_tool_description();
        assert!(desc.contains("sql-reviewer"), "desc: {desc}");
        assert!(desc.contains("Reviews SQL migrations"));
        assert!(desc.starts_with(tool.description()));
        let schema = tool.schema();
        let at = schema["properties"]["agent_type"]["description"]
            .as_str()
            .unwrap();
        let st = schema["properties"]["subagent_type"]["description"]
            .as_str()
            .unwrap();
        assert!(at.contains("sql-reviewer"), "agent_type desc: {at}");
        assert!(st.contains("sql-reviewer"), "subagent_type desc: {st}");
        // 刻意不加严格 enum（容忍 Claude 别名与大小写变体）
        assert!(schema["properties"]["agent_type"].get("enum").is_none());
    }

    #[test]
    fn empty_catalog_keeps_static_surface() {
        // 目录为空（含未接 runtime）→ 描述逐字节不变
        let services = services_with_catalog(vec![]);
        let tool = AgentTool::new(services);
        assert_eq!(tool.api_tool_description(), tool.description());

        let bare = Arc::new(ToolServices::default());
        assert!(bare.agent_catalog().is_empty());
        let task_tool = LegacyTaskAgentTool::new(bare);
        assert_eq!(task_tool.api_tool_description(), task_tool.description());
    }

    #[test]
    fn legacy_task_tool_also_exposes_catalog() {
        let services =
            services_with_catalog(vec![("researcher".into(), "Industry research".into())]);
        let tool = LegacyTaskAgentTool::new(services);
        assert!(tool.api_tool_description().contains("researcher"));
        let schema = tool.schema();
        let st = schema["properties"]["subagent_type"]["description"]
            .as_str()
            .unwrap();
        assert!(st.contains("researcher"), "subagent_type desc: {st}");
    }
}

#[cfg(test)]
mod background_agent_tests {
    use super::AgentTool;
    use crate::orchestration::{TaskOutputTool, TaskStopTool};
    use crate::services::ToolServices;
    use anycode_core::prelude::*;
    use async_trait::async_trait;
    use serde_json::json;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    use std::time::Duration;
    use uuid::Uuid;

    struct DelayedOkEx {
        delay_ms: u64,
        #[allow(dead_code)]
        calls: AtomicU32,
    }

    #[async_trait]
    impl SubAgentExecutor for DelayedOkEx {
        async fn run_nested_task(
            &self,
            invoke: NestedTaskInvoke,
        ) -> Result<NestedTaskRun, CoreError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;
            let task_id = invoke
                .task_id
                .ok_or_else(|| CoreError::LLMError("expected task_id".into()))?;
            Ok(NestedTaskRun {
                task_id,
                result: TaskResult::Success {
                    output: "nested-done".into(),
                    artifacts: vec![],
                },
            })
        }
    }

    fn bg_input(prompt: &str) -> ToolInput {
        ToolInput {
            name: "Agent".into(),
            input: json!({
                "prompt": prompt,
                "run_in_background": true,
            }),
            working_directory: Some(".".into()),
            sandbox_mode: false,
            dashboard_session_id: None,
            task_id: None,
        }
    }

    #[tokio::test]
    async fn background_started_then_taskoutput_eventually_completed() {
        let services = Arc::new(ToolServices::default());
        services.attach_sub_agent_executor(Arc::new(DelayedOkEx {
            delay_ms: 80,
            calls: AtomicU32::new(0),
        }));
        let agent = AgentTool::new(services.clone());
        let out = agent.execute(bg_input("hi")).await.expect("execute");
        assert!(out.error.is_none());
        assert_eq!(out.result["status"], "started");
        let id_str = out.result["nested_task_id"].as_str().unwrap();

        let to = TaskOutputTool::new(services.clone());
        let tout = to
            .execute(ToolInput {
                name: "TaskOutput".into(),
                input: json!({ "id": id_str }),
                working_directory: None,
                sandbox_mode: false,
                dashboard_session_id: None,
                task_id: None,
            })
            .await
            .unwrap();
        let st = tout.result["background_status"].as_str();
        assert!(
            st == Some("running") || st == Some("completed"),
            "unexpected background_status={st:?} body={}",
            tout.result
        );

        tokio::time::sleep(Duration::from_millis(250)).await;
        let tout2 = to
            .execute(ToolInput {
                name: "TaskOutput".into(),
                input: json!({ "id": id_str }),
                working_directory: None,
                sandbox_mode: false,
                dashboard_session_id: None,
                task_id: None,
            })
            .await
            .unwrap();
        assert_eq!(
            tout2.result["background_status"].as_str(),
            Some("completed")
        );
    }

    #[tokio::test]
    async fn background_taskstop_cancelled() {
        let services = Arc::new(ToolServices::default());
        services.attach_sub_agent_executor(Arc::new(DelayedOkEx {
            delay_ms: 10_000,
            calls: AtomicU32::new(0),
        }));
        let agent = AgentTool::new(services.clone());
        let out = agent.execute(bg_input("slow")).await.unwrap();
        let id_str = out.result["nested_task_id"].as_str().unwrap();

        let stop = TaskStopTool::new(services.clone());
        let stout = stop
            .execute(ToolInput {
                name: "TaskStop".into(),
                input: json!({ "id": id_str }),
                working_directory: None,
                sandbox_mode: false,
                dashboard_session_id: None,
                task_id: None,
            })
            .await
            .unwrap();
        assert_eq!(stout.result["kind"].as_str(), Some("background_agent"));

        tokio::time::sleep(Duration::from_millis(200)).await;
        let to = TaskOutputTool::new(services.clone());
        let tout = to
            .execute(ToolInput {
                name: "TaskOutput".into(),
                input: json!({ "id": id_str }),
                working_directory: None,
                sandbox_mode: false,
                dashboard_session_id: None,
                task_id: None,
            })
            .await
            .unwrap();
        assert_eq!(tout.result["background_status"].as_str(), Some("cancelled"));
    }

    #[tokio::test]
    async fn background_rejected_when_sub_agent_depth_exhausted() {
        let services = Arc::new(ToolServices::default());
        services.attach_sub_agent_executor(Arc::new(DelayedOkEx {
            delay_ms: 1,
            calls: AtomicU32::new(0),
        }));
        let parent = Uuid::new_v4();
        services.ensure_sub_agent_lineage(parent);
        let mut current = parent;
        for _ in 0..6 {
            let child = Uuid::new_v4();
            assert!(
                services.try_spawn_sub_agent(Some(current), child),
                "expected enter up to max depth (6)"
            );
            current = child;
        }
        assert!(
            !services.try_spawn_sub_agent(Some(current), Uuid::new_v4()),
            "seventh enter should fail"
        );
        let agent = AgentTool::new(services.clone());
        let mut input = bg_input("depth");
        input.task_id = Some(current);
        let out = agent.execute(input).await.expect("execute");
        assert_eq!(out.result["error"], "sub-agent nesting depth exceeded");
        assert!(
            out.result.get("nested_task_id").is_none(),
            "must not spawn or register background job"
        );
    }
}

#[cfg(test)]
mod structured_output_tests {
    use super::AgentTool;
    use crate::services::ToolServices;
    use anycode_core::prelude::*;
    use anycode_core::Artifact;
    use async_trait::async_trait;
    use serde_json::json;
    use std::sync::Arc;
    use std::sync::Mutex as StdMutex;
    use std::time::Duration;
    use uuid::Uuid;

    /// 模拟子代理：捕获 invoke（断言注入/task_id），并按 task_id 记录结构化输出。
    struct SchemaEchoEx {
        services: Arc<ToolServices>,
        seen: StdMutex<Option<NestedTaskInvoke>>,
        record_value: serde_json::Value,
    }

    #[async_trait]
    impl SubAgentExecutor for SchemaEchoEx {
        async fn run_nested_task(
            &self,
            invoke: NestedTaskInvoke,
        ) -> Result<NestedTaskRun, CoreError> {
            let task_id = invoke
                .task_id
                .ok_or_else(|| CoreError::LLMError("expected task_id".into()))?;
            self.services
                .record_structured_output(task_id, self.record_value.clone());
            *self.seen.lock().expect("seen") = Some(invoke);
            Ok(NestedTaskRun {
                task_id,
                result: TaskResult::Success {
                    output: "done".into(),
                    artifacts: vec![Artifact {
                        name: "report".into(),
                        path: Some("/tmp/report.md".into()),
                        kind: Some("document".into()),
                        title: Some("Review report".into()),
                        bytes: Some(123),
                        mime: Some("text/markdown".into()),
                        ..Artifact::default()
                    }],
                },
            })
        }
    }

    fn fg_input(extra: serde_json::Value) -> ToolInput {
        let mut base = json!({ "prompt": "review code" });
        base.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        ToolInput {
            name: "Agent".into(),
            input: base,
            working_directory: Some(".".into()),
            sandbox_mode: false,
            dashboard_session_id: None,
            task_id: None,
        }
    }

    #[tokio::test]
    async fn foreground_schema_injected_and_structured_output_returned() {
        let services = Arc::new(ToolServices::default());
        let ex = Arc::new(SchemaEchoEx {
            services: services.clone(),
            seen: StdMutex::new(None),
            record_value: json!({ "verdict": "ok", "issues": [] }),
        });
        services.attach_sub_agent_executor(ex.clone());
        let agent = AgentTool::new(services.clone());
        let out = agent
            .execute(fg_input(json!({
                "output_schema": {
                    "type": "object",
                    "required": ["verdict"],
                    "properties": { "verdict": { "type": "string" } }
                }
            })))
            .await
            .expect("execute");

        // 父结果携带 structured_output 与 artifacts 元数据
        assert_eq!(out.result["status"], "completed");
        assert_eq!(out.result["structured_output"]["verdict"], "ok");
        let arts = out.result["artifacts"].as_array().expect("artifacts meta");
        assert_eq!(arts.len(), 1);
        assert_eq!(arts[0]["path"], "/tmp/report.md");
        assert_eq!(arts[0]["kind"], "document");
        assert!(
            arts[0].get("content").is_none(),
            "metadata only, no content"
        );

        // 子代理收到 schema 注入与预分配 task_id
        let seen = ex.seen.lock().expect("seen").clone().expect("invoke seen");
        assert!(seen.task_id.is_some());
        let inj = seen.context_injections.join("\n");
        assert!(
            inj.contains("Structured output contract"),
            "injection missing: {inj}"
        );
        assert!(inj.contains("verdict"), "schema text missing: {inj}");

        // schema 槽与捕获槽已回收（无泄漏）
        let tid = seen.task_id.unwrap();
        assert!(services.structured_output_schema(tid).is_none());
        assert!(services.take_structured_output(tid).is_none());
    }

    #[tokio::test]
    async fn nested_invoke_inherits_parent_task_budget() {
        let services = Arc::new(ToolServices::default());
        let parent = Uuid::new_v4();
        services.set_task_budget(
            parent,
            TaskBudget {
                token_budget_total: Some(42),
                ..TaskBudget::default()
            },
        );
        let ex = Arc::new(SchemaEchoEx {
            services: services.clone(),
            seen: StdMutex::new(None),
            record_value: json!({}),
        });
        services.attach_sub_agent_executor(ex.clone());
        let agent = AgentTool::new(services);
        let mut input = fg_input(json!({}));
        input.task_id = Some(parent);
        agent.execute(input).await.expect("execute");
        let seen = ex.seen.lock().expect("seen").clone().expect("invoke");
        assert_eq!(
            seen.budget.and_then(|b| b.token_budget_total),
            Some(42),
            "child must inherit the parent TaskBudget, not process env"
        );
    }

    #[tokio::test]
    async fn unified_kernel_refuses_detached_background_agent() {
        let services = Arc::new(ToolServices::default());
        services.set_forbid_detached_nested(true);
        let ex = Arc::new(SchemaEchoEx {
            services: services.clone(),
            seen: StdMutex::new(None),
            record_value: json!({}),
        });
        services.attach_sub_agent_executor(ex.clone());
        let agent = AgentTool::new(services);
        let out = agent
            .execute(fg_input(json!({ "run_in_background": true })))
            .await
            .expect("execute");
        assert!(out.error.is_some(), "{out:?}");
        assert!(
            ex.seen.lock().expect("seen").is_none(),
            "detached spawn must not start a nested task"
        );
    }

    #[tokio::test]
    async fn foreground_without_schema_no_injection_no_slot_leak() {
        let services = Arc::new(ToolServices::default());
        let ex = Arc::new(SchemaEchoEx {
            services: services.clone(),
            seen: StdMutex::new(None),
            record_value: json!({ "free": true }),
        });
        services.attach_sub_agent_executor(ex.clone());
        let agent = AgentTool::new(services.clone());
        let out = agent.execute(fg_input(json!({}))).await.expect("execute");

        assert_eq!(out.result["status"], "completed");
        // 未声明 schema 时捕获仍会透传（子代理主动调用 StructuredOutput 的情况）
        assert_eq!(out.result["structured_output"]["free"], true);
        let seen = ex.seen.lock().expect("seen").clone().expect("invoke seen");
        assert!(seen.context_injections.is_empty());
    }

    #[tokio::test]
    async fn background_schema_slot_keyed_by_returned_task_id() {
        let services = Arc::new(ToolServices::default());
        let ex = Arc::new(SchemaEchoEx {
            services: services.clone(),
            seen: StdMutex::new(None),
            record_value: json!({ "v": 1 }),
        });
        services.attach_sub_agent_executor(ex.clone());
        let agent = AgentTool::new(services.clone());
        let out = agent
            .execute(fg_input(json!({
                "run_in_background": true,
                "output_schema": { "type": "object" }
            })))
            .await
            .expect("execute");
        assert_eq!(out.result["status"], "started");
        let id_str = out.result["nested_task_id"].as_str().unwrap();
        let tid = uuid::Uuid::parse_str(id_str).unwrap();
        // 关键断言：schema 槽键 = 返回给父的 nested_task_id（后台路径不再覆盖 id）
        assert!(
            services.structured_output_schema(tid).is_some(),
            "schema slot must be keyed by the background task id"
        );
        // 等待后台完成：schema 槽回收；结构化捕获保留给 TaskOutput 一次性取走
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(services.structured_output_schema(tid).is_none());
        assert_eq!(
            services.take_structured_output(tid),
            Some(json!({ "v": 1 }))
        );
        assert!(services.take_structured_output(tid).is_none());
    }

    /// 未知 subagent_type：执行器返回 AgentNotFound（type-miss 路径）。
    struct TypeMissEx;

    #[async_trait]
    impl SubAgentExecutor for TypeMissEx {
        async fn run_nested_task(
            &self,
            invoke: NestedTaskInvoke,
        ) -> Result<NestedTaskRun, CoreError> {
            Err(CoreError::AgentNotFound(invoke.task_id.expect("task_id")))
        }

        fn agent_catalog(&self) -> Vec<(String, String)> {
            vec![
                ("explore".into(), "search".into()),
                ("plan".into(), "plan".into()),
            ]
        }
    }

    #[tokio::test]
    async fn foreground_type_miss_returns_known_agents_for_self_heal() {
        let services = Arc::new(ToolServices::default());
        services.attach_sub_agent_executor(Arc::new(TypeMissEx));
        let agent = AgentTool::new(services.clone());
        let out = agent
            .execute(fg_input(json!({ "subagent_type": "no-such-agent" })))
            .await
            .expect("execute");
        assert_eq!(out.result["status"], "failed");
        assert_eq!(out.result["error"], "unknown subagent_type: no-such-agent");
        let known = out.result["known_agents"].as_array().expect("known_agents");
        assert!(known.iter().any(|v| v == "explore"));
        assert!(known.iter().any(|v| v == "plan"));
    }

    #[tokio::test]
    async fn foreground_invoke_carries_per_call_cancel_flag() {
        use std::sync::atomic::Ordering;
        let services = Arc::new(ToolServices::default());
        let ex = Arc::new(SchemaEchoEx {
            services: services.clone(),
            seen: StdMutex::new(None),
            record_value: json!({}),
        });
        services.attach_sub_agent_executor(ex.clone());
        let agent = AgentTool::new(services.clone());
        let out = agent.execute(fg_input(json!({}))).await.expect("execute");
        assert_eq!(out.result["status"], "completed");
        let seen = ex.seen.lock().expect("seen").clone().expect("invoke seen");
        let flag = seen.cancel.expect("per-call cancel flag");
        // 工具返回时 guard 已随作用域 drop → 标志置位（飞行中由子代理边界消费）。
        assert!(flag.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn foreground_cancel_guard_sets_flag_when_tool_future_dropped() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let flag = Arc::new(AtomicBool::new(false));
        let mut fut = Box::pin({
            let flag = Arc::clone(&flag);
            async move {
                let _guard = super::ForegroundCancelGuard(flag);
                std::future::pending::<()>().await;
            }
        });
        tokio::select! {
            biased;
            _ = &mut fut => unreachable!("pending future"),
            _ = tokio::time::sleep(Duration::from_millis(20)) => {}
        }
        drop(fut);
        assert!(flag.load(Ordering::SeqCst));
    }
}

#[cfg(test)]
mod propose_skills_tests {
    use super::*;
    use crate::skills::SkillCatalog;
    use std::fs;

    fn services_with_skill_catalog() -> Arc<ToolServices> {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("skills");
        let dir = root.join("existing-skill");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("SKILL.md"),
            "---\nname: existing-skill\ndescription: an existing skill\n---\n",
        )
        .unwrap();
        let catalog = SkillCatalog::scan(&[root], None, 1_000, true);
        Arc::new(ToolServices::new_ephemeral_with_skills(
            None,
            Arc::new(catalog),
        ))
    }

    async fn run(services: Arc<ToolServices>, proposals: serde_json::Value) -> ToolOutput {
        ProposeSkillsTool::new(services)
            .execute(ToolInput {
                name: "ProposeSkills".into(),
                input: serde_json::json!({ "proposals": proposals }),
                working_directory: None,
                sandbox_mode: false,
                dashboard_session_id: None,
                task_id: None,
            })
            .await
            .expect("execute")
    }

    #[tokio::test]
    async fn rejects_empty_and_oversized_proposals() {
        let services = services_with_skill_catalog();
        let out = run(services.clone(), serde_json::json!([])).await;
        assert!(out.error.is_some(), "empty proposals must error");

        let many = (0..4)
            .map(|i| {
                serde_json::json!({
                    "name": format!("skill-{i}"),
                    "kind": "new",
                    "description": "d"
                })
            })
            .collect::<Vec<_>>();
        let out = run(services, serde_json::json!(many)).await;
        assert!(out.error.is_some(), ">3 proposals must error");
    }

    #[tokio::test]
    async fn accepts_valid_new_proposal() {
        let services = services_with_skill_catalog();
        let out = run(
            services,
            serde_json::json!([{
                "name": "my-skill",
                "kind": "new",
                "description": "A brand new skill",
                "evidence": "seen in repo",
                "skillMd": "---\nname: my-skill\ndescription: A brand new skill\n---\n"
            }]),
        )
        .await;
        assert!(out.error.is_none());
        assert_eq!(out.result["count"], 1);
        assert_eq!(out.result["accepted"], 1);
        assert_eq!(out.result["reviews"][0]["accepted"], true);
    }

    #[tokio::test]
    async fn flags_invalid_kind_and_unknown_target() {
        let services = services_with_skill_catalog();
        let out = run(
            services,
            serde_json::json!([
                {
                    "name": "bad-kind",
                    "kind": "delete",
                    "description": "d"
                },
                {
                    "name": "improved",
                    "kind": "improvement",
                    "target": "no-such-skill",
                    "description": "d"
                }
            ]),
        )
        .await;
        assert!(out.error.is_none());
        assert_eq!(out.result["accepted"], 0);
        assert_eq!(out.result["count"], 2);
        let issues0 = out.result["reviews"][0]["issues"].as_array().unwrap();
        assert!(issues0
            .iter()
            .any(|i| i["message"].as_str().unwrap().contains("kind")));
        let issues1 = out.result["reviews"][1]["issues"].as_array().unwrap();
        assert!(issues1.iter().any(|i| i["message"]
            .as_str()
            .unwrap()
            .contains("target skill not found")));
    }

    #[tokio::test]
    async fn accepts_improvement_of_existing_skill() {
        let services = services_with_skill_catalog();
        let out = run(
            services,
            serde_json::json!([{
                "name": "existing-skill",
                "kind": "improvement",
                "target": "existing-skill",
                "description": "make it better"
            }]),
        )
        .await;
        assert!(out.error.is_none());
        assert_eq!(out.result["accepted"], 1);
        assert_eq!(out.result["reviews"][0]["targetExists"], true);
    }
}

#[cfg(test)]
mod skill_tool_tests {
    use super::*;
    use crate::skills::SkillCatalog;
    use std::fs;

    fn write_executable_run(path: &Path, body: &str) {
        fs::write(path, body).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(path, perms).unwrap();
        }
    }

    fn catalog_with_network_denied_run() -> (tempfile::TempDir, Arc<ToolServices>) {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("skills");
        let dir = root.join("net-denied");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("SKILL.md"),
            "---\nname: net-denied\ndescription: no net\npermissions:\n  network: false\n---\n# Body\n",
        )
        .unwrap();
        write_executable_run(&dir.join("run"), "#!/bin/sh\necho should-not-run\n");
        let catalog = SkillCatalog::scan(&[root], None, 1_000, true);
        let services = Arc::new(ToolServices::new_ephemeral_with_skills(
            None,
            Arc::new(catalog),
        ));
        (temp, services)
    }

    #[tokio::test]
    async fn skill_run_blocked_when_network_false() {
        let (_tmp, services) = catalog_with_network_denied_run();
        let out = SkillTool::new(services)
            .execute(ToolInput {
                name: "Skill".into(),
                input: serde_json::json!({ "name": "net-denied", "args": ["x"] }),
                working_directory: None,
                sandbox_mode: false,
                dashboard_session_id: None,
                task_id: None,
            })
            .await
            .expect("execute");
        assert!(out.error.as_deref().unwrap_or("").contains("network=false"));
        assert_eq!(
            out.result["error"],
            "skill run blocked: permissions.network=false"
        );
    }

    #[tokio::test]
    async fn skill_instructions_mode_without_run_args() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("skills");
        let dir = root.join("docs-only");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("SKILL.md"),
            "---\nname: docs-only\ndescription: d\n---\n# Hello skill\n",
        )
        .unwrap();
        let catalog = SkillCatalog::scan(&[root], None, 1_000, true);
        let services = Arc::new(ToolServices::new_ephemeral_with_skills(
            None,
            Arc::new(catalog),
        ));
        let out = SkillTool::new(services)
            .execute(ToolInput {
                name: "Skill".into(),
                input: serde_json::json!({ "name": "docs-only" }),
                working_directory: None,
                sandbox_mode: false,
                dashboard_session_id: None,
                task_id: None,
            })
            .await
            .expect("execute");
        assert!(out.error.is_none());
        assert_eq!(out.result["mode"], "instructions");
        assert!(out.result["instructions"]
            .as_str()
            .unwrap()
            .contains("Hello skill"));
    }
}
