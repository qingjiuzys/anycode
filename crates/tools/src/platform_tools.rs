//! PowerShell / Config / SendUserMessage / Brief / AskUserQuestion / REPL

use crate::ask_user_question_host::{
    AskUserQuestionHostError, AskUserQuestionOption, AskUserQuestionRequest,
};
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
struct QIn {
    #[serde(default)]
    question: String,
    #[serde(default)]
    header: String,
    #[serde(default)]
    options: Vec<AskUserQuestionOption>,
    #[serde(default, rename = "multiSelect")]
    multi_select: bool,
}

pub struct AskUserQuestionTool {
    services: Arc<ToolServices>,
    policy: SecurityPolicy,
}

impl AskUserQuestionTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            services,
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
        "Ask multiple-choice questions; requires an interactive host (TTY dialoguer, stream REPL, or fullscreen TUI)."
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
        let q: QIn = serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;
        if q.options.is_empty() {
            return Ok(ToolOutput {
                result: json!({
                    "error": "AskUserQuestion requires at least one option",
                    "status": "unsupported",
                    "question": q.question,
                    "header": q.header
                }),
                error: Some("no options".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        let Some(host) = self.services.ask_user_question_host() else {
            return Ok(ToolOutput {
                result: json!({
                    "error": "No interactive host for AskUserQuestion (non-TTY or channel mode without UI bridge)",
                    "status": "unsupported_host",
                    "hint": "Run from an interactive terminal (`anycode`) or attach a host.",
                    "question": q.question,
                    "header": q.header,
                    "options": q.options
                }),
                error: Some("unsupported_host".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        };

        let req = AskUserQuestionRequest {
            question: q.question.clone(),
            header: q.header.clone(),
            options: q.options.clone(),
            multi_select: q.multi_select,
        };

        match host.ask_user_question(req).await {
            Ok(resp) => Ok(ToolOutput {
                result: json!({
                    "selected": resp.selected_labels,
                    "status": "answered",
                    "question": q.question,
                    "header": q.header
                }),
                error: None,
                duration_ms: start.elapsed().as_millis() as u64,
            }),
            Err(AskUserQuestionHostError(msg)) => Ok(ToolOutput {
                result: json!({
                    "error": msg,
                    "status": "cancelled_or_error",
                    "question": q.question,
                    "header": q.header
                }),
                error: Some(msg),
                duration_ms: start.elapsed().as_millis() as u64,
            }),
        }
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
                "status": "unsupported",
                "info": "REPL not executed",
                "hint": "No host VM is wired for REPL in anyCode runtime.",
                "input": input.input
            }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod ask_user_question_tool_tests {
    use super::*;
    use crate::ask_user_question_host::{
        AskUserQuestionHost, AskUserQuestionRequest, AskUserQuestionResponse,
    };

    struct PickFirst;

    #[async_trait::async_trait]
    impl AskUserQuestionHost for PickFirst {
        async fn ask_user_question(
            &self,
            request: AskUserQuestionRequest,
        ) -> Result<AskUserQuestionResponse, AskUserQuestionHostError> {
            let label = request
                .options
                .first()
                .map(|o| o.label.clone())
                .unwrap_or_default();
            Ok(AskUserQuestionResponse {
                selected_labels: vec![label],
            })
        }
    }

    #[tokio::test]
    async fn no_host_returns_unsupported() {
        let services = Arc::new(ToolServices::default());
        let t = AskUserQuestionTool::new(services);
        let out = t
            .execute(ToolInput {
                name: "AskUserQuestion".into(),
                input: json!({
                    "question": "q?",
                    "options": [{"label": "A"}]
                }),
                working_directory: None,
                sandbox_mode: false,
            })
            .await
            .unwrap();
        assert_eq!(out.error.as_deref(), Some("unsupported_host"));
    }

    #[tokio::test]
    async fn with_host_returns_selected() {
        let services = Arc::new(ToolServices::default());
        services.attach_ask_user_question_host(Arc::new(PickFirst));
        let t = AskUserQuestionTool::new(services);
        let out = t
            .execute(ToolInput {
                name: "AskUserQuestion".into(),
                input: json!({
                    "question": "q?",
                    "options": [{"label": "A"}, {"label": "B"}]
                }),
                working_directory: None,
                sandbox_mode: false,
            })
            .await
            .unwrap();
        assert!(out.error.is_none());
        let sel = out
            .result
            .get("selected")
            .and_then(|v| v.as_array())
            .map(|a| a.len());
        assert_eq!(sel, Some(1));
    }
}
