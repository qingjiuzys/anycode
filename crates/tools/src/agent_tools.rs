//! `Agent`、`Skill`、`SendMessage`、旧版子代理名 `Task`。

use crate::services::ToolServices;
use crate::skills::{truncate_skill_output, SkillCatalog, MAX_SKILL_OUTPUT_BYTES};
use anycode_core::prelude::*;
use async_trait::async_trait;
use serde::Deserialize;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// 嵌套 Agent 未指定类型时的默认子类型（与常见 `Agent`/`Task` 工具约定一致）。
const DEFAULT_SUBAGENT_AGENT_TYPE: &str = "general-purpose";

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
    #[serde(default)]
    agent_type: Option<String>,
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
            .ok_or_else(|| CoreError::LLMError("字段 prompt 或 task 必填（非空字符串）".into()))?;
        let agent_type = v
            .agent_type
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(default_agent_type);

        let wd = input
            .working_directory
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| ".".to_string());

        match exe
            .run_nested_task(AgentType::new(agent_type.to_string()), prompt, wd)
            .await?
        {
            TaskResult::Success { output, artifacts } => Ok(ToolOutput {
                result: serde_json::json!({
                    "output": output,
                    "artifacts_count": artifacts.len()
                }),
                error: None,
                duration_ms: start.elapsed().as_millis() as u64,
            }),
            TaskResult::Failure { error, details } => Ok(ToolOutput {
                result: serde_json::json!({ "error": error, "details": details }),
                error: Some("subtask failed".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            }),
            TaskResult::Partial { success, remaining } => Ok(ToolOutput {
                result: serde_json::json!({ "partial_success": success, "remaining": remaining }),
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
        "Spawn a nested agent run via the same AgentRuntime（子类型工具集由 agent_type 决定；未指定时默认 general-purpose）。"
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "prompt": { "type": "string", "description": "子任务说明" },
                "task": { "type": "string", "description": "与 prompt 二选一" },
                "agent_type": { "type": "string", "description": "explore | plan | general-purpose；省略时默认 general-purpose" }
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
    #[serde(default)]
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
        "Send a message to another agent/team channel (in-memory queue)."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "recipient": { "type": "string" },
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
        let text = if !m.body.is_empty() {
            m.body
        } else {
            m.message
        };
        self.services.push_message(m.recipient, text.clone());
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
        "Legacy tool name `Task`（与 `Agent` 等价）；默认子 agent 为 general-purpose。"
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "prompt": { "type": "string" },
                "task": { "type": "string" },
                "agent_type": { "type": "string" }
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
        self.inner
            .run_sub_agent(input, DEFAULT_SUBAGENT_AGENT_TYPE)
            .await
    }
}
