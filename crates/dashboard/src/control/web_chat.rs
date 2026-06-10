//! Long-lived Web chat sessions backed by non-TTY `anycode` line REPL.

use crate::cancel_ipc::dashboard_state_dir;
use crate::control::task_trigger::resolve_anycode_binary;
use crate::db::DashboardDb;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::Mutex;

const SPAWN_TIMEOUT: Duration = Duration::from_secs(10);
const WRITE_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebChatSendResult {
    pub session_id: String,
    pub pid: u32,
    pub log_path: String,
    pub started_at: String,
    pub queued: bool,
}

#[derive(Default, Clone)]
pub struct WebChatHub {
    sessions: Arc<Mutex<HashMap<String, Arc<Mutex<WebChatProcess>>>>>,
    launch_locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
}

struct WebChatProcess {
    pid: u32,
    log_path: PathBuf,
    started_at: String,
    stdin: ChildStdin,
    child: Arc<Mutex<Option<Child>>>,
}

impl WebChatHub {
    /// Terminate a cached REPL and drop hub state (e.g. agent profile changed or session cancelled).
    pub async fn evict(&self, session_id: &str) {
        let entry = self.sessions.lock().await.remove(session_id);
        self.launch_locks.lock().await.remove(session_id);
        if let Some(entry) = entry {
            let _ = crate::cancel_ipc::request_cancel(session_id);
            if let Ok(proc) = entry.try_lock() {
                terminate_child(&proc.child).await;
            }
        }
    }

    pub async fn send(
        &self,
        db: DashboardDb,
        session_id: &str,
        project_root: &Path,
        agent: Option<&str>,
        dashboard_url: &str,
        prompt: &str,
        vision_images: Option<&[crate::control::vision_payload::VisionImagePayload]>,
    ) -> Result<WebChatSendResult> {
        let prompt = prompt.trim();
        if prompt.is_empty() && vision_images.is_none_or(|v| v.is_empty()) {
            bail!("message is required");
        }
        let root = crate::project_root::ensure_project_root(project_root, false)?;
        let existing = {
            let sessions = self.sessions.lock().await;
            sessions.get(session_id).cloned()
        };
        let entry = if let Some(existing) = existing {
            existing
        } else {
            let launch_lock = {
                let mut locks = self.launch_locks.lock().await;
                locks
                    .entry(session_id.to_string())
                    .or_insert_with(|| Arc::new(Mutex::new(())))
                    .clone()
            };
            let _launch_guard = launch_lock.lock().await;
            let existing = {
                let sessions = self.sessions.lock().await;
                sessions.get(session_id).cloned()
            };
            if let Some(existing) = existing {
                existing
            } else {
                let proc = tokio::time::timeout(
                    SPAWN_TIMEOUT,
                    self.spawn_process(db, session_id.to_string(), &root, agent, dashboard_url),
                )
                .await
                .map_err(|_| anyhow::anyhow!("timed out starting web chat process"))??;
                let proc = Arc::new(Mutex::new(proc));
                self.sessions
                    .lock()
                    .await
                    .insert(session_id.to_string(), proc.clone());
                proc
            }
        };

        let mut proc = entry
            .try_lock()
            .map_err(|_| anyhow::anyhow!("web chat session is busy; wait for the current send"))?;
        tokio::time::timeout(WRITE_TIMEOUT, async {
            if let Some(images) = vision_images.filter(|v| !v.is_empty()) {
                if let Some(path) =
                    crate::control::vision_payload::write_vision_payload(session_id, images)?
                {
                    proc.stdin
                        .write_all(
                            crate::control::vision_payload::vision_file_line(&path).as_bytes(),
                        )
                        .await
                        .context("write vision file line to web chat session")?;
                }
            }
            if !prompt.is_empty() {
                proc.stdin
                    .write_all(prompt.as_bytes())
                    .await
                    .context("write prompt to web chat session")?;
            } else {
                proc.stdin
                    .write_all(b"Please describe or analyze this image.")
                    .await
                    .context("write default vision prompt to web chat session")?;
            }
            proc.stdin
                .write_all(b"\n")
                .await
                .context("flush prompt newline to web chat session")?;
            proc.stdin.flush().await.context("flush web chat stdin")?;
            Ok::<(), anyhow::Error>(())
        })
        .await
        .map_err(|_| anyhow::anyhow!("timed out writing prompt to web chat session"))??;
        Ok(WebChatSendResult {
            session_id: session_id.to_string(),
            pid: proc.pid,
            log_path: proc.log_path.display().to_string(),
            started_at: proc.started_at.clone(),
            queued: true,
        })
    }

