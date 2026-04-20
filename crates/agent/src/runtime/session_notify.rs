//! 会话外向通知：HTTP POST + stdin 喂 JSON 的 shell（失败仅打日志，不阻断编排）。

use super::artifacts::truncate_text;
use anycode_core::{SessionNotificationSettings, TaskId};
use serde_json::{json, Value};
use std::time::Duration;
use tracing::warn;

pub(crate) fn expand_env_placeholders(s: &str) -> String {
    let mut out = String::with_capacity(s.len().saturating_mul(2));
    let mut rest = s;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        rest = &rest[start + 2..];
        if let Some(end) = rest.find('}') {
            let name = rest[..end].trim();
            let val = std::env::var(name).unwrap_or_default();
            out.push_str(&val);
            rest = &rest[end + 1..];
        } else {
            out.push_str("${");
            out.push_str(rest);
            return out;
        }
    }
    out.push_str(rest);
    out
}

pub(crate) fn build_notification_value(
    event: &str,
    session_id: &str,
    task_id: TaskId,
    turn: usize,
    tool_name: Option<&str>,
    excerpt: &str,
    cwd: Option<&str>,
    max_body_bytes: usize,
) -> Value {
    let (excerpt_body, excerpt_truncated) = truncate_text(excerpt.to_string(), max_body_bytes);
    let mut v = json!({
        "schema_version": 1,
        "event": event,
        "session_id": session_id,
        "task_id": task_id.to_string(),
        "turn": turn,
        "excerpt": excerpt_body,
        "excerpt_truncated": excerpt_truncated,
        "timestamp": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    });
    if let Some(tn) = tool_name {
        v["tool_name"] = json!(tn);
    }
    if let Some(c) = cwd.filter(|s| !s.is_empty()) {
        v["working_directory"] = json!(c);
    }
    v
}

pub(crate) fn spawn_dispatch(settings: SessionNotificationSettings, payload: Value) {
    if !settings.is_configured() {
        return;
    }
    tokio::spawn(async move {
        if let Err(e) = dispatch_once(&settings, &payload).await {
            warn!(target: "anycode_agent", "session notification failed: {:#}", e);
        }
    });
}

async fn dispatch_once(
    settings: &SessionNotificationSettings,
    payload: &Value,
) -> anyhow::Result<()> {
    let body = serde_json::to_vec(payload)?;
    if let Some(url) = settings
        .http_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        dispatch_http(settings, url, payload).await?;
    }
    if let Some(cmd) = settings
        .shell_command
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        dispatch_shell(settings, cmd, &body).await?;
    }
    Ok(())
}

async fn dispatch_http(
    settings: &SessionNotificationSettings,
    url: &str,
    payload: &Value,
) -> anyhow::Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(settings.http_timeout_ms.max(100)))
        .build()?;
    let mut req = client.post(url).json(payload);
    for (k, v) in &settings.http_headers {
        let k = k.trim();
        if k.is_empty() {
            continue;
        }
        req = req.header(k, expand_env_placeholders(v));
    }
    let resp = req.send().await?;
    let status = resp.status();
    if !status.is_success() {
        let txt = resp.text().await.unwrap_or_default();
        let brief: String = txt.chars().take(512).collect();
        anyhow::bail!("HTTP status {} body {}", status, brief);
    }
    Ok(())
}

async fn dispatch_shell(
    settings: &SessionNotificationSettings,
    cmd: &str,
    body: &[u8],
) -> anyhow::Result<()> {
    use std::process::Stdio;
    use tokio::io::AsyncWriteExt;
    use tokio::process::Command;
    use tokio::time::timeout;

    let timeout_d = Duration::from_millis(settings.shell_timeout_ms.max(100));

    #[cfg(unix)]
    let mut child = Command::new("/bin/sh")
        .arg("-c")
        .arg(cmd)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    #[cfg(windows)]
    let mut child = Command::new("cmd.exe")
        .arg("/C")
        .arg(cmd)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(body).await?;
        stdin.flush().await?;
    }

    let wait = async {
        let out = child.wait_with_output().await?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            let e: String = err.chars().take(512).collect();
            anyhow::bail!("shell hook exit {:?}: {}", out.status.code(), e);
        }
        Ok::<(), anyhow::Error>(())
    };

    timeout(timeout_d, wait).await??;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn http_dispatch_posts_json() {
        let std_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        std_listener.set_nonblocking(true).unwrap();
        let addr = std_listener.local_addr().unwrap();
        let url = format!("http://127.0.0.1:{}/hook", addr.port());

        let server = tokio::spawn(async move {
            let listener = TcpListener::from_std(std_listener).unwrap();
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut data = vec![0u8; 8192];
            let n = socket.read(&mut data).await.unwrap();
            let s = String::from_utf8_lossy(&data[..n]);
            assert!(s.contains("POST /hook"));
            assert!(s.contains("tool_result"));
            let _ = socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                .await;
        });

        let mut cfg = SessionNotificationSettings::default();
        cfg.http_url = Some(url);
        cfg.shell_command = None;
        let payload = json!({"event": "tool_result", "task_id": "x"});
        dispatch_once(&cfg, &payload).await.unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn shell_dispatch_writes_stdin() {
        let mut cfg = SessionNotificationSettings::default();
        cfg.http_url = None;
        cfg.shell_command = Some("wc -c".to_string());
        let payload = json!({"event": "tool_result", "excerpt": "ab"});
        dispatch_once(&cfg, &payload).await.unwrap();
    }

    #[test]
    fn expand_env_basic() {
        std::env::set_var("ANYCODE_HOOK_TEST_X", "hello");
        let s = expand_env_placeholders("a ${ANYCODE_HOOK_TEST_X} b");
        assert_eq!(s, "a hello b");
        std::env::remove_var("ANYCODE_HOOK_TEST_X");
    }

    #[test]
    fn build_payload_truncates() {
        let tid = uuid::Uuid::new_v4();
        let long = "x".repeat(100);
        let v = build_notification_value(
            "tool_result",
            "sess",
            tid,
            2,
            Some("bash"),
            &long,
            Some("/tmp"),
            20,
        );
        let ex = v["excerpt"].as_str().unwrap();
        assert!(
            ex.contains("truncated") || ex.len() <= 80,
            "excerpt should be bounded, got len {}",
            ex.len()
        );
        assert_eq!(v["excerpt_truncated"], true);
        assert_eq!(v["tool_name"], "bash");
        assert_eq!(v["working_directory"], "/tmp");
    }
}
