//! Plan / Worktree / ToolSearch / Sleep / StructuredOutput

use crate::services::ToolServices;
use anycode_core::prelude::*;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::process::Command;
use std::sync::Arc;
use std::time::Instant;

pub struct EnterPlanModeTool {
    services: Arc<ToolServices>,
}

impl EnterPlanModeTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self { services }
    }
}

#[async_trait]
impl Tool for EnterPlanModeTool {
    fn name(&self) -> &str {
        "EnterPlanMode"
    }
    fn description(&self) -> &str {
        "Mark session as plan mode (stored in ToolServices)."
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
        self.services.set_plan_mode(true);
        Ok(ToolOutput {
            result: json!({ "plan_mode": true }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

pub struct ExitPlanModeTool {
    services: Arc<ToolServices>,
}

impl ExitPlanModeTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self { services }
    }
}

#[async_trait]
impl Tool for ExitPlanModeTool {
    fn name(&self) -> &str {
        "ExitPlanMode"
    }
    fn description(&self) -> &str {
        "Leave plan mode."
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
        self.services.set_plan_mode(false);
        Ok(ToolOutput {
            result: json!({ "plan_mode": false }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[derive(Deserialize)]
struct EwIn {
    #[serde(default)]
    name: Option<String>,
}

pub struct EnterWorktreeTool {
    services: Arc<ToolServices>,
    policy: SecurityPolicy,
}

impl EnterWorktreeTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            services,
            policy: SecurityPolicy::sensitive_mutation(),
        }
    }
}

#[async_trait]
impl Tool for EnterWorktreeTool {
    fn name(&self) -> &str {
        "EnterWorktree"
    }
    fn description(&self) -> &str {
        "Create a git worktree under the repo and record its path."
    }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "Optional worktree directory name segment" }
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
        let e: EwIn = serde_json::from_value(input.input).unwrap_or(EwIn { name: None });
        let cwd = input
            .working_directory
            .clone()
            .unwrap_or_else(|| ".".to_string());
        let slug = e.name.unwrap_or_else(|| {
            format!(
                "anycode-{}",
                uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
            )
        });
        let path = format!("../wt-{}", slug);
        let cwd_b = cwd.clone();
        let path_b = path.clone();
        let status = tokio::task::spawn_blocking(move || {
            Command::new("git")
                .args(["worktree", "add", &path_b, "HEAD"])
                .current_dir(&cwd_b)
                .status()
        })
        .await
        .map_err(|e| CoreError::Other(anyhow::anyhow!("join: {}", e)))?
        .map_err(CoreError::IoError)?;

        if !status.success() {
            return Ok(ToolOutput {
                result: json!({
                    "error": "git worktree add failed",
                    "path": path,
                    "cwd": cwd
                }),
                error: Some("git failed".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        let abs = std::path::Path::new(&cwd).join(&path);
        let abs_s = abs.to_string_lossy().to_string();
        self.services.set_worktree(Some(abs_s.clone()));

        Ok(ToolOutput {
            result: json!({
                "worktreePath": abs_s,
                "message": "worktree created"
            }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

pub struct ExitWorktreeTool {
    services: Arc<ToolServices>,
    policy: SecurityPolicy,
}

impl ExitWorktreeTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            services,
            policy: SecurityPolicy::sensitive_mutation(),
        }
    }
}

#[async_trait]
impl Tool for ExitWorktreeTool {
    fn name(&self) -> &str {
        "ExitWorktree"
    }
    fn description(&self) -> &str {
        "Clear recorded worktree path (does not remove git worktree on disk)."
    }
    fn schema(&self) -> serde_json::Value {
        json!({"type":"object","properties":{}})
    }
    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Default
    }
    fn security_policy(&self) -> Option<&SecurityPolicy> {
        Some(&self.policy)
    }
    async fn execute(&self, _input: ToolInput) -> Result<ToolOutput, CoreError> {
        let start = Instant::now();
        let prev = self.services.worktree_path();
        self.services.set_worktree(None);
        Ok(ToolOutput {
            result: json!({ "previous": prev, "cleared": true }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[derive(Deserialize)]
struct TsIn {
    #[serde(default)]
    tool_name: String,
    #[serde(default)]
    name: String,
    /// 支持 `select:a,b,c`（对齐 Claude ToolSearch 多选登记）。
    #[serde(default)]
    query: String,
}

fn toolsearch_deferred_names(t: &TsIn) -> Vec<String> {
    let q = t.query.trim();
    if let Some(rest) = q
        .strip_prefix("select:")
        .or_else(|| q.strip_prefix("select :"))
        .map(str::trim_start)
    {
        return rest
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }
    let n = if !t.tool_name.trim().is_empty() {
        t.tool_name.trim().to_string()
    } else {
        t.name.trim().to_string()
    };
    if n.is_empty() {
        vec![]
    } else {
        vec![n]
    }
}

pub struct ToolSearchTool {
    services: Arc<ToolServices>,
}

impl ToolSearchTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self { services }
    }
}

#[async_trait]
impl Tool for ToolSearchTool {
    fn name(&self) -> &str {
        "ToolSearch"
    }
    fn description(&self) -> &str {
        "Defer discovery of tools: tool_name or name, or query \"select:a,b\" for multiple. Unlocks deferred MCP tools when defer_mcp_tools is enabled."
    }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "tool_name": { "type": "string" },
                "name": { "type": "string" },
                "query": { "type": "string", "description": "e.g. select:mcp__srv__tool_a,mcp__srv__tool_b" }
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
        let t: TsIn = serde_json::from_value(input.input).unwrap_or(TsIn {
            tool_name: String::new(),
            name: String::new(),
            query: String::new(),
        });
        let names = toolsearch_deferred_names(&t);
        if names.is_empty() {
            return Ok(ToolOutput {
                result: json!({ "error": "missing tool_name, name, or query select:..." }),
                error: Some("missing tool name".to_string()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
        for n in &names {
            self.services.defer_tool(n.clone());
            self.services.register_mcp_tool_for_llm_session(n);
        }
        Ok(ToolOutput {
            result: json!({ "deferred": names, "all": self.services.deferred_tools() }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[derive(Deserialize)]
struct SleepIn {
    #[serde(default = "default_ms")]
    duration_ms: u64,
}

fn default_ms() -> u64 {
    1000
}

pub struct SleepTool;

#[async_trait]
impl Tool for SleepTool {
    fn name(&self) -> &str {
        "Sleep"
    }
    fn description(&self) -> &str {
        "Async sleep (capped) for proactive pacing."
    }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "duration_ms": { "type": "number", "description": "Milliseconds to wait (max 60s)" }
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
        let s: SleepIn =
            serde_json::from_value(input.input).unwrap_or(SleepIn { duration_ms: 1000 });
        let ms = s.duration_ms.min(60_000).max(1);
        tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
        Ok(ToolOutput {
            result: json!({ "slept_ms": ms }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

pub struct StructuredOutputTool;

#[async_trait]
impl Tool for StructuredOutputTool {
    fn name(&self) -> &str {
        "StructuredOutput"
    }
    fn description(&self) -> &str {
        "Return structured JSON output (passthrough)."
    }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": true
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
        Ok(ToolOutput {
            result: input.input,
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}
