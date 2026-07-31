//! Detect “start / preview local site” user intents and optional local preview helpers.
//!
//! Embedded chat **does not** auto-call these helpers (no host short-circuit). The agent
//! reads README / compose files and uses Bash — same as Claude Code.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

const PREVIEW_PORT: u16 = 8080;

/// Whether the user is asking to start or open a local preview / HTTP server.
#[must_use]
pub fn is_start_server_intent(prompt: &str) -> bool {
    let t = prompt.trim().to_lowercase();
    if t.is_empty() {
        return false;
    }
    const NEEDLES: &[&str] = &[
        "启动",
        "起站",
        "打开网站",
        "打开站点",
        "跑起来",
        "跑一下",
        "preview",
        "start the server",
        "start server",
        "start the site",
        "start website",
        "start the website",
        "run the server",
        "http.server",
        "localhost:",
        "npm run dev",
        "yarn dev",
        "pnpm dev",
    ];
    NEEDLES.iter().any(|n| t.contains(&n.to_lowercase()))
}

#[derive(Debug, Clone)]
pub struct PreviewServerStatus {
    pub url: String,
    pub port: u16,
    pub already_running: bool,
    pub started: bool,
    pub working_directory: String,
    pub error: Option<String>,
}

impl PreviewServerStatus {
    #[must_use]
    pub fn ok(&self) -> bool {
        self.error.is_none() && (self.started || self.already_running)
    }

    /// User-facing reply after the host ensured the preview server.
    #[must_use]
    pub fn user_reply_zh(&self) -> String {
        if self.already_running {
            format!(
                "站点已在运行：{}\n目录：{}\n可在浏览器打开该地址。",
                self.url, self.working_directory
            )
        } else {
            format!(
                "已在后台启动预览服务：{}\n目录：{}\n可在浏览器打开该地址。",
                self.url, self.working_directory
            )
        }
    }
}

/// Probe `http://127.0.0.1:PORT/` for a quick 2xx/3xx.
pub async fn preview_server_healthy(port: u16) -> bool {
    let url = format!("http://127.0.0.1:{port}/");
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    match client.get(&url).send().await {
        Ok(resp) => resp.status().is_success() || resp.status().is_redirection(),
        Err(_) => false,
    }
}

