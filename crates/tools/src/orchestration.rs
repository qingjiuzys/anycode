//! Task / Team / Cron / RemoteTrigger 编排工具。
//!
//! 在常规 CLI 下，变更会持久化到 `~/.anycode/tasks/orchestration.json`（`ToolServices::load_or_new*` 绑定路径时）；
//! 无用户主目录的 ephemeral 会话中为进程内状态。

use crate::services::{TaskRecord, ToolServices};
use anycode_core::prelude::*;
use async_trait::async_trait;
use chrono::{Datelike, Local, Timelike};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

fn sens() -> SecurityPolicy {
    SecurityPolicy::sensitive_mutation()
}

// --- TaskCreate ---
pub struct TaskCreateTool {
    services: Arc<ToolServices>,
    policy: SecurityPolicy,
}

impl TaskCreateTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            services,
            policy: sens(),
        }
    }
}

#[derive(Deserialize)]
struct TcIn {
    subject: String,
    description: String,
    #[serde(default)]
    metadata: serde_json::Value,
    #[serde(default, alias = "active_form")]
    active_form: Option<String>,
}

#[async_trait]
impl Tool for TaskCreateTool {
    fn name(&self) -> &str {
        "TaskCreate"
    }
    fn description(&self) -> &str {
        "Create an orchestration task record (persists with ~/.anycode/tasks/orchestration.json when a home directory is available)."
    }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "subject": { "type": "string" },
                "description": { "type": "string" },
                "activeForm": { "type": "string", "description": "Present continuous form shown in spinner when in_progress (e.g. \"Running tests\")" },
                "metadata": {}
            },
            "required": ["subject", "description"]
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
        let v: TcIn = serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;
        let t = self
            .services
            .insert_task_full(v.subject, v.description, v.metadata, v.active_form);
        Ok(ToolOutput {
            result: json!({ "task": { "id": t.id, "subject": t.subject } }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

// --- TaskUpdate ---
#[derive(Deserialize)]
struct TuIn {
    #[serde(default)]
    id: Option<String>,
    #[serde(default, alias = "task_id")]
    task_id: Option<String>,
    #[serde(default)]
    subject: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    metadata: serde_json::Value,
    #[serde(default, alias = "active_form")]
    active_form: Option<String>,
    #[serde(default)]
    owner: Option<String>,
    #[serde(default)]
    add_blocks: Option<Vec<String>>,
    #[serde(default)]
    add_blocked_by: Option<Vec<String>>,
}

impl TuIn {
    fn id(&self) -> Option<String> {
        self.task_id.clone().or_else(|| self.id.clone())
    }
}

pub struct TaskUpdateTool {
    services: Arc<ToolServices>,
    policy: SecurityPolicy,
}

impl TaskUpdateTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            services,
            policy: sens(),
        }
    }
}

#[async_trait]
impl Tool for TaskUpdateTool {
    fn name(&self) -> &str {
        "TaskUpdate"
    }
    fn description(&self) -> &str {
        "Update an orchestration task by id (same persistence rules as TaskCreate)."
    }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "taskId": { "type": "string", "description": "The ID of the task to update (alias: id)" },
                "subject": { "type": "string" },
                "description": { "type": "string" },
                "status": { "type": "string", "enum": ["pending", "in_progress", "completed", "deleted"] },
                "activeForm": { "type": "string", "description": "Present continuous form shown in spinner when in_progress (e.g. \"Running tests\")" },
                "owner": { "type": "string", "description": "New owner for the task" },
                "addBlocks": { "type": "array", "items": { "type": "string" }, "description": "Task IDs that this task blocks" },
                "addBlockedBy": { "type": "array", "items": { "type": "string" }, "description": "Task IDs that block this task" },
                "metadata": {}
            },
            "required": ["taskId"]
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
        let u: TuIn = serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;
        let id = u
            .id()
            .ok_or_else(|| CoreError::Other(anyhow::anyhow!("TaskUpdate requires taskId")))?;
        let patch = TaskRecord {
            id: id.clone(),
            subject: u.subject,
            description: u.description,
            status: u.status,
            metadata: u.metadata,
            active_form: u.active_form,
            owner: u.owner,
            blocks: u.add_blocks.unwrap_or_default(),
            blocked_by: u.add_blocked_by.unwrap_or_default(),
        };
        let out = self.services.update_task(&id, patch);
        Ok(ToolOutput {
            result: json!({ "task": out }),
            error: if out.is_none() {
                Some("not found".into())
            } else {
                None
            },
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

// --- TaskList ---
pub struct TaskListTool {
    services: Arc<ToolServices>,
}

impl TaskListTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self { services }
    }
}

