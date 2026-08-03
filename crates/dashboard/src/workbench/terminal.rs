//! Persistent interactive PTY sessions for the workbench Terminal panel.
//!
//! Unlike the original implementation (one PTY per WebSocket connection,
//! destroyed on disconnect), this module keeps a PTY alive for a
//! `conversation_id` and lets multiple connections **attach** to it.
//! Disconnecting a socket does **not** kill the shell — the session lives on
//! until `TerminalSessionManager::close` is called (or the shell exits).

use anyhow::{Context, Result};
use portable_pty::{native_pty_system, Child as PtyChildTrait, CommandBuilder, MasterPty, PtySize};
use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use tokio::sync::broadcast;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TerminalClientMessage {
    Input { data: String },
    Resize { cols: u16, rows: u16 },
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TerminalServerMessage {
    Output { data: String },
    Exit { code: i32 },
    Error { message: String },
}

/// Serialized view of a live terminal session (returned by REST endpoints).
#[derive(Debug, Clone, serde::Serialize)]
pub struct TerminalSessionInfo {
    pub session_id: String,
    pub project_id: String,
    pub conversation_id: String,
}

type PtyWriter = Box<dyn Write + Send>;
type PtyMaster = Box<dyn MasterPty + Send>;
type PtyChild = Box<dyn PtyChildTrait + Send + Sync>;

/// A persistent PTY session. Cloning shares the same underlying shell: every
/// clone writes to the same master, and output fans out to all subscribers.
pub struct PtySession {
    id: String,
    project_id: String,
    conversation_id: String,
    writer: Arc<Mutex<PtyWriter>>,
    master: Arc<Mutex<PtyMaster>>,
    child: Arc<Mutex<Option<PtyChild>>>,
    tx: broadcast::Sender<TerminalServerMessage>,
    /// Ring buffer of recent output so late subscribers can replay the
    /// prompt and any output produced before they attached.
    history: Arc<Mutex<VecDeque<String>>>,
}

impl PtySession {
    pub fn spawn(
        project_id: &str,
        conversation_id: &str,
        cwd: &Path,
    ) -> Result<(Self, broadcast::Receiver<TerminalServerMessage>)> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let shell = std::env::var("SHELL").unwrap_or_else(|_| {
            if cfg!(windows) {
                "cmd.exe".into()
            } else {
                "/bin/zsh".into()
            }
        });

        let mut cmd = CommandBuilder::new(&shell);
        cmd.cwd(cwd);
        if !cfg!(windows) {
            cmd.arg("-l");
        }

        let child = pair.slave.spawn_command(cmd)?;
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader().context("clone pty reader")?;
        let writer = pair.master.take_writer()?;
        let master = pair.master;

        let (tx, rx) = broadcast::channel(256);
        let writer = Arc::new(Mutex::new(writer));
        let master = Arc::new(Mutex::new(master));
        let child = Arc::new(Mutex::new(Some(child)));
        let history: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));
        let reader_tx = tx.clone();
        let exit_tx = tx.clone();
        let history_writer = history.clone();

        let child_waiter = child.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let data = String::from_utf8_lossy(&buf[..n]).into_owned();
                        // Replay buffer for late subscribers. Cap the size so a
                        // long-lived session cannot grow unbounded.
                        if let Ok(mut h) = history_writer.lock() {
                            h.push_back(data.clone());
                            while h.len() > 4096 {
                                h.pop_front();
                            }
                        }
                        let _ = reader_tx.send(TerminalServerMessage::Output { data });
                    }
                    Err(e) => {
                        let _ = reader_tx.send(TerminalServerMessage::Error {
                            message: e.to_string(),
                        });
                        break;
                    }
                }
            }
            // The child handle is consumed here so `close()` can kill a still
            // running shell; once EOF is seen we reap the exit code.
            let code = match child_waiter.lock() {
                Ok(mut guard) => match guard.take() {
                    Some(mut c) => c.wait().map(|s| s.exit_code() as i32).unwrap_or(1),
                    None => 0,
                },
                Err(_) => 1,
            };
            let _ = exit_tx.send(TerminalServerMessage::Exit { code });
        });

        Ok((
            Self {
                id: Uuid::new_v4().to_string(),
                project_id: project_id.to_string(),
                conversation_id: conversation_id.to_string(),
                writer,
                master,
                child,
                tx,
                history,
            },
            rx,
        ))
    }

    pub fn info(&self) -> TerminalSessionInfo {
        TerminalSessionInfo {
            session_id: self.id.clone(),
            project_id: self.project_id.clone(),
            conversation_id: self.conversation_id.clone(),
        }
    }

    /// Subscribe to all output of this session (multi-connection attach).
    pub fn subscribe(&self) -> broadcast::Receiver<TerminalServerMessage> {
        self.tx.subscribe()
    }

    /// Snapshot of recent output, replayed to a newly attached subscriber so
    /// the prompt (and any output before attach) is not lost.
    pub fn replay_history(&self) -> Vec<TerminalServerMessage> {
        let mut out = Vec::new();
        if let Ok(h) = self.history.lock() {
            for data in h.iter() {
                out.push(TerminalServerMessage::Output { data: data.clone() });
            }
        }
        out
    }

    pub fn write_input(&self, data: &str) -> Result<()> {
        let mut w = self
            .writer
            .lock()
            .map_err(|_| anyhow::anyhow!("pty writer poisoned"))?;
        w.write_all(data.as_bytes())?;
        w.flush()?;
        Ok(())
    }

    pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        let master = self
            .master
            .lock()
            .map_err(|_| anyhow::anyhow!("pty master poisoned"))?;
        master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        Ok(())
    }

    /// Terminate the underlying shell (best-effort; no-op if already reaped).
    pub fn kill(&self) {
        if let Ok(mut guard) = self.child.lock() {
            if let Some(mut c) = guard.take() {
                let _ = c.kill();
            }
        }
    }
}

