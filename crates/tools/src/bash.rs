use crate::services::ToolServices;
use crate::shell_exec::{
    clamp_timeout_ms, kill_background_child, nested_output_log_path, run_foreground,
    spawn_background_child, DEFAULT_TIMEOUT_MS,
};
use anycode_core::prelude::*;
use async_trait::async_trait;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

pub struct BashTool {
    security_policy: SecurityPolicy,
    services: Arc<ToolServices>,
}

impl BashTool {
    pub fn new(sandbox_mode: bool, services: Arc<ToolServices>) -> Self {
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
                timeout_ms: Some(DEFAULT_TIMEOUT_MS),
            },
            services,
        }
    }

    fn check_denied(&self, command: &str) -> bool {
        for pattern in &self.security_policy.deny_commands {
            if command_matches_deny_pattern(command, pattern) {
                return true;
            }
        }
        false
    }
}

/// Naive substring deny caused false positives (e.g. `git add` contains `dd `).
fn command_matches_deny_pattern(command: &str, pattern: &str) -> bool {
    match pattern {
        "dd " => command_has_standalone_dd(command),
        _ => command.contains(pattern),
    }
}

fn command_has_standalone_dd(command: &str) -> bool {
    for segment in command.split(['|', ';']) {
        for part in segment.split("&&") {
            let trimmed = part.trim();
            if trimmed.starts_with("dd ") || trimmed == "dd" {
                return true;
            }
        }
    }
    false
}

fn resolve_cwd(
    policy: &SecurityPolicy,
    input: &ToolInput,
) -> Result<Option<std::path::PathBuf>, CoreError> {
    if policy.sandbox_mode && input.sandbox_mode {
        let wd = input.working_directory.as_deref().ok_or_else(|| {
            CoreError::PermissionDenied(
                "sandbox_mode requires working_directory on tool input".to_string(),
            )
        })?;
        Ok(Some(std::path::PathBuf::from(wd)))
    } else {
        Ok(input
            .working_directory
            .as_ref()
            .map(std::path::PathBuf::from))
    }
}

/// Strip a trailing shell `&` and detect long-running server/watch commands that must not block.
fn coerce_background(
    command: &str,
    explicit: Option<bool>,
) -> (String, bool, Option<&'static str>) {
    let mut cmd = command.trim().to_string();
    let mut bg = explicit == Some(true);
    let mut reason: Option<&'static str> = None;

    // Trailing `&` is a common (broken) attempt to background — treat as run_in_background.
    let trimmed = cmd.trim_end();
    if trimmed.ends_with('&') {
        let without = trimmed.trim_end_matches(['&', ' ', '\t']);
        // Avoid eating `cmd1 && cmd2` — only a sole trailing `&`.
        if !without.ends_with('&') {
            cmd = without.to_string();
            bg = true;
            reason = Some("stripped trailing &; use run_in_background instead");
        }
    }

    if !bg && looks_like_long_running_server(&cmd) {
        bg = true;
        reason = Some("auto background: long-running server/dev command");
    }

    (cmd, bg, reason)
}

