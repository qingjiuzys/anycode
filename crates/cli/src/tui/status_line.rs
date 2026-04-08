//! Claude 风格 status line：向配置的 shell 命令写入 JSON（stdin），读取 stdout 显示在 TUI 底栏。

use crate::app_config::{effective_session_context_window_tokens, SessionConfig};
use anycode_core::Usage;
use serde::Serialize;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::task::JoinHandle;
use tokio::time::{timeout, Duration};

const DEBOUNCE_MS: u64 = 300;

/// 与 Claude Code `StatusLineCommandInput` 对齐的子集（snake_case JSON）。
#[derive(Debug, Serialize)]
pub(crate) struct StatusLinePayload<'a> {
    pub version: &'a str,
    pub session_id: &'a str,
    pub cwd: &'a str,
    pub model: ModelInfo<'a>,
    pub workspace: WorkspaceInfo<'a>,
    pub context_window: ContextWindowInfo,
}

#[derive(Debug, Serialize)]
pub(crate) struct ModelInfo<'a> {
    pub id: &'a str,
    pub display_name: &'a str,
}

#[derive(Debug, Serialize)]
pub(crate) struct WorkspaceInfo<'a> {
    pub current_dir: &'a str,
    pub project_dir: &'a str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub added_dirs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ContextWindowInfo {
    pub context_window_size: u32,
    pub total_input_tokens: u32,
    pub total_output_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_usage: Option<CurrentUsageInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used_percentage: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_percentage: Option<f64>,
}

#[derive(Debug, Serialize)]
pub(crate) struct CurrentUsageInfo {
    pub input_tokens: u32,
    pub output_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u32>,
}

/// 构建 stdin JSON。
pub(crate) fn build_status_line_payload(
    version: &str,
    session_id: &str,
    cwd: &str,
    project_dir: &str,
    model_id: &str,
    session: &SessionConfig,
    provider_raw: &str,
    last_max_input_tokens: u32,
    last_usage: Option<&Usage>,
) -> anyhow::Result<Vec<u8>> {
    let win = effective_session_context_window_tokens(session, provider_raw, model_id);
    let (total_in, total_out, cur) = if let Some(u) = last_usage {
        (
            u.input_tokens,
            u.output_tokens,
            Some(CurrentUsageInfo {
                input_tokens: u.input_tokens,
                output_tokens: u.output_tokens,
                cache_creation_input_tokens: u.cache_creation_tokens,
                cache_read_input_tokens: u.cache_read_tokens,
            }),
        )
    } else if last_max_input_tokens > 0 {
        (
            last_max_input_tokens,
            0u32,
            Some(CurrentUsageInfo {
                input_tokens: last_max_input_tokens,
                output_tokens: 0,
                cache_creation_input_tokens: None,
                cache_read_input_tokens: None,
            }),
        )
    } else {
        (0u32, 0u32, None)
    };

    let (used_pct, rem_pct) = if win > 0 && total_in > 0 {
        let u = (total_in as f64 / win as f64) * 100.0;
        let u = u.min(100.0);
        (Some(u), Some((100.0 - u).max(0.0)))
    } else {
        (None, None)
    };

    let p = StatusLinePayload {
        version,
        session_id,
        cwd,
        model: ModelInfo {
            id: model_id,
            display_name: model_id,
        },
        workspace: WorkspaceInfo {
            current_dir: cwd,
            project_dir,
            added_dirs: vec![],
        },
        context_window: ContextWindowInfo {
            context_window_size: win,
            total_input_tokens: total_in,
            total_output_tokens: total_out,
            current_usage: cur,
            used_percentage: used_pct,
            remaining_percentage: rem_pct,
        },
    };
    Ok(serde_json::to_vec(&p)?)
}

/// 内置一行（无 `command` 且 `show_builtin`）。
pub(crate) fn format_builtin_status_line(
    model_id: &str,
    session: &SessionConfig,
    provider_raw: &str,
    last_max_input_tokens: u32,
) -> String {
    let win = effective_session_context_window_tokens(session, provider_raw, model_id);
    if win == 0 {
        return format!("{model_id} · ctx —");
    }
    if last_max_input_tokens == 0 {
        return format!("{model_id} · ctx 0% / {win} tok");
    }
    let pct = ((last_max_input_tokens as f64 / win as f64) * 100.0).min(100.0);
    format!("{model_id} · ctx {pct:.0}% / {win} tok")
}

/// 剥离 CSI `\x1b[ ... m` 等序列。
pub(crate) fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\x1b' {
            if it.peek() == Some(&'[') {
                it.next();
                while let Some(ch) = it.next() {
                    if ('\x40'..='\x7e').contains(&ch) {
                        break;
                    }
                }
                continue;
            }
        }
        out.push(c);
    }
    out
}

pub(crate) fn normalize_status_stdout(raw: &str) -> String {
    let s = strip_ansi(raw.trim());
    let line = s.lines().next().unwrap_or("").trim();
    if line.is_empty() {
        " ".to_string()
    } else {
        line.to_string()
    }
}

/// 异步执行 `sh -c command`，stdin 为 JSON。
pub(crate) async fn run_status_line_command(
    command: &str,
    stdin_json: &[u8],
    timeout_ms: u64,
) -> anyhow::Result<String> {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()?;

    let mut stdin = child.stdin.take().ok_or_else(|| anyhow::anyhow!("stdin"))?;
    stdin.write_all(stdin_json).await?;
    stdin.flush().await?;
    drop(stdin);

    let dur = Duration::from_millis(timeout_ms.max(100));
    let out = timeout(dur, async move {
        let o = child.wait_with_output().await?;
        anyhow::ensure!(o.status.success(), "status {}", o.status);
        Ok::<_, anyhow::Error>(String::from_utf8_lossy(&o.stdout).into_owned())
    })
    .await
    .map_err(|_| anyhow::anyhow!("status line timeout"))??;

    Ok(normalize_status_stdout(&out))
}