/// Owns the pool of live terminal sessions. Sessions are grouped by
/// `conversation_id` so a workbench session can hold multiple terminal tabs.
#[derive(Default)]
pub struct TerminalSessionManager {
    sessions: RwLock<HashMap<String, Arc<PtySession>>>,
    by_group: RwLock<HashMap<(String, String), Vec<String>>>,
}

impl TerminalSessionManager {
    /// Create a new terminal tab inside a conversation's group and return its
    /// live session (owned) plus a fresh subscriber for the creating socket.
    pub fn create(
        &self,
        project_id: &str,
        conversation_id: &str,
        cwd: &Path,
    ) -> Result<(Arc<PtySession>, broadcast::Receiver<TerminalServerMessage>)> {
        let (session, rx) = PtySession::spawn(project_id, conversation_id, cwd)?;
        let id = session.id.clone();
        let session = Arc::new(session);

        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| anyhow::anyhow!("terminal session map poisoned"))?;
        let mut by_group = self
            .by_group
            .write()
            .map_err(|_| anyhow::anyhow!("terminal group map poisoned"))?;
        sessions.insert(id.clone(), session.clone());
        by_group
            .entry((project_id.to_string(), conversation_id.to_string()))
            .or_default()
            .push(id);

        Ok((session, rx))
    }

    /// List live sessions belonging to a conversation's terminal group.
    pub fn list(&self, project_id: &str, conversation_id: &str) -> Vec<TerminalSessionInfo> {
        let Ok(sessions) = self.sessions.read() else {
            return Vec::new();
        };
        let Ok(by_group) = self.by_group.read() else {
            return Vec::new();
        };
        by_group
            .get(&(project_id.to_string(), conversation_id.to_string()))
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| sessions.get(id).map(|s| s.info()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Fetch a live session by id for attaching an additional socket.
    pub fn get(&self, session_id: &str) -> Option<Arc<PtySession>> {
        self.sessions.read().ok()?.get(session_id).cloned()
    }

    /// Close a session: remove from the pool and kill the shell.
    pub fn close(&self, session_id: &str) -> bool {
        let Ok(mut sessions) = self.sessions.write() else {
            return false;
        };
        let Some(session) = sessions.remove(session_id) else {
            return false;
        };
        drop(sessions);
        let Ok(mut by_group) = self.by_group.write() else {
            session.kill();
            return true;
        };
        let key = (session.project_id.clone(), session.conversation_id.clone());
        if let Some(ids) = by_group.get_mut(&key) {
            ids.retain(|id| id != session_id);
            if ids.is_empty() {
                by_group.remove(&key);
            }
        }
        session.kill();
        true
    }
}

pub fn shared_manager() -> Arc<TerminalSessionManager> {
    static MANAGER: OnceLock<Arc<TerminalSessionManager>> = OnceLock::new();
    MANAGER
        .get_or_init(|| Arc::new(TerminalSessionManager::default()))
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cwd() -> std::path::PathBuf {
        std::env::current_dir().unwrap()
    }

    #[test]
    fn create_groups_sessions_by_conversation() {
        let mgr = TerminalSessionManager::default();
        let (s1, _rx1) = mgr.create("proj-a", "conv-1", &cwd()).unwrap();
        let (s2, _rx2) = mgr.create("proj-a", "conv-1", &cwd()).unwrap();
        let (_s3, _rx3) = mgr.create("proj-a", "conv-2", &cwd()).unwrap();

        let conv1 = mgr.list("proj-a", "conv-1");
        let conv2 = mgr.list("proj-a", "conv-2");
        let other = mgr.list("proj-a", "conv-missing");

        assert_eq!(conv1.len(), 2);
        assert_eq!(conv2.len(), 1);
        assert!(other.is_empty());

        // Distinct session ids, same conversation grouping.
        assert_ne!(s1.info().session_id, s2.info().session_id);
        assert!(conv1.iter().all(|i| i.conversation_id == "conv-1"));
        assert!(conv1.iter().all(|i| i.project_id == "proj-a"));
    }

    #[test]
    fn get_returns_live_session_for_attach() {
        let mgr = TerminalSessionManager::default();
        let (s, _rx) = mgr.create("proj-a", "conv-1", &cwd()).unwrap();
        let id = s.info().session_id;

        let fetched = mgr.get(&id).expect("session should be reachable by id");
        assert_eq!(fetched.info().session_id, id);
        assert!(mgr.get("does-not-exist").is_none());
    }

    #[test]
    fn close_removes_session_and_cleans_group() {
        let mgr = TerminalSessionManager::default();
        let (s1, _rx1) = mgr.create("proj-a", "conv-1", &cwd()).unwrap();
        let (s2, _rx2) = mgr.create("proj-a", "conv-1", &cwd()).unwrap();
        let id1 = s1.info().session_id;
        let id2 = s2.info().session_id;

        // Unknown id -> false, no-op.
        assert!(!mgr.close("nope"));

        assert!(mgr.close(&id1));
        assert!(mgr.get(&id1).is_none());
        let remaining = mgr.list("proj-a", "conv-1");
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].session_id, id2);

        // Closing the last session drops the group entirely.
        assert!(mgr.close(&id2));
        assert!(mgr.list("proj-a", "conv-1").is_empty());
        assert!(!mgr.close(&id2)); // already gone
    }

    #[test]
    fn spawn_echoes_input_on_subscribe() {
        // Verify a freshly created session surfaces output to subscribers and
        // that an additional `subscribe()` attaches to the same live shell.
        let mgr = TerminalSessionManager::default();
        let (s, rx) = mgr.create("proj-a", "conv-1", &cwd()).unwrap();
        let rx2 = s.subscribe();

        // Write a command that prints to stdout; both receivers should see it.
        let cmd = if cfg!(windows) {
            "echo hi\r\n"
        } else {
            "echo hi\n"
        };
        s.write_input(cmd).unwrap();

        let mut got: Vec<String> = Vec::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut rx = rx;
        let mut rx2 = rx2;
        while got.iter().filter(|m| m.contains("hi")).count() < 2 {
            if std::time::Instant::now() > deadline {
                break;
            }
            for r in [&mut rx, &mut rx2] {
                if let Ok(TerminalServerMessage::Output { data }) = r.try_recv() {
                    got.push(data);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(
            got.iter().any(|d| d.contains("hi")),
            "expected echoed output on subscribers, got {got:?}"
        );
        s.kill();
    }

    #[test]
    fn replay_history_surfaces_output_for_late_subscribers() {
        // A freshly created session emits its prompt before any client
        // attaches; a late subscriber must be able to replay it instead of
        // seeing a blank terminal until it presses Enter.
        let mgr = TerminalSessionManager::default();
        let (s, _rx) = mgr.create("proj-a", "conv-1", &cwd()).unwrap();

        // Wait for the shell to produce some output (prompt / banner).
        let mut seen = String::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !seen.contains("echo-ready") {
            if std::time::Instant::now() > deadline {
                break;
            }
            s.write_input("echo-ready\n").unwrap();
            std::thread::sleep(std::time::Duration::from_millis(80));
            seen = s
                .replay_history()
                .into_iter()
                .filter_map(|m| match m {
                    TerminalServerMessage::Output { data } => Some(data),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("");
        }

        assert!(
            seen.contains("echo-ready"),
            "late subscriber should replay historical output, got {seen:?}"
        );
        s.kill();
    }
}
