//! PowerShell / Config / SendUserMessage / Brief / AskUserQuestion / REPL

use crate::services::ToolServices;
use anycode_core::prelude::*;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::process::Command;
use std::sync::Arc;
use std::time::Instant;

pub struct PowerShellTool {
    security_policy: SecurityPolicy,
}

impl PowerShellTool {
    pub fn new(sandbox_mode: bool) -> Self {
        let mut p = SecurityPolicy::interactive_shell();
        p.sandbox_mode = sandbox_mode;
        Self { security_policy: p }
    }
}

#[derive(Deserialize)]
struct PsIn {
    command: String,
}

#[async_trait]
impl Tool for PowerShellTool {
    fn name(&self) -> &str {
        "PowerShell"
    }

    fn description(&self) -> &str {
        "Execute PowerShell on Windows; on Unix returns an error (use Bash)."
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": { "command": { "type": "string" } },
            "required": ["command"]
        })
    }

    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Default
    }

    fn security_policy(&self) -> Option<&SecurityPolicy> {
        Some(&self.security_policy)
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let start = Instant::now();
        if !cfg!(target_os = "windows") {
            return Ok(ToolOutput {
                result: json!({"error": "PowerShell tool is only available on Windows"}),
                error: Some("unsupported platform".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
        let ps: PsIn =
            serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;
        let mut c = Command::new("powershell");
        c.args(["-NoProfile", "-Command", &ps.command]);
        if self.security_policy.sandbox_mode && input.sandbox_mode {
            if let Some(wd) = input.working_directory.as_deref() {
                c.current_dir(wd);
            }
        }
        let output = c.output().map_err(CoreError::IoError)?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Ok(ToolOutput {
            result: json!({
                "stdout": stdout,
                "stderr": stderr,
                "exit_code": output.status.code()
            }),
            error: if output.status.success() {
                None
            } else {
                Some("powershell failed".into())
            },
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[derive(Deserialize)]
struct CfgIn {
    #[serde(default)]
    action: String,
    #[serde(default)]
    key: String,
    #[serde(default)]
    value: serde_json::Value,
}

pub struct ConfigTool {
    services: Arc<ToolServices>,
    policy: SecurityPolicy,
}

impl ConfigTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            services,
            policy: SecurityPolicy::sensitive_mutation(),
        }
    }
}

#[async_trait]
impl Tool for ConfigTool {
    fn name(&self) -> &str {
        "Config"
    }

    fn description(&self) -> &str {
        "Get/set in-memory config overrides (session); does not write ~/.anycode/config.json."
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "enum": ["get", "set", "list"] },
                "key": { "type": "string" },
                "value": {}
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
        let c: CfgIn = serde_json::from_value(input.input).unwrap_or(CfgIn {
            action: "list".into(),
            key: String::new(),
            value: serde_json::Value::Null,
        });
        let act = if c.action.is_empty() {
            "list"
        } else {
            c.action.as_str()
        };
        let result = match act {
            "get" => json!({ "key": c.key, "value": self.services.config_get(&c.key) }),
            "set" => {
                self.services.config_set(c.key.clone(), c.value.clone());
                json!({ "ok": true, "key": c.key })
            }
            _ => json!({ "all": self.services.config_snapshot() }),
        };
        Ok(ToolOutput {
            result,
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[derive(Deserialize)]
struct BriefIn {
    #[serde(default)]
    message: String,
    #[serde(default)]
    text: String,
}

fn brief_body(m: &BriefIn) -> String {
    if !m.text.is_empty() {
        m.text.clone()
    } else {
        m.message.clone()
    }
}

pub struct SendUserMessageTool {
    policy: SecurityPolicy,
}

impl SendUserMessageTool {
    pub fn new() -> Self {
        Self {
            policy: SecurityPolicy {
                require_approval: false,
                ..SecurityPolicy::sensitive_mutation()
            },
        }
    }
}

#[async_trait]
impl Tool for SendUserMessageTool {
    fn name(&self) -> &str {
        "SendUserMessage"
    }

    fn description(&self) -> &str {
        "Primary user-visible message channel (Claude Code Brief / SendUserMessage)."
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "message": { "type": "string" },
                "text": { "type": "string" }
            }
        })
    }

    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Auto
    }

    fn security_policy(&self) -> Option<&SecurityPolicy> {
        Some(&self.policy)
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let start = Instant::now();
        let m: BriefIn = serde_json::from_value(input.input).unwrap_or(BriefIn {
            message: String::new(),
            text: String::new(),
        });
        let body = brief_body(&m);
        Ok(ToolOutput {
            result: json!({ "delivered": true, "body": body }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

pub struct BriefTool {
    inner: SendUserMessageTool,
}

impl BriefTool {
    pub fn new() -> Self {
        Self {
            inner: SendUserMessageTool::new(),
        }
    }
}

#[async_trait]
impl Tool for BriefTool {
    fn name(&self) -> &str {
        "Brief"
    }

    fn description(&self) -> &str {
        "Legacy alias for SendUserMessage."
    }

    fn schema(&self) -> serde_json::Value {
        self.inner.schema()
    }

    fn permission_mode(&self) -> PermissionMode {
        self.inner.permission_mode()
    }

    fn security_policy(&self) -> Option<&SecurityPolicy> {
        self.inner.security_policy()
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        self.inner.execute(input).await
    }
}

#[derive(Deserialize)]
struct Opt {
    label: String,
    #[serde(default, rename = "description")]
    _description: String,
}

#[derive(Deserialize)]
struct QIn {
    #[serde(default)]
    question: String,
    #[serde(default)]
    header: String,
    #[serde(default)]
    options: Vec<Opt>,
    #[serde(default)]
    multi_select: bool,
}

pub struct AskUserQuestionTool {
    policy: SecurityPolicy,
}

impl AskUserQuestionTool {
    pub fn new() -> Self {
        Self {
            policy: SecurityPolicy::sensitive_mutation(),
        }
    }
}

#[async_trait]
impl Tool for AskUserQuestionTool {
    fn name(&self) -> &str {
        "AskUserQuestion"
    }

    fn description(&self) -> &str {
        "Ask multiple-choice questions (non-interactive fallback: first option)."
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "question": { "type": "string" },
                "header": { "type": "string" },
                "options": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "label": { "type": "string" },
                            "description": { "type": "string" }
                        }
                    }
                },
                "multiSelect": { "type": "boolean" }
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
        let q: QIn = serde_json::from_value(input.input).unwrap_or(QIn {
            question: String::new(),
            header: String::new(),
            options: vec![],
            multi_select: false,
        });
        let first = q
            .options
            .first()
            .map(|o| o.label.clone())
            .unwrap_or_default();
        Ok(ToolOutput {
            result: json!({
                "selected": if q.multi_select { vec![first.clone()] } else { vec![first.clone()] },
                "note": "Interactive UI not wired; returned first option",
                "question": q.question,
                "header": q.header
            }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

pub struct ReplTool {
    policy: SecurityPolicy,
}

impl ReplTool {
    pub fn new() -> Self {
        Self {
            policy: SecurityPolicy::sensitive_mutation(),
        }
    }
}

#[async_trait]
impl Tool for ReplTool {
    fn name(&self) -> &str {
        "REPL"
    }

    fn description(&self) -> &str {
        "REPL evaluation (host VM not available in anyCode; echo only)."
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": true
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
        Ok(ToolOutput {
            result: json!({
                "info": "REPL not executed",
                "input": input.input
            }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}