#[async_trait]
impl Tool for TaskListTool {
    fn name(&self) -> &str {
        "TaskList"
    }
    fn description(&self) -> &str {
        "List orchestration task records (same persistence rules as TaskCreate)."
    }
    fn schema(&self) -> serde_json::Value {
        json!({"type":"object","properties":{}})
    }
    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Auto
    }
    fn security_policy(&self) -> Option<&SecurityPolicy> {
        None
    }
    async fn execute(&self, _input: ToolInput) -> Result<ToolOutput, CoreError> {
        let start = Instant::now();
        let list = self.services.list_tasks();
        Ok(ToolOutput {
            result: json!({ "tasks": list }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

// --- TaskGet ---
#[derive(Deserialize)]
struct TgIn {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    task_id: Option<String>,
}

impl TgIn {
    /// 统一取 ID：优先 `taskId`/`task_id`（Claude Code 参数名），其次兼容 `id`。
    fn resolve_id(&self) -> Option<String> {
        self.task_id.clone().or_else(|| self.id.clone())
    }
}

pub struct TaskGetTool {
    services: Arc<ToolServices>,
}

impl TaskGetTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self { services }
    }
}

#[async_trait]
impl Tool for TaskGetTool {
    fn name(&self) -> &str {
        "TaskGet"
    }
    fn description(&self) -> &str {
        "Get one orchestration task by taskId (alias: id)."
    }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "taskId": { "type": "string", "description": "The ID of the task to retrieve (alias: id)" }
            },
            "required": ["taskId"]
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
        let g: TgIn = serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;
        let id = g
            .resolve_id()
            .ok_or_else(|| CoreError::Other(anyhow::anyhow!("TaskGet requires taskId")))?;
        let t = self.services.get_task(&id);
        Ok(ToolOutput {
            result: json!({ "task": t }),
            error: if t.is_none() {
                Some("not found".into())
            } else {
                None
            },
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

// --- TaskStop ---
pub struct TaskStopTool {
    services: Arc<ToolServices>,
    policy: SecurityPolicy,
}

impl TaskStopTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            services,
            policy: sens(),
        }
    }
}

#[async_trait]
impl Tool for TaskStopTool {
    fn name(&self) -> &str {
        "TaskStop"
    }
    fn description(&self) -> &str {
        "Remove a task record by task_id (same persistence rules as TaskCreate). If the id is a UUID for a nested `Agent` or Bash `run_in_background` job, best-effort aborts that background run (see `background_agent` in JSON). Accepts `task_id` (Claude Code), `id`, or `shell_id` (deprecated)."
    }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "task_id": { "type": "string", "description": "The ID of the background task to stop (alias: id, shell_id)" },
                "shell_id": { "type": "string", "description": "Deprecated: use task_id instead" }
            },
            "required": ["task_id"]
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
        let shell_id = input
            .input
            .get("shell_id")
            .and_then(|v| v.as_str())
            .map(String::from);
        let g: TgIn = serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;
        let gid = g
            .resolve_id()
            .or(shell_id)
            .ok_or_else(|| CoreError::Other(anyhow::anyhow!("TaskStop requires task_id")))?;
        if let Ok(uid) = Uuid::parse_str(gid.trim()) {
            if self.services.cancel_background_agent(uid) {
                return Ok(ToolOutput {
                    result: json!({
                        "stopped": true,
                        "kind": "background_agent",
                        "task_id": gid,
                        "id": gid,
                        "note": "best-effort abort: sets nested cooperative-cancel flag and aborts the background task (same flag as NestedTaskInvoke.cancel / TaskContext.nested_cancel)"
                    }),
                    error: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        }
        let ok = self.services.remove_task(&gid);
        Ok(ToolOutput {
            result: json!({ "stopped": ok, "task_id": gid }),
            error: if ok { None } else { Some("not found".into()) },
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

// --- TaskOutput ---
pub struct TaskOutputTool {
    services: Arc<ToolServices>,
}

impl TaskOutputTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self { services }
    }
}

#[async_trait]
impl Tool for TaskOutputTool {
    fn name(&self) -> &str {
        "TaskOutput"
    }
    fn description(&self) -> &str {
        "Returns the orchestration task record when `task_id` matches TaskCreate. If it is a runtime execution UUID (e.g. `nested_task_id` from Agent, or `background_task_id` from Bash `run_in_background`), also returns `output_log_path` and a tail of `output.log` under ~/.anycode/tasks/<id>/ when the file exists. For background jobs, includes `background_status` / `background_summary` from the in-process registry while the process lives. Accepts `task_id` (Claude Code) or `id`."
    }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "task_id": { "type": "string", "description": "The task ID to get output from (alias: id)" }
            },
            "required": ["task_id"]
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
        let g: TgIn = serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;
        let gid = g
            .resolve_id()
            .ok_or_else(|| CoreError::Other(anyhow::anyhow!("TaskOutput requires task_id")))?;
        let t = self.services.get_task(&gid);

        const TAIL_MAX: usize = 24 * 1024;
        let mut output_log_path: Option<String> = None;
        let mut output_tail: Option<String> = None;
        let mut background_status: Option<String> = None;
        let mut background_summary: Option<String> = None;
        if let Ok(uid) = Uuid::parse_str(gid.trim()) {
            if let Some((st, sum)) = self.services.background_agent_tool_view(uid) {
                background_status = Some(st.as_json_str().to_string());
                background_summary = sum;
            }
            if let Some(home) = dirs::home_dir() {
                let disk = DiskTaskOutput::new(home.join(".anycode").join("tasks"));
                let path = disk.output_path(uid);
                output_log_path = Some(path.to_string_lossy().into_owned());
                if path.is_file() {
                    let tail = disk.tail(uid, TAIL_MAX).unwrap_or_default();
                    if !tail.is_empty() {
                        output_tail = Some(tail);
                    }
                }
            }
        }

        Ok(ToolOutput {
            result: json!({
                "task": t,
                "output_log_path": output_log_path,
                "output_file": output_log_path,
                "output_tail": output_tail,
                "background_status": background_status,
                "background_summary": background_summary,
            }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

// --- Team ---
#[derive(Deserialize)]
struct TeamIn {
    name: String,
}

pub struct TeamCreateTool {
    services: Arc<ToolServices>,
    policy: SecurityPolicy,
}

impl TeamCreateTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            services,
            policy: sens(),
        }
    }
}

#[async_trait]
impl Tool for TeamCreateTool {
    fn name(&self) -> &str {
        "TeamCreate"
    }
    fn description(&self) -> &str {
        "Create a team record (same persistence rules as TaskCreate)."
    }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": { "name": { "type": "string" } },
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
        let t: TeamIn =
            serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;
        let r = self.services.insert_team(t.name);
        Ok(ToolOutput {
            result: json!({ "team": r }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

pub struct TeamDeleteTool {
    services: Arc<ToolServices>,
    policy: SecurityPolicy,
}

impl TeamDeleteTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            services,
            policy: sens(),
        }
    }
}

#[async_trait]
impl Tool for TeamDeleteTool {
    fn name(&self) -> &str {
        "TeamDelete"
    }
    fn description(&self) -> &str {
        "Delete a team by id."
    }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": { "id": { "type": "string" } },
            "required": ["id"]
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
        let g: TgIn = serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;
        let ok = self.services.remove_team(
            g.resolve_id()
                .as_deref()
                .ok_or_else(|| CoreError::Other(anyhow::anyhow!("TeamDelete requires id")))?,
        );
        Ok(ToolOutput {
            result: json!({ "deleted": ok }),
            error: if ok { None } else { Some("not found".into()) },
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

// --- Cron ---
#[derive(Deserialize)]
struct CronIn {
    #[serde(default)]
    schedule: Option<String>,
    #[serde(default)]
    command: Option<String>,
    /// Claude Code 兼容别名：`cron` → `schedule`，`prompt` → `command`。
    #[serde(default)]
    cron: Option<String>,
    #[serde(default)]
    prompt: Option<String>,
    /// `local`（默认）：`schedule` 按本机墙钟理解并转为 UTC 存储；`utc`：字段已是 UTC。
    #[serde(default = "default_cron_tz")]
    schedule_timezone: String,
    /// Stable session correlation id for all future runs (auto-generated when omitted).
    #[serde(default)]
    session_id: Option<String>,
    /// Failure routing: `log` (default), `same_channel`, `shell`, `http`.
    #[serde(default)]
    failure_destination: Option<String>,
    /// Tool surface profile: `default`, `read_only`, `observability`, or `allowlist`.
    #[serde(default)]
    tool_profile: Option<String>,
    /// Required when `tool_profile` is `allowlist`.
    #[serde(default)]
    tool_allowlist: Option<Vec<String>>,
    /// Optional workflow definition file: the scheduler runs the DAG (ADR 014 §6)
    /// instead of a single-prompt task; `command` becomes the workflow user prompt.
    #[serde(default)]
    workflow: Option<String>,
    /// Claude Code 语义：true (default) = fire on every cron match; false = fire once then auto-delete.
    #[serde(default)]
    recurring: Option<bool>,
    /// Claude Code 语义：true = persist across restarts. anyCode 的编排 cron 始终持久化，故默认 true。
    #[serde(default)]
    durable: Option<bool>,
}

fn default_cron_tz() -> String {
    "local".to_string()
}

impl CronIn {
    /// 兼容 `cron`/`prompt` 别名（Claude Code）与 `schedule`/`command`（anyCode）。
    fn schedule_expr(&self) -> Option<&str> {
        self.schedule.as_deref().or(self.cron.as_deref())
    }

    fn command_str(&self) -> Option<&str> {
        self.command.as_deref().or(self.prompt.as_deref())
    }
}

#[derive(Deserialize)]
struct CronId {
    id: String,
}

pub struct CronCreateTool {
    services: Arc<ToolServices>,
    policy: SecurityPolicy,
}

impl CronCreateTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            services,
            policy: sens(),
        }
    }
}

#[async_trait]
impl Tool for CronCreateTool {
    fn name(&self) -> &str {
        "CronCreate"
    }
    fn description(&self) -> &str {
        "Register a cron job (persisted in ~/.anycode/tasks/orchestration.json). \
         `schedule`: 6 fields sec min hour day month weekday (5-field unix gets leading 0 sec). \
         Default `schedule_timezone` is `local` (wall clock on this machine, stored as UTC for the built-in scheduler). \
         Use `utc` only if you already converted to UTC, or an IANA name (e.g. `Asia/Shanghai`) for wall clock in that zone. \
         `command` runs as one agent task when the scheduler holds ~/.anycode/tasks/scheduler.lock \
         (`anycode-daemon scheduler`). Results are recorded in the Workbench session and cron-runs.jsonl. \
         Optional `session_id`, `failure_destination` (`log`|`shell`|`http`), \
         `tool_profile` (`default`|`read_only`|`observability`|`allowlist`), and `tool_allowlist` when using allowlist."
    }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "cron": { "type": "string", "description": "Standard 5-field cron expression in local time (Claude Code alias for `schedule`): \"M H DoM Mon DoW\" (e.g. \"*/5 * * * *\" = every 5 minutes). 6-field sec-prefixed forms also accepted." },
                "prompt": { "type": "string", "description": "The prompt to enqueue at each fire time (Claude Code alias for `command`)." },
                "schedule": { "type": "string", "description": "6 fields sec min hour day month weekday (5-field unix gets leading 0 sec). Alias: `cron`." },
                "command": { "type": "string", "description": "Runs as one agent task when the scheduler holds the lock. Alias: `prompt`." },
                "recurring": { "type": "boolean", "description": "true (default) = fire on every cron match until deleted; false = fire once at the next match, then auto-delete." },
                "durable": { "type": "boolean", "description": "true (default) = persist across restarts (anyCode cron always persists; flag kept for Claude Code parity)." },
                "schedule_timezone": {
                    "type": "string",
                    "description": "local (default): machine wall clock; utc: schedule already UTC; or IANA e.g. Asia/Shanghai"
                },
                "session_id": {
                    "type": "string",
                    "description": "Stable session correlation id for all future runs (auto-generated when omitted)"
                },
                "failure_destination": {
                    "type": "string",
                    "description": "log (default), same_channel, shell, or http"
                },
                "tool_profile": {
                    "type": "string",
                    "description": "default, read_only, observability, or allowlist"
                },
                "tool_allowlist": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Required when tool_profile is allowlist"
                }
            },
            "required": ["schedule", "command"]
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
        let c: CronIn =
            serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;
        let schedule_expr = c.schedule_expr().unwrap_or_default();
        let command_str = c.command_str().unwrap_or_default();
        if schedule_expr.is_empty() || command_str.is_empty() {
            return Ok(ToolOutput {
                result: json!({
                    "error": "CronCreate requires schedule (or cron) and command (or prompt)"
                }),
                error: Some("missing schedule/command".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
        if let Err(e) = crate::cron_schedule::validate_cron_schedule_expr(schedule_expr) {
            return Ok(ToolOutput {
                result: json!({ "error": format!("invalid cron schedule: {e}") }),
                error: Some(format!("invalid cron schedule: {e}")),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
        let tz_raw = c.schedule_timezone.trim();
        let resolved = match crate::cron_schedule::resolve_schedule_timezone(tz_raw) {
            Ok(t) => t,
            Err(e) => {
                return Ok(ToolOutput {
                    result: json!({ "error": e }),
                    error: Some("unsupported schedule_timezone".into()),
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        };
        let prepared =
            crate::cron_schedule::prepare_cron_schedule_for_storage(schedule_expr, resolved);
        let stored_schedule = prepared.schedule;
        let tz_note = prepared.note;
        if let Some(ref profile) = c.tool_profile {
            if !crate::catalog::is_known_cron_tool_profile(profile) {
                return Ok(ToolOutput {
                    result: json!({
                        "error": format!(
                            "unsupported tool_profile: {profile}; use one of {}",
                            crate::catalog::known_cron_tool_profiles().join(", ")
                        )
                    }),
                    error: Some("unsupported tool_profile".into()),
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        }
        if let Some(ref dest) = c.failure_destination {
            if !crate::catalog::is_known_cron_failure_destination(dest) {
                return Ok(ToolOutput {
                    result: json!({
                        "error": format!(
                            "unsupported failure_destination: {dest}; use one of {}",
                            crate::catalog::known_cron_failure_destinations().join(", ")
                        )
                    }),
                    error: Some("unsupported failure_destination".into()),
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        }
        if c.tool_profile.as_deref() == Some("allowlist") {
            let has_tools = c
                .tool_allowlist
                .as_ref()
                .is_some_and(|list| list.iter().any(|s| !s.trim().is_empty()));
            if !has_tools {
                return Ok(ToolOutput {
                    result: json!({
                        "error": "tool_profile allowlist requires non-empty tool_allowlist"
                    }),
                    error: Some("missing tool_allowlist".into()),
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        }
        let job = self.services.push_cron_with_options(
            stored_schedule.clone(),
            command_str.to_string(),
            crate::services::CronJobCreateOptions {
                name: None,
                enabled: None,
                schedule_timezone: Some(c.schedule_timezone),
                session_id: c.session_id,
                failure_destination: c.failure_destination,
                tool_profile: c.tool_profile,
                tool_allowlist: c.tool_allowlist,
                project_id: None,
                workflow: c.workflow,
                recurring: c.recurring,
            },
        );
        let next_utc = crate::cron_schedule::next_fire_utc_from_stored_schedule(&stored_schedule);
        let (next_utc_s, next_local_s) = next_utc
            .map(crate::cron_schedule::format_next_fire_human)
            .unwrap_or_else(|| ("unknown".into(), "unknown".into()));
        Ok(ToolOutput {
            result: json!({
                "job_id": job.id,
                "session_id": job.session_id,
                "failure_destination": job.failure_destination,
                "tool_profile": job.tool_profile,
                "tool_allowlist": job.tool_allowlist,
                "schedule_stored_utc": stored_schedule,
                "schedule_timezone_applied": tz_note,
                "recurring": c.recurring.unwrap_or(true),
                "durable": c.durable.unwrap_or(true),
                "next_fire_utc": next_utc_s,
                "next_fire_local": next_local_s,
                "hint": "Requires the scheduler (`anycode-daemon scheduler`). Cron output is recorded in the Workbench session and ~/.anycode/logs/cron-runs.jsonl."
            }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

pub struct CronDeleteTool {
    services: Arc<ToolServices>,
    policy: SecurityPolicy,
}

impl CronDeleteTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            services,
            policy: sens(),
        }
    }
}

#[async_trait]
impl Tool for CronDeleteTool {
    fn name(&self) -> &str {
        "CronDelete"
    }
    fn description(&self) -> &str {
        "Delete a cron job by id."
    }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": { "id": { "type": "string" } },
            "required": ["id"]
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
        let c: CronId =
            serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;
        let ok = self.services.remove_cron(&c.id);
        Ok(ToolOutput {
            result: json!({ "deleted": ok }),
            error: if ok { None } else { Some("not found".into()) },
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[derive(Deserialize)]
struct CronUpdateIn {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    schedule: Option<String>,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    schedule_timezone: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    failure_destination: Option<String>,
    #[serde(default)]
    tool_profile: Option<String>,
    #[serde(default)]
    project_id: Option<String>,
    #[serde(default)]
    workflow: Option<String>,
    #[serde(default)]
    recurring: Option<bool>,
}

pub struct CronUpdateTool {
    services: Arc<ToolServices>,
    policy: SecurityPolicy,
}

impl CronUpdateTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            services,
            policy: sens(),
        }
    }
}

#[async_trait]
impl Tool for CronUpdateTool {
    fn name(&self) -> &str {
        "CronUpdate"
    }
    fn description(&self) -> &str {
        "Update fields on an existing cron job by id (persisted in ~/.anycode/tasks/orchestration.json). \
         Provide `id` and any of: name, enabled, schedule, command, schedule_timezone, session_id, \
         failure_destination, tool_profile, project_id. When updating `schedule`, the same timezone \
         rules as CronCreate apply (`local` default)."
    }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "id": { "type": "string" },
                "name": { "type": "string" },
                "enabled": { "type": "boolean" },
                "schedule": { "type": "string" },
                "command": { "type": "string" },
                "schedule_timezone": { "type": "string" },
                "session_id": { "type": "string" },
                "failure_destination": { "type": "string" },
                "tool_profile": { "type": "string" },
                "project_id": { "type": "string" },
                "recurring": { "type": "boolean", "description": "false = one-shot (auto-delete after next fire); true = recurring" }
            },
            "required": ["id"]
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
        let c: CronUpdateIn =
            serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;
        if c.id.trim().is_empty() {
            return Ok(ToolOutput {
                result: json!({ "error": "id required" }),
                error: Some("id required".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        let mut patch = crate::services::CronJobPatch {
            name: c.name,
            enabled: c.enabled,
            schedule: None,
            command: c.command,
            schedule_timezone: c.schedule_timezone.clone(),
            session_id: c.session_id,
            failure_destination: c.failure_destination.clone(),
            tool_profile: c.tool_profile.clone(),
            project_id: c.project_id,
            workflow: c.workflow.clone(),
            recurring: c.recurring,
        };

        let mut schedule_note = None;
        if let Some(ref schedule) = c.schedule {
            if let Err(e) = crate::cron_schedule::validate_cron_schedule_expr(schedule) {
                return Ok(ToolOutput {
                    result: json!({ "error": format!("invalid cron schedule: {e}") }),
                    error: Some(format!("invalid cron schedule: {e}")),
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
            let tz_raw = c.schedule_timezone.as_deref().unwrap_or("local").trim();
            let resolved = match crate::cron_schedule::resolve_schedule_timezone(tz_raw) {
                Ok(t) => t,
                Err(e) => {
                    return Ok(ToolOutput {
                        result: json!({ "error": e }),
                        error: Some("unsupported schedule_timezone".into()),
                        duration_ms: start.elapsed().as_millis() as u64,
                    });
                }
            };
            let prepared =
                crate::cron_schedule::prepare_cron_schedule_for_storage(schedule, resolved);
            patch.schedule = Some(prepared.schedule);
            schedule_note = Some(prepared.note);
        }

        if let Some(ref profile) = c.tool_profile {
            if !crate::catalog::is_known_cron_tool_profile(profile) {
                return Ok(ToolOutput {
                    result: json!({
                        "error": format!(
                            "unsupported tool_profile: {profile}; use one of {}",
                            crate::catalog::known_cron_tool_profiles().join(", ")
                        )
                    }),
                    error: Some("unsupported tool_profile".into()),
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        }
        if let Some(ref dest) = c.failure_destination {
            if !crate::catalog::is_known_cron_failure_destination(dest) {
                return Ok(ToolOutput {
                    result: json!({
                        "error": format!(
                            "unsupported failure_destination: {dest}; use one of {}",
                            crate::catalog::known_cron_failure_destinations().join(", ")
                        )
                    }),
                    error: Some("unsupported failure_destination".into()),
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        }

        let Some(job) = self.services.update_cron(&c.id, patch) else {
            return Ok(ToolOutput {
                result: json!({ "error": "not found", "id": c.id }),
                error: Some("not found".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        };

        let next_utc = crate::cron_schedule::next_fire_utc_from_stored_schedule(&job.schedule);
        let (next_utc_s, next_local_s) = next_utc
            .map(crate::cron_schedule::format_next_fire_human)
            .unwrap_or_else(|| ("unknown".into(), "unknown".into()));

        Ok(ToolOutput {
            result: json!({
                "job": job,
                "schedule_timezone_applied": schedule_note,
                "next_fire_utc": next_utc_s,
                "next_fire_local": next_local_s,
            }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

pub struct CronListTool {
    services: Arc<ToolServices>,
}

impl CronListTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self { services }
    }
}

#[async_trait]
impl Tool for CronListTool {
    fn name(&self) -> &str {
        "CronList"
    }
    fn description(&self) -> &str {
        "List cron jobs."
    }
    fn schema(&self) -> serde_json::Value {
        json!({"type":"object","properties":{}})
    }
    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Auto
    }
    fn security_policy(&self) -> Option<&SecurityPolicy> {
        None
    }
    async fn execute(&self, _input: ToolInput) -> Result<ToolOutput, CoreError> {
        let start = Instant::now();
        Ok(ToolOutput {
            result: json!({ "jobs": self.services.list_crons() }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

// --- RemoteTrigger ---
#[derive(Deserialize)]
struct RtIn {
    #[serde(default)]
    url: String,
}

pub struct RemoteTriggerTool {
    services: Arc<ToolServices>,
    policy: SecurityPolicy,
}

impl RemoteTriggerTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            services,
            policy: sens(),
        }
    }
}

#[async_trait]
impl Tool for RemoteTriggerTool {
    fn name(&self) -> &str {
        "RemoteTrigger"
    }
    fn description(&self) -> &str {
        "Register a remote trigger URL (persisted like other orchestration data; no outbound call in v1)."
    }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": { "url": { "type": "string" } },
            "required": ["url"]
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
        let r: RtIn = serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;
        self.services.push_remote_hook(r.url.clone());
        Ok(ToolOutput {
            result: json!({ "registered": true, "url": r.url }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

// --- ScheduleWakeup ---
#[derive(Deserialize)]
struct SwIn {
    #[serde(default)]
    delay_seconds: Option<u64>,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    prompt: Option<String>,
    #[serde(default)]
    stop: bool,
}

const WAKEUP_JOB_NAME_PREFIX: &str = "schedule-wakeup:";

pub struct ScheduleWakeupTool {
    services: Arc<ToolServices>,
    policy: SecurityPolicy,
}

impl ScheduleWakeupTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            services,
            policy: sens(),
        }
    }
}

#[async_trait]
impl Tool for ScheduleWakeupTool {
    fn name(&self) -> &str {
        "ScheduleWakeup"
    }
    fn description(&self) -> &str {
        "Schedule the agent to wake up after a delay and run a prompt. \\\n         On fire, the prompt runs as one agent task (like CronCreate with a one-shot schedule). \\\n         Pass `stop: true` to cancel all pending wakeups."
    }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "delaySeconds": { "type": "integer", "description": "Seconds from now to wake up. Clamped to [60, 3600] by the runtime. Required unless `stop` is true." },
                "reason": { "type": "string", "description": "One short sentence explaining the chosen delay. Shown to the user." },
                "prompt": { "type": "string", "description": "The prompt to run when the wakeup fires. Required unless `stop` is true." },
                "stop": { "type": "boolean", "description": "Set to true to cancel all pending wakeups immediately. When true, other fields are ignored." }
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
        let w: SwIn = serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;

        if w.stop {
            let mut cancelled = 0u32;
            for job in self.services.list_crons() {
                if job
                    .name
                    .as_deref()
                    .is_some_and(|n| n.starts_with(WAKEUP_JOB_NAME_PREFIX))
                {
                    if self.services.remove_cron(&job.id) {
                        cancelled += 1;
                    }
                }
            }
            return Ok(ToolOutput {
                result: json!({
                    "stopped": true,
                    "cancelledWakeups": cancelled,
                    "message": "pending wakeups cancelled"
                }),
                error: None,
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        let Some(delay) = w.delay_seconds else {
            return Ok(ToolOutput {
                result: json!({ "error": "ScheduleWakeup requires delaySeconds unless stop is true" }),
                error: Some("missing delaySeconds".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        };
        let prompt = w.prompt.unwrap_or_default().trim().to_string();
        if prompt.is_empty() {
            return Ok(ToolOutput {
                result: json!({ "error": "ScheduleWakeup requires prompt unless stop is true" }),
                error: Some("missing prompt".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        let was_clamped = delay < 60 || delay > 3600;
        let clamped = delay.clamp(60, 3600);

        // 一次性 cron：当前本地时刻 + delay 秒（具体数字字段，星期用 * 避免 OR 语义）。
        let target = Local::now() + chrono::Duration::seconds(clamped as i64);
        let expr = format!(
            "{} {} {} {} {} *",
            target.second(),
            target.minute(),
            target.hour(),
            target.day(),
            target.month()
        );
        let prepared = crate::cron_schedule::prepare_cron_schedule_for_storage(
            &expr,
            crate::cron_schedule::ScheduleTimezone::Local,
        );

        let id = Uuid::new_v4().to_string();
        let job = self.services.push_cron_with_options(
            prepared.schedule,
            prompt,
            crate::services::CronJobCreateOptions {
                name: Some(format!("{WAKEUP_JOB_NAME_PREFIX}{id}")),
                enabled: None,
                schedule_timezone: Some("local".to_string()),
                session_id: None,
                failure_destination: Some("log".to_string()),
                tool_profile: Some("default".to_string()),
                tool_allowlist: None,
                project_id: None,
                workflow: None,
                // 一次性唤醒：fire 后由调度器自动删除。
                recurring: Some(false),
            },
        );

        let scheduled_for = target.timestamp_millis();
        Ok(ToolOutput {
            result: json!({
                "scheduledFor": scheduled_for,
                "clampedDelaySeconds": clamped,
                "wasClamped": was_clamped,
                "jobId": job.id,
                "reason": w.reason.unwrap_or_default(),
                "hint": "Wakeups are persisted cron jobs; cancel with ScheduleWakeup stop:true or CronDelete."
            }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

// --- Monitor ---
#[derive(Deserialize)]
struct MonIn {
    description: String,
    #[serde(default)]
    timeout_ms: Option<u64>,
    #[serde(default)]
    persistent: bool,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    ws: Option<serde_json::Value>,
}

pub struct MonitorTool {
    services: Arc<ToolServices>,
    policy: SecurityPolicy,
}

impl MonitorTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            services,
            policy: sens(),
        }
    }
}

#[async_trait]
impl Tool for MonitorTool {
    fn name(&self) -> &str {
        "Monitor"
    }
    fn description(&self) -> &str {
        "Run a background monitor: a shell command whose stdout lines are events (or a WebSocket). \\\n         Returns a task id; poll with TaskOutput and stop with TaskStop."
    }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "description": { "type": "string", "description": "Short human-readable description of what you are monitoring (shown in notifications)." },
                "timeout_ms": { "type": "integer", "description": "Kill the monitor after this deadline. Default 300000ms, max 3600000ms. Ignored when persistent is true." },
                "persistent": { "type": "boolean", "description": "Run for the lifetime of the session (no timeout). Stop with TaskStop." },
                "command": { "type": "string", "description": "Shell command or script. Each stdout line is an event; exit ends the watch." },
                "ws": { "type": "object", "description": "WebSocket to open. Each text frame is an event. Cannot be combined with command." }
            },
            "required": ["description"]
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
        let m: MonIn =
            serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;
        if m.description.trim().is_empty() {
            return Ok(ToolOutput {
                result: json!({ "error": "Monitor requires description" }),
                error: Some("missing description".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        let Some(command) = m.command.clone() else {
            let ws_hint = if m.ws.is_some() {
                " (WebSocket monitors are not implemented yet)"
            } else {
                ""
            };
            return Ok(ToolOutput {
                result: json!({
                    "error": format!("Monitor requires `command`{ws_hint}")
                }),
                error: Some("missing command".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        };

        let timeout_ms = if m.persistent {
            0
        } else {
            m.timeout_ms.unwrap_or(300_000).clamp(1_000, 3_600_000)
        };

        let task_id = Uuid::new_v4();
        let job = self.services.insert_background_agent_job(task_id);
        job.set_title(m.description.clone());

        let (program, args): (String, Vec<String>) = if cfg!(target_os = "windows") {
            ("cmd".into(), vec!["/C".into(), command])
        } else {
            ("bash".into(), vec!["-c".into(), command])
        };
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();

        let spawned =
            crate::shell_exec::spawn_background_child(&program, &arg_refs, None, task_id)?;
        let mut child = spawned.child;
        let log_path = spawned.log_path;

        let services = self.services.clone();
        let persistent = m.persistent;
        let timeout = timeout_ms;
        let handle = tokio::spawn(async move {
            let outcome = if persistent {
                match child.wait().await {
                    Ok(st) => (
                        crate::services::BackgroundAgentStatus::Completed,
                        format!("exit_code={}", st.code().unwrap_or(0)),
                    ),
                    Err(e) => {
                        crate::shell_exec::kill_background_child(&mut child);
                        (
                            crate::services::BackgroundAgentStatus::Failed,
                            e.to_string(),
                        )
                    }
                }
            } else {
                match tokio::time::timeout(std::time::Duration::from_millis(timeout), child.wait())
                    .await
                {
                    Ok(Ok(st)) => (
                        crate::services::BackgroundAgentStatus::Completed,
                        format!("exit_code={}", st.code().unwrap_or(0)),
                    ),
                    Ok(Err(e)) => {
                        crate::shell_exec::kill_background_child(&mut child);
                        (
                            crate::services::BackgroundAgentStatus::Failed,
                            e.to_string(),
                        )
                    }
                    Err(_) => {
                        crate::shell_exec::kill_background_child(&mut child);
                        let _ = child.wait().await;
                        (
                            crate::services::BackgroundAgentStatus::Failed,
                            format!("timed out after {timeout}ms"),
                        )
                    }
                }
            };
            services.finish_background_shell(task_id, outcome.0, outcome.1);
        });
        job.set_abort(handle.abort_handle());

        let output_file = crate::shell_exec::nested_output_log_path(task_id)
            .unwrap_or_else(|| log_path.to_string_lossy().into_owned());

        Ok(ToolOutput {
            result: json!({
                "taskId": task_id.to_string(),
                "timeoutMs": timeout,
                "persistent": persistent,
                "output_file": output_file,
                "hint": "Poll with TaskOutput id=<taskId>; stop with TaskStop id=<taskId>."
            }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}
