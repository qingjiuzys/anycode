//! `Agent`、`Skill`、`SendMessage`、旧版子代理名 `Task`。
//!
//! **Claude Code `Agent` 对齐**：`subagent_type`（同 `agent_type`）、可选 `description`、可选 `cwd`（相对则相对工具工作目录，再 canonical 为绝对路径）、
//! 可选 `model`（`sonnet`/`opus`/`haiku` 或裸 id）、可选 `isolation: "worktree"`（临时 git worktree）、`run_in_background: true` 时 `tokio::spawn` 嵌套执行并立即返回 `status: started`（见 `ToolServices` 注册表与 **`TaskStop`** / **`TaskOutput`**）。
//! 成功/失败 JSON 含 `status`、`agent_id`（= `nested_task_id`）、`output_file`、`model`/`isolation` 回显、类 Claude 的 `content: [{type,text}]`。
//! `SendMessage` 接受 `to` 作为 `recipient` 别名。

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
    disarmed: bool,
}

impl<'a> SubAgentDepthGuard<'a> {
    fn new(services: &'a ToolServices) -> Self {
        Self {
            services,
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
            self.services.leave_sub_agent_depth();
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
        let mut depth_guard = SubAgentDepthGuard::new(self.services.as_ref());

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

        let invoke = NestedTaskInvoke {
            agent_type: AgentType::new(agent_type_owned.clone()),
            prompt: prompt.clone(),
            working_directory: wd.clone(),
            model,
            isolation,
            task_id: None,
        };

        if v.run_in_background == Some(true) {
            let task_id = Uuid::new_v4();
            let mut invoke_bg = invoke;
            invoke_bg.task_id = Some(task_id);
            let output_file = nested_output_log_path(task_id);
            let nested_task_id = task_id.to_string();

            let job = self.services.insert_background_agent_job(task_id);
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
                        self.services.leave_sub_agent_depth();
                        self.services
                            .finalize_background_if_still_running(self.task_id);
                    }
                }
                let _g = BgDepthGuard {
                    services: services_c.clone(),
                    task_id,
                };
                let res = exe_c.run_nested_task(invoke_bg).await;
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

        let NestedTaskRun { task_id, result } = exe.run_nested_task(invoke).await?;
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
    fn schema(&self) -> serde_json::Value {
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
        "Legacy wire name `Task` (Claude Code): same as `Agent` — `prompt`/`task`, optional `subagent_type`, `description`, `cwd` (absolute after resolve), `model`, `isolation` (`worktree`); `run_in_background: true` spawns async nested run (TaskOutput / TaskStop)."
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
                "cwd": { "type": "string" },
                "model": { "type": "string" },
                "isolation": { "type": "string" },
                "run_in_background": { "type": "boolean" }
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
            })
            .await
            .unwrap();
        assert_eq!(tout.result["background_status"].as_str(), Some("cancelled"));
    }
}