pub(crate) fn debounce_std() -> std::time::Duration {
    std::time::Duration::from_millis(DEBOUNCE_MS)
}

/// TUI 主循环用：与 Claude 一致，在「新 assistant 轮次」或「一轮执行结束」时重新 arm debounce。
/// `last_sl_transcript_gen` 初始化为 `u64::MAX`，首帧会与 `transcript_gen` 不等从而触发一次刷新。
pub(crate) fn status_line_arm_refresh(
    last_sl_transcript_gen: &mut u64,
    transcript_gen: u64,
    prev_executing: bool,
    executing: bool,
) -> bool {
    let mut arm = false;
    if transcript_gen != *last_sl_transcript_gen {
        *last_sl_transcript_gen = transcript_gen;
        arm = true;
    }
    if prev_executing && !executing {
        arm = true;
    }
    arm
}

pub(crate) fn spawn_status_line_task<F>(f: F) -> JoinHandle<F::Output>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    tokio::spawn(f)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_config::SessionConfig;
    use anycode_core::Usage;

    fn session_fixed_window(tokens: u32) -> SessionConfig {
        let mut s = SessionConfig::default();
        s.context_window_auto = false;
        s.context_window_tokens = tokens;
        s
    }

    #[test]
    fn payload_serializes_keys() {
        let session = SessionConfig::default();
        let v = build_status_line_payload(
            "0.2.0", "sid", "/a", "/a", "glm-5", &session, "z.ai", 1000, None,
        )
        .unwrap();
        let s = String::from_utf8(v).unwrap();
        assert!(s.contains("\"session_id\":\"sid\""));
        assert!(s.contains("\"context_window_size\""));
    }

    #[test]
    fn payload_includes_usage_percentages_when_window_and_input_positive() {
        let session = session_fixed_window(100_000);
        let u = Usage {
            input_tokens: 50_000,
            output_tokens: 100,
            cache_creation_tokens: None,
            cache_read_tokens: None,
        };
        let v = build_status_line_payload(
            "0.2.0",
            "sid",
            "/proj",
            "/proj",
            "m",
            &session,
            "z.ai",
            0,
            Some(&u),
        )
        .unwrap();
        let s = String::from_utf8(v).unwrap();
        assert!(s.contains("\"used_percentage\":50"));
        assert!(s.contains("\"remaining_percentage\":50"));
    }

    #[test]
    fn format_builtin_shows_percent_and_window() {
        let session = session_fixed_window(1000);
        let line = format_builtin_status_line("my-model", &session, "z.ai", 250);
        assert!(line.contains("my-model"));
        assert!(line.contains("25%"));
        assert!(line.contains("1000"));
    }

    #[test]
    fn format_builtin_zero_window_uses_dash() {
        let session = session_fixed_window(0);
        let line = format_builtin_status_line("m", &session, "z.ai", 10);
        assert!(line.contains("ctx —"));
    }

    #[test]
    fn format_builtin_zero_usage_shows_zero_pct() {
        let session = session_fixed_window(8000);
        let line = format_builtin_status_line("m", &session, "z.ai", 0);
        assert!(line.contains("0%"));
    }

    #[test]
    fn normalize_status_stdout_empty_becomes_space() {
        assert_eq!(normalize_status_stdout("  \n  "), " ");
    }

    #[test]
    fn normalize_status_stdout_multiline_first_line_wins() {
        assert_eq!(normalize_status_stdout("  hello  \nignored\n"), "hello");
    }

    #[test]
    fn strip_ansi_strips_simple() {
        assert_eq!(strip_ansi("\x1b[0;32mok\x1b[0m"), "ok");
    }

    #[test]
    fn debounce_std_matches_claude_style_300ms() {
        assert_eq!(debounce_std().as_millis(), 300);
    }

    #[test]
    fn status_line_arm_refresh_first_transcript_updates_tracker() {
        let mut last = u64::MAX;
        assert!(status_line_arm_refresh(&mut last, 0, false, false));
        assert_eq!(last, 0);
    }

    #[test]
    fn status_line_arm_refresh_same_gen_idle_no_arm() {
        let mut last = 1u64;
        assert!(!status_line_arm_refresh(&mut last, 1, false, false));
        assert_eq!(last, 1);
    }

    #[test]
    fn status_line_arm_refresh_on_turn_end() {
        let mut last = 5u64;
        assert!(status_line_arm_refresh(&mut last, 5, true, false));
        assert_eq!(last, 5);
    }

    #[test]
    fn status_line_arm_refresh_new_gen_and_turn_end_same_tick() {
        let mut last = 0u64;
        assert!(status_line_arm_refresh(&mut last, 1, true, false));
        assert_eq!(last, 1);
    }

    #[tokio::test]
    async fn run_status_line_command_cat_roundtrips_json() {
        let out = run_status_line_command("cat", br#"{"pipe":"ok"}"#, 4000)
            .await
            .unwrap();
        assert!(out.contains("pipe"));
    }

    #[tokio::test]
    async fn run_status_line_command_times_out() {
        let err = run_status_line_command("sleep 10", b"{}", 80).await;
        assert!(err.is_err(), "expected timeout, got {err:?}");
    }
}
