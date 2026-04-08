//! `Agent`、`Skill`、`SendMessage`、旧版子代理名 `Task`。
//!
//! **Claude Code 对齐**：接受 `subagent_type`（同 `agent_type`）、可选 `description`、可选 `cwd`；
//! 成功/失败结果中带 `status`、`agent_id`（= `nested_task_id`）、`output_file`（`~/.anycode/tasks/<id>/output.log`）、
//! 以及类 Claude 的 `content: [{type,text}]`。`SendMessage` 接受 `to` 作为 `recipient` 别名。

use crate::services::ToolServices;
use crate::skills::{truncate_skill_output, SkillCatalog, MAX_SKILL_OUTPUT_BYTES};
use anycode_core::prelude::*;
use anycode_core::DiskTaskOutput;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::path::Path;
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
fn normalize_subagent_type_name(raw: &str) -> String {
    let t = raw.trim();
    if t.is_empty() {
        return String::new();
    }
    match t.to_ascii_lowercase().as_str() {
        "explore" => "explore".to_string(),
        "plan" => "plan".to_string(),
        "general-purpose" | "general_purpose" => "general-purpose".to_string(),
        // Claude built-in we do not ship standalone — fall back so the run still works.
        "verification" | "claude-code-guide" | "statusline-setup" => "general-purpose".to_string(),
        _ => t.to_string(),
    }
}

struct SubAgentDepthGuard<'a> {
    services: &'a ToolServices,
}

impl Drop for SubAgentDepthGuard<'_> {
    fn drop(&mut self) {
        self.services.leave_sub_agent_depth();
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
}

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
        if !self.services.try_enter_sub_agent_depth() {
            return Ok(ToolOutput {
                result: serde_json::json!({ "error": "sub-agent nesting depth exceeded" }),
                error: Some("max sub-agent depth".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
        let _guard = SubAgentDepthGuard {
            services: self.services.as_ref(),
        };

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
        let wd = v
            .cwd
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or(base_wd);

        let desc = v
            .description
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let NestedTaskRun { task_id, result } = exe
            .run_nested_task(
                AgentType::new(agent_type_owned.clone()),
                prompt.clone(),
                wd.clone(),
            )
            .await?;
        let nested_task_id = task_id.to_string();
        let output_file = nested_output_log_path(task_id);

        match result {
            TaskResult::Success { output, artifacts } => {
                let content_text = output.clone();
                Ok(ToolOutput {
                    result: json!({
                        "status": "completed",
                        "output": output,
                        "content": [{ "type": "text", "text": content_text }],
                        "artifacts_count": artifacts.len(),
                        "nested_task_id": &nested_task_id,
                        "agent_id": &nested_task_id,
                        "output_file": output_file,
                        "agent_type": &agent_type_owned,
                        "subagent_type_resolved": &agent_type_owned,
                        "working_directory": wd,
                        "prompt": prompt,
                        "description": desc,
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
                    "nested_task_id": &nested_task_id,
                    "agent_id": &nested_task_id,
                    "output_file": output_file,
                    "agent_type": &agent_type_owned,
                    "subagent_type_resolved": &agent_type_owned,
                    "working_directory": wd,
                    "prompt": prompt,
                    "description": desc,
                }),
                error: Some("subtask failed".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            }),
            TaskResult::Partial { success, remaining } => Ok(ToolOutput {
                result: json!({
                    "status": "partial",
                    "partial_success": success,
                    "remaining": remaining,
                    "nested_task_id": &nested_task_id,
                    "agent_id": &nested_task_id,
                    "output_file": output_file,
                    "agent_type": &agent_type_owned,
                    "subagent_type_resolved": &agent_type_owned,
                    "working_directory": wd,
                    "prompt": prompt,
                    "description": desc,
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
        "Nested agent run (same AgentRuntime as the host). Claude Code–compatible fields: `prompt` (or legacy `task`), optional `subagent_type` (alias: `agent_type`; Explore/Plan/general-purpose), optional `description`, optional `cwd` overriding the tool working directory. Results include `status`, `agent_id`, `nested_task_id`, `output_file` (path to output.log when HOME is set), and `content` like Claude sync results. Nesting depth capped (~6)."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "prompt": { "type": "string", "description": "Task for the agent (Claude Code primary field)" },
                "task": { "type": "string", "description": "Alias of prompt" },
                "description": { "type": "string", "description": "Short human-readable summary (Claude Code style, optional)" },
                "agent_type": { "type": "string", "description": "anyCode: explore | plan | general-purpose" },
                "subagent_type": { "type": "string", "description": "Claude Code: same as agent_type (Explore, Plan, …)" },
                "cwd": { "type": "string", "description": "Working directory for the nested agent; overrides tool-call cwd when set" }
            },
            "anyOf": [
                { "required": ["prompt"] },
                { "required": ["task"] }
            ]
        })
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
        "Run a skill's `run` executable from a discovered skill directory (see system prompt \"Available skills\"). Pass `name` (skill id) and optional `args`."
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
        let runner = root.join("run");
        if !runner.is_file() {
            return Ok(ToolOutput {
                result: serde_json::json!({
                    "error": "skill run script not found",
                    "expected_path": runner.to_string_lossy(),
                }),
                error: Some("skill has no run script".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
        let cwd = std::fs::canonicalize(&root).unwrap_or_else(|_| root.clone());
        let timeout = Duration::from_millis(cat.run_timeout_ms.max(1_000));
        let mut cmd = tokio::process::Command::new(&runner);
        cmd.current_dir(&cwd);
        cmd.args(&v.args);
        cmd.kill_on_drop(true);
        if cat.minimal_env {
            cmd.env_clear();
            for k in ["PATH", "HOME", "USER", "TMPDIR", "SYSTEMROOT", "LANG"] {
                if let Ok(val) = std::env::var(k) {
                    cmd.env(k, val);
                }
            }
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
        Ok(ToolOutput {
            result: serde_json::json!({
                "stdout": stdout,
                "stderr": stderr,
                "code": out.status.code()
            }),
            error: if ok {
                None
            } else {
                Some("skill exited non-zero".into())
            },
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
        "Legacy wire name `Task` (Claude Code): same as `Agent` — use `prompt`/`task`, optional `subagent_type`, `description`, `cwd`; default subagent general-purpose."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "prompt": { "type": "string" },
                "task": { "type": "string" },
                "description": { "type": "string" },
                "agent_type": { "type": "string" },
                "subagent_type": { "type": "string" },
                "cwd": { "type": "string" }
            },
            "anyOf": [
                { "required": ["prompt"] },
                { "required": ["task"] }
            ]
        })
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
        assert_eq!(
            normalize_subagent_type_name("Verification"),
            "general-purpose"
        );
    }
}
