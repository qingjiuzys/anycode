//! Task / Team / Cron / RemoteTrigger 编排工具。
//!
//! 在常规 CLI 下，变更会持久化到 `~/.anycode/tasks/orchestration.json`（`ToolServices::load_or_new*` 绑定路径时）；
//! 无用户主目录的 ephemeral 会话中为进程内状态。

use crate::services::{TaskRecord, ToolServices};
use anycode_core::prelude::*;
use async_trait::async_trait;
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
            .insert_task(v.subject, v.description, v.metadata);
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
    id: String,
    #[serde(default)]
    subject: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    metadata: serde_json::Value,
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
                "id": { "type": "string" },
                "subject": { "type": "string" },
                "description": { "type": "string" },
                "status": { "type": "string" },
                "metadata": {}
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
        let u: TuIn = serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;
        let patch = TaskRecord {
            id: u.id.clone(),
            subject: u.subject,
            description: u.description,
            status: u.status,
            metadata: u.metadata,
        };
        let out = self.services.update_task(&u.id, patch);
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
    id: String,
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
        "Get one orchestration task by id."
    }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": { "id": { "type": "string" } },
            "required": ["id"]
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
        let t = self.services.get_task(&g.id);
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
        "Remove a task record by id (same persistence rules as TaskCreate)."
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
        let ok = self.services.remove_task(&g.id);
        Ok(ToolOutput {
            result: json!({ "stopped": ok }),
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
        "Returns the orchestration task record when `id` matches TaskCreate. If `id` is a runtime execution UUID (e.g. `nested_task_id` from the Agent tool), also returns `output_log_path` and a tail of `output.log` under ~/.anycode/tasks/<id>/ when the file exists."
    }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": { "id": { "type": "string" } },
            "required": ["id"]
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
        let t = self.services.get_task(&g.id);

        const TAIL_MAX: usize = 24 * 1024;
        let mut output_log_path: Option<String> = None;
        let mut output_tail: Option<String> = None;
        if let Ok(uid) = Uuid::parse_str(g.id.trim()) {
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
                "output_tail": output_tail,
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
        let ok = self.services.remove_team(&g.id);
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
    schedule: String,
    command: String,
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
        "Register a cron-like job (persisted like other orchestration data; not executed by built-in scheduler in v1)."
    }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "schedule": { "type": "string" },
                "command": { "type": "string" }
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
        let id = self.services.push_cron(c.schedule, c.command);
        Ok(ToolOutput {
            result: json!({ "job_id": id }),
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
