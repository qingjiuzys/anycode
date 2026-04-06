use anycode_core::prelude::*;
use async_trait::async_trait;
use serde::Deserialize;
use std::process::Command;
use std::time::Instant;

pub struct BashTool {
    security_policy: SecurityPolicy,
}

impl BashTool {
    pub fn new(sandbox_mode: bool) -> Self {
        Self {
            security_policy: SecurityPolicy {
                allow_commands: vec![
                    "git status".to_string(),
                    "git diff".to_string(),
                    "git log".to_string(),
                    "ls".to_string(),
                    "cat".to_string(),
                    "find".to_string(),
                    "grep".to_string(),
                ],
                deny_commands: vec!["rm -rf".to_string(), "dd ".to_string(), ":()>".to_string()],
                require_approval: true,
                sandbox_mode,
                timeout_ms: Some(120_000),
            },
        }
    }

    fn check_denied(&self, command: &str) -> bool {
        for pattern in &self.security_policy.deny_commands {
            if command.contains(pattern) {
                return true;
            }
        }
        false
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "Bash"
    }

    fn description(&self) -> &str {
        "Execute shell commands. Use this for system commands and terminal operations that require shell execution."
    }

    fn api_tool_description(&self) -> String {
        format!(
            "{}\n\n\
            Runs a shell command and returns stdout/stderr/exit code. Prefer this over asking the user to run commands.\n\
            - Use non-interactive flags where possible; assume no TTY.\n\
            - Respect working_directory and sandbox: stay within allowed paths when sandbox_mode is on.\n\
            - Avoid destructive patterns (e.g. recursive rm on project roots); dangerous commands may require approval.\n\
            - For long output, the host may truncate; narrow commands (pipes, head) if needed.",
            self.description()
        )
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Shell command to execute"
                },
                "timeout_ms": {
                    "type": "number",
                    "description": "Timeout in milliseconds (default: 120000)"
                }
            },
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

        #[derive(Deserialize)]
        struct BashInput {
            command: String,
            #[serde(default = "default_timeout")]
            timeout_ms: u64,
        }

        fn default_timeout() -> u64 {
            120000
        }

        let bash_input: BashInput =
            serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;

        let _timeout_ms = bash_input.timeout_ms;

        if self.check_denied(&bash_input.command) {
            return Ok(ToolOutput {
                result: serde_json::json!({"error": "Command denied by security policy"}),
                error: Some("Command denied".to_string()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        let output = if cfg!(target_os = "windows") {
            let mut c = Command::new("cmd");
            c.args(["/C", &bash_input.command]);
            if self.security_policy.sandbox_mode && input.sandbox_mode {
                let wd = input.working_directory.as_deref().ok_or_else(|| {
                    CoreError::PermissionDenied(
                        "sandbox_mode requires working_directory on tool input".to_string(),
                    )
                })?;
                c.current_dir(wd);
            }
            c.output()
        } else {
            let mut c = Command::new("bash");
            c.arg("-c").arg(&bash_input.command);
            if self.security_policy.sandbox_mode && input.sandbox_mode {
                let wd = input.working_directory.as_deref().ok_or_else(|| {
                    CoreError::PermissionDenied(
                        "sandbox_mode requires working_directory on tool input".to_string(),
                    )
                })?;
                c.current_dir(wd);
            }
            c.output()
        };

        let output = output.map_err(CoreError::IoError)?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let result = serde_json::json!({
            "stdout": stdout,
            "stderr": stderr,
            "exit_code": output.status.code()
        });

        Ok(ToolOutput {
            result,
            error: if output.status.success() {
                None
            } else {
                Some("Command failed".to_string())
            },
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}