    async fn spawn_process(
        &self,
        db: DashboardDb,
        session_id: String,
        root: &Path,
        agent: Option<&str>,
        dashboard_url: &str,
    ) -> Result<WebChatProcess> {
        let dir = dashboard_state_dir().join("web-chat");
        std::fs::create_dir_all(&dir)?;
        let log_path = dir.join(format!("{session_id}.log"));
        let exe = resolve_anycode_binary()?;
        let mut cmd = Command::new(&exe);
        cmd.arg("-C").arg(root);
        if let Some(agent) = agent.map(str::trim).filter(|a| !a.is_empty()) {
            cmd.arg("--agent").arg(agent);
        }
        let log_file = std::fs::File::create(&log_path).context("create web chat log")?;
        let err_file = log_file.try_clone().context("clone web chat log fd")?;
        cmd.current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(log_file))
            .stderr(Stdio::from(err_file))
            .env("ANYCODE_DASHBOARD_RECORD", "1")
            .env("ANYCODE_DASHBOARD_DB", db.path())
            .env("ANYCODE_DASHBOARD_URL", dashboard_url)
            .env("ANYCODE_MEMORY_ATTACH", "shared")
            .env(crate::ipc::approval_ipc::SESSION_ENV, &session_id)
            .env("ANYCODE_DASHBOARD_SESSION_STICKY", "1");
        let mut child = cmd.spawn().context("spawn anycode web chat repl")?;
        let pid = child.id().unwrap_or(0);
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("web chat stdin unavailable"))?;
        let child_handle = Arc::new(Mutex::new(Some(child)));
        let started_at = chrono::Utc::now().to_rfc3339();
        let watch_session_id = session_id.clone();
        let watch_log = log_path.clone();
        let watch_sessions = self.sessions.clone();
        let watch_launch_locks = self.launch_locks.clone();
        let watch_child = child_handle.clone();
        tokio::spawn(async move {
            if let Some(mut child) = watch_child.lock().await.take() {
                let _ = child.wait().await;
            }
            remove_finished_process(&watch_sessions, &watch_session_id, pid).await;
            watch_launch_locks.lock().await.remove(&watch_session_id);
            let summary = crate::db::read_log_excerpt(&watch_log).unwrap_or_else(|| {
                format!(
                    "Interactive web chat process exited. See log: {}",
                    watch_log.display()
                )
            });
            if let Ok(Some(sess)) = db.get_session(&watch_session_id).await {
                if sess.status == "running" || sess.status == "pending" {
                    let _ = db
                        .finish_session(&watch_session_id, "failed", Some(&summary))
                        .await;
                }
            }
        });
        Ok(WebChatProcess {
            pid,
            log_path,
            started_at,
            stdin,
            child: child_handle,
        })
    }
}

async fn terminate_child(child: &Arc<Mutex<Option<Child>>>) {
    if let Some(mut proc) = child.lock().await.take() {
        let _ = proc.kill().await;
        let _ = proc.wait().await;
    }
}

async fn remove_finished_process(
    sessions: &Arc<Mutex<HashMap<String, Arc<Mutex<WebChatProcess>>>>>,
    session_id: &str,
    pid: u32,
) {
    let entry = sessions.lock().await.get(session_id).cloned();
    let Some(entry) = entry else {
        return;
    };
    let remove = entry
        .try_lock()
        .map(|proc| proc.pid == pid)
        .unwrap_or(false);
    if !remove {
        return;
    }
    let mut sessions = sessions.lock().await;
    if let Some(current) = sessions.get(session_id) {
        if Arc::ptr_eq(current, &entry) {
            sessions.remove(session_id);
        }
    }
}