fn looks_like_long_running_server(command: &str) -> bool {
    let c = command.to_ascii_lowercase();
    let needles = [
        "http.server",
        "python -m http.server",
        "python3 -m http.server",
        "npm run dev",
        "npm run start:dev",
        "yarn dev",
        "pnpm dev",
        "pnpm run dev",
        "bun run dev",
        "npx vite",
        "webpack-dev-server",
        "next dev",
        "nuxt dev",
        "flask run",
        "uvicorn ",
        "gunicorn ",
        "cargo watch",
        "watchexec ",
        "nodemon ",
        "docker compose up",
        "docker-compose up",
    ];
    needles.iter().any(|n| c.contains(n)) || c.split_whitespace().any(|tok| tok == "vite")
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
            - For long output, the host may truncate; narrow commands (pipes, head) if needed.\n\
            - Default wall-clock timeout is {}ms (`timeout_ms`); on timeout the process tree is killed and an error is returned.\n\
            - Long-running processes (dev servers, `npm run dev`, `python -m http.server`, vite, watchers): \
              set `run_in_background: true`. Do **not** use a trailing `&` — `&` is stripped and treated as background, \
              but prefer the explicit parameter. Prefer binding HTTP servers to `127.0.0.1` (e.g. \
              `python3 -m http.server 8080 --bind 127.0.0.1`). After start, verify with a short curl. \
              Poll `TaskOutput` / cancel with `TaskStop` using `background_task_id`.\n\
            - Known server/dev patterns are auto-backgrounded even without the flag so the session cannot hang.",
            self.description(),
            DEFAULT_TIMEOUT_MS
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
                "timeout": {
                    "type": "number",
                    "description": format!(
                        "Optional timeout in milliseconds for foreground runs (default: {DEFAULT_TIMEOUT_MS}, max: 600000). Ignored when run_in_background is true."
                    )
                },
                "timeout_ms": {
                    "type": "number",
                    "description": format!(
                        "Deprecated alias for `timeout` (default: {DEFAULT_TIMEOUT_MS}, max: 600000)."
                    )
                },
                "description": {
                    "type": "string",
                    "description": "Clear, concise description of what this command does in active voice. Never use words like \"complex\" or \"risk\" — just describe what it does."
                },
                "run_in_background": {
                    "type": "boolean",
                    "description": "When true, start the command in the background immediately and return background_task_id. Use for long-running servers; do not append &. Poll TaskOutput / cancel with TaskStop."
                },
                "dangerouslyDisableSandbox": {
                    "type": "boolean",
                    "description": "Set this to true to dangerously override sandbox mode and run commands without sandboxing."
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
            #[serde(default)]
            timeout: Option<u64>,
            #[serde(default)]
            run_in_background: Option<bool>,
            #[serde(default)]
            description: Option<String>,
            #[serde(default)]
            dangerously_disable_sandbox: Option<bool>,
            #[serde(default, rename = "dangerouslyDisableSandbox")]
            dangerously_disable_sandbox_camel: Option<bool>,
        }

        fn default_timeout() -> u64 {
            DEFAULT_TIMEOUT_MS
        }

        let bash_input: BashInput =
            serde_json::from_value(input.input.clone()).map_err(CoreError::SerializationError)?;

        // 兼容 Claude Code 的 `timeout` 参数名（timeout_ms 仍是 anyCode 别名）
        let effective_timeout = bash_input.timeout.unwrap_or(bash_input.timeout_ms);
        let dangerously_disable_sandbox = bash_input
            .dangerously_disable_sandbox
            .or(bash_input.dangerously_disable_sandbox_camel)
            .unwrap_or(false);
        let description = bash_input.description.clone();

        if self.check_denied(&bash_input.command) {
            return Ok(ToolOutput {
                result: serde_json::json!({"error": "Command denied by security policy"}),
                error: Some("Command denied".to_string()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        let cwd = if dangerously_disable_sandbox {
            // 显式覆盖沙箱：使用传入工作目录（如有），不要求 sandbox 约束
            input
                .working_directory
                .as_ref()
                .map(std::path::PathBuf::from)
        } else {
            resolve_cwd(&self.security_policy, &input)?
        };
        let cwd_ref = cwd.as_deref();

        let (command, run_bg, bg_reason) =
            coerce_background(&bash_input.command, bash_input.run_in_background);

        if run_bg {
            let mut out = self
                .execute_background(&command, cwd_ref, description.as_deref(), start)
                .await?;
            if let Some(reason) = bg_reason {
                if let Some(obj) = out.result.as_object_mut() {
                    obj.insert("auto_background_reason".into(), serde_json::json!(reason));
                }
            }
            return Ok(out);
        }

        let (program, args): (String, Vec<String>) = if cfg!(target_os = "windows") {
            ("cmd".into(), vec!["/C".into(), command.clone()])
        } else {
            ("bash".into(), vec!["-c".into(), command.clone()])
        };
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();

        let capture = run_foreground(
            &program,
            &arg_refs,
            cwd_ref,
            clamp_timeout_ms(effective_timeout),
        )
        .await?;

        if capture.timed_out {
            return Ok(ToolOutput {
                result: serde_json::json!({
                    "error": "timed out",
                    "stdout": capture.stdout,
                    "stderr": capture.stderr,
                    "exit_code": capture.exit_code,
                    "timeout_ms": clamp_timeout_ms(effective_timeout),
                    "hint": "For long-running servers (http.server, npm run dev, vite), use run_in_background: true instead of waiting or appending &.",
                }),
                error: Some("Command timed out".to_string()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        let result = serde_json::json!({
            "stdout": capture.stdout,
            "stderr": capture.stderr,
            "exit_code": capture.exit_code
        });

        let failed = capture.exit_code.is_some_and(|c| c != 0);
        Ok(ToolOutput {
            result,
            error: if failed {
                Some("Command failed".to_string())
            } else {
                None
            },
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

impl BashTool {
    async fn execute_background(
        &self,
        command: &str,
        cwd: Option<&std::path::Path>,
        description: Option<&str>,
        start: Instant,
    ) -> Result<ToolOutput, CoreError> {
        let task_id = Uuid::new_v4();
        let job = self.services.insert_background_agent_job(task_id);
        if let Some(desc) = description {
            job.set_title(desc.to_string());
        }

        let (program, args): (String, Vec<String>) = if cfg!(target_os = "windows") {
            ("cmd".into(), vec!["/C".into(), command.to_string()])
        } else {
            ("bash".into(), vec!["-c".into(), command.to_string()])
        };
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();

        let BackgroundSpawn {
            log_path,
            mut child,
        } = {
            let spawned = spawn_background_child(&program, &arg_refs, cwd, task_id)?;
            BackgroundSpawn {
                log_path: spawned.log_path,
                child: spawned.child,
            }
        };

        let services = self.services.clone();
        let handle = tokio::spawn(async move {
            let status = child.wait().await;
            match status {
                Ok(st) if st.success() => {
                    services.finish_background_shell(
                        task_id,
                        crate::services::BackgroundAgentStatus::Completed,
                        format!("exit_code={}", st.code().unwrap_or(0)),
                    );
                }
                Ok(st) => {
                    services.finish_background_shell(
                        task_id,
                        crate::services::BackgroundAgentStatus::Failed,
                        format!("exit_code={}", st.code().unwrap_or(-1)),
                    );
                }
                Err(e) => {
                    kill_background_child(&mut child);
                    services.finish_background_shell(
                        task_id,
                        crate::services::BackgroundAgentStatus::Failed,
                        e.to_string(),
                    );
                }
            }
        });
        job.set_abort(handle.abort_handle());

        let output_file = nested_output_log_path(task_id)
            .unwrap_or_else(|| log_path.to_string_lossy().into_owned());

        Ok(ToolOutput {
            result: serde_json::json!({
                "status": "started",
                "background": true,
                "background_task_id": task_id.to_string(),
                "nested_task_id": task_id.to_string(),
                "output_file": output_file,
                "hint": "Poll TaskOutput with id=background_task_id; cancel with TaskStop on the same id.",
            }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

struct BackgroundSpawn {
    log_path: PathBuf,
    child: tokio::process::Child,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::ToolServices;

    fn services() -> Arc<ToolServices> {
        Arc::new(ToolServices::default())
    }

    #[test]
    fn docker_compose_up_is_long_running() {
        assert!(looks_like_long_running_server("docker compose up"));
        assert!(looks_like_long_running_server("docker-compose up -d"));
        assert!(!looks_like_long_running_server("docker compose ps"));
    }

    #[test]
    fn git_add_is_not_blocked_by_dd_deny() {
        let tool = BashTool::new(false, services());
        assert!(
            !tool.check_denied("cd repo && git add crates/sales-metrics/src/lib.rs && git status")
        );
        assert!(!tool.check_denied("git add ."));
    }

    #[test]
    fn dd_command_is_blocked() {
        let tool = BashTool::new(false, services());
        assert!(tool.check_denied("dd if=/dev/zero of=/dev/null bs=1M count=1"));
        assert!(tool.check_denied("echo ok && dd if=/dev/zero of=/tmp/x"));
    }

    #[tokio::test]
    async fn foreground_echo_ok() {
        let tool = BashTool::new(false, services());
        let out = tool
            .execute(ToolInput {
                name: "Bash".into(),
                input: serde_json::json!({"command": "echo hello-bash"}),
                working_directory: None,
                sandbox_mode: false,
            })
            .await
            .expect("echo");
        assert!(out.error.is_none(), "{:?}", out);
        let stdout = out.result["stdout"].as_str().unwrap_or("");
        assert!(stdout.contains("hello-bash"), "{stdout}");
    }

    #[tokio::test]
    async fn foreground_timeout() {
        let tool = BashTool::new(false, services());
        let out = tool
            .execute(ToolInput {
                name: "Bash".into(),
                input: serde_json::json!({
                    "command": "sleep 30",
                    "timeout_ms": 1500
                }),
                working_directory: None,
                sandbox_mode: false,
            })
            .await
            .expect("timeout run");
        assert_eq!(out.error.as_deref(), Some("Command timed out"), "{:?}", out);
        assert_eq!(out.result["error"].as_str(), Some("timed out"));
    }

    #[tokio::test]
    async fn background_starts_immediately() {
        let tool = BashTool::new(false, services());
        let out = tool
            .execute(ToolInput {
                name: "Bash".into(),
                input: serde_json::json!({
                    "command": "sleep 0.2; echo bg-done",
                    "run_in_background": true
                }),
                working_directory: None,
                sandbox_mode: false,
            })
            .await
            .expect("bg");
        assert_eq!(out.result["status"].as_str(), Some("started"));
        let id = out.result["background_task_id"].as_str().expect("id");
        assert!(Uuid::parse_str(id).is_ok());
        tokio::time::sleep(std::time::Duration::from_millis(600)).await;
        if let Some(path) = out.result["output_file"].as_str() {
            if std::path::Path::new(path).is_file() {
                let body = std::fs::read_to_string(path).unwrap_or_default();
                assert!(
                    body.contains("bg-done") || body.is_empty(),
                    "unexpected log: {body}"
                );
            }
        }
    }

    #[test]
    fn coerce_strips_trailing_amp_and_detects_http_server() {
        let (cmd, bg, _) = coerce_background("python3 -m http.server 8080 &", None);
        assert_eq!(cmd, "python3 -m http.server 8080");
        assert!(bg);

        let (_, bg2, reason) = coerce_background("cd /tmp && python3 -m http.server 8080", None);
        assert!(bg2);
        assert!(reason.is_some());

        let (_, bg3, _) = coerce_background("echo hi", None);
        assert!(!bg3);
    }

    #[tokio::test]
    async fn http_server_auto_backgrounds_without_flag() {
        let tool = BashTool::new(false, services());
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
        let port = listener.local_addr().expect("addr").port();
        drop(listener);
        let out = tool
            .execute(ToolInput {
                name: "Bash".into(),
                input: serde_json::json!({
                    "command": format!(
                        "python3 -m http.server {port} --bind 127.0.0.1"
                    ),
                }),
                working_directory: Some("/tmp".into()),
                sandbox_mode: false,
            })
            .await
            .expect("bg http");
        assert_eq!(out.result["status"].as_str(), Some("started"));
        assert!(out.result["auto_background_reason"].as_str().is_some());
        let id = out.result["background_task_id"]
            .as_str()
            .unwrap()
            .to_string();
        // Give server a moment, then hit it (retry — bind/listen can lag briefly).
        let mut ok = None;
        for _ in 0..10 {
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
            ok = tokio::process::Command::new("curl")
                .args([
                    "-s",
                    "-o",
                    "/dev/null",
                    "-w",
                    "%{http_code}",
                    &format!("http://127.0.0.1:{port}/"),
                ])
                .output()
                .await
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok());
            if ok.as_deref() == Some("200") {
                break;
            }
        }
        // Stop background job.
        let _ = tool
            .services
            .cancel_background_agent(Uuid::parse_str(&id).unwrap());
        assert_eq!(
            ok.as_deref(),
            Some("200"),
            "server should respond, got {ok:?}"
        );
    }
}