fn shell_quote_path(path: &Path) -> String {
    let s = path.to_string_lossy();
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Best-effort: free `PREVIEW_PORT` when something is listening but not serving HTTP OK.
async fn clear_unhealthy_preview_listener(port: u16) {
    if preview_server_healthy(port).await {
        return;
    }
    #[cfg(unix)]
    {
        let _ = tokio::process::Command::new("bash")
            .args([
                "-c",
                &format!(
                    "pids=$(lsof -nP -tiTCP:{port} -sTCP:LISTEN 2>/dev/null); \
                     if [ -n \"$pids\" ]; then kill -KILL $pids 2>/dev/null || true; fi"
                ),
            ])
            .status()
            .await;
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

/// Ensure a static preview server is listening on 127.0.0.1:8080 for `working_directory`.
pub async fn ensure_local_preview_server(working_directory: &str) -> PreviewServerStatus {
    let url = format!("http://127.0.0.1:{PREVIEW_PORT}/");
    let wd_raw = working_directory.trim();
    let wd = if wd_raw.is_empty() {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    } else {
        PathBuf::from(wd_raw)
    };
    let wd_display = wd.display().to_string();

    if preview_server_healthy(PREVIEW_PORT).await {
        return PreviewServerStatus {
            url,
            port: PREVIEW_PORT,
            already_running: true,
            started: false,
            working_directory: wd_display,
            error: None,
        };
    }

    // Stale listener (wrong cwd → 404 / hung accept) blocks bind; clear then start.
    clear_unhealthy_preview_listener(PREVIEW_PORT).await;

    if !wd.is_dir() {
        return PreviewServerStatus {
            url,
            port: PREVIEW_PORT,
            already_running: false,
            started: false,
            working_directory: wd_display,
            error: Some(format!("working directory missing: {}", wd.display())),
        };
    }

    let shell = format!(
        "cd {} && exec python3 -m http.server {PREVIEW_PORT} --bind 127.0.0.1",
        shell_quote_path(&wd)
    );

    let mut cmd = Command::new("bash");
    cmd.args(["-c", &shell]);
    // Preview must outlive this turn; do not kill when the Child handle is dropped.
    cmd.kill_on_drop(false);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());
    #[cfg(unix)]
    {
        // Own process group so the server is not tied to the dashboard TTY.
        cmd.process_group(0);
    }

    match cmd.spawn() {
        Ok(mut child) => {
            // Reap exit status in the background to avoid zombies if the server dies.
            tokio::spawn(async move {
                let _ = child.wait().await;
            });
            tokio::time::sleep(Duration::from_millis(600)).await;
            if preview_server_healthy(PREVIEW_PORT).await {
                PreviewServerStatus {
                    url,
                    port: PREVIEW_PORT,
                    already_running: false,
                    started: true,
                    working_directory: wd_display,
                    error: None,
                }
            } else {
                PreviewServerStatus {
                    url,
                    port: PREVIEW_PORT,
                    already_running: false,
                    started: false,
                    working_directory: wd_display,
                    error: Some(
                        "spawned http.server but health probe failed (port busy or bind error)"
                            .into(),
                    ),
                }
            }
        }
        Err(e) => PreviewServerStatus {
            url,
            port: PREVIEW_PORT,
            already_running: false,
            started: false,
            working_directory: wd_display,
            error: Some(e.to_string()),
        },
    }
}

/// Per-turn ephemeral + user-message guidance after host ensure (or as fallback).
#[must_use]
pub fn start_server_host_hint(
    working_directory: &str,
    status: Option<&PreviewServerStatus>,
) -> String {
    if let Some(st) = status {
        if st.ok() {
            return format!(
                "【启动本地站点 — 主机已完成】预览已在 {} 运行（目录 {}）。\
本回合只允许用 Bash 执行一次 `curl -s -o /dev/null -w '%{{http_code}}' {}` 验证，\
然后用中文把该 URL 告诉用户。禁止 Glob / FileRead / Grep / Edit。",
                st.url, st.working_directory, st.url
            );
        }
        if let Some(err) = &st.error {
            return format!(
                "【启动本地站点 — 主机失败】{err}。请用 Bash（run_in_background: true）在目录 {} 启动 \
`python3 -m http.server {PREVIEW_PORT} --bind 127.0.0.1`，再 curl 验证。禁止只跑 Glob/FileRead。",
                if working_directory.trim().is_empty() {
                    "."
                } else {
                    working_directory.trim()
                }
            );
        }
    }
    let wd = working_directory.trim();
    let cmd = if wd.is_empty() {
        format!("python3 -m http.server {PREVIEW_PORT} --bind 127.0.0.1")
    } else {
        format!("cd {wd} && python3 -m http.server {PREVIEW_PORT} --bind 127.0.0.1")
    };
    format!(
        "【启动本地站点 — 强制】本回合必须调用 Bash（run_in_background: true），禁止只跑 Glob/FileRead。\
建议命令：`{cmd}`。启动后用 curl 检查 http://127.0.0.1:{PREVIEW_PORT}/，再把可打开的 URL 告诉用户。"
    )
}

/// Append host intent block onto the user prompt text (persists in transcript).
#[must_use]
pub fn with_start_server_user_appendix(
    prompt: &str,
    working_directory: &str,
    status: Option<&PreviewServerStatus>,
) -> String {
    let hint = start_server_host_hint(working_directory, status);
    format!(
        "{}\n\n[anycode:host-intent]\n{hint}\nDo this in the first tool round; no exploratory Glob-only turns.",
        prompt.trim()
    )
}

/// Tools to deny while host handles a start-server intent (force Bash verify).
#[must_use]
pub fn start_server_tool_deny_names() -> Vec<String> {
    vec![
        "Glob".into(),
        "Grep".into(),
        "FileRead".into(),
        "FileWrite".into(),
        "Edit".into(),
        "NotebookEdit".into(),
        "PlanWrite".into(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_zh_start() {
        assert!(is_start_server_intent("继续帮我启动下"));
        assert!(is_start_server_intent("帮忙启动一下网站"));
        assert!(!is_start_server_intent("分析当前项目"));
    }

    #[tokio::test]
    async fn ensure_starts_or_reuses_preview() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("index.html"), "<html>ok</html>").unwrap();

        if !preview_server_healthy(PREVIEW_PORT).await {
            // Port held by a non-HTTP process — skip to avoid flaky CI.
            if tokio::net::TcpListener::bind(format!("127.0.0.1:{PREVIEW_PORT}"))
                .await
                .is_err()
            {
                return;
            }
        }

        let st = ensure_local_preview_server(tmp.path().to_str().unwrap()).await;
        assert!(st.ok(), "ensure failed: {:?}", st.error);
        assert!(st.already_running || st.started);
        assert!(preview_server_healthy(PREVIEW_PORT).await);
    }
}
