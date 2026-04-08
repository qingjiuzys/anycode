//! TUI 全屏会话持久化（`~/.anycode/tui-sessions/<uuid>.json`），供 `--resume` 恢复。

use anycode_core::Message;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TuiSessionSnapshot {
    pub version: u32,
    pub id: Uuid,
    pub workspace_root: String,
    pub agent: String,
    pub model: String,
    pub messages: Vec<Message>,
}

fn sessions_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".anycode")
        .join("tui-sessions")
}

pub(crate) fn session_file_path(id: Uuid) -> PathBuf {
    sessions_dir().join(format!("{id}.json"))
}

pub(crate) fn save_tui_session(snap: &TuiSessionSnapshot) -> anyhow::Result<()> {
    let dir = sessions_dir();
    std::fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    let path = session_file_path(snap.id);
    let tmp = path.with_extension("json.tmp");
    let j = serde_json::to_string_pretty(snap).context("serialize tui session")?;
    std::fs::write(&tmp, j).with_context(|| format!("write {}", tmp.display()))?;
    std::fs::rename(&tmp, &path).with_context(|| format!("rename to {}", path.display()))?;
    Ok(())
}

pub(crate) fn load_tui_session(id: Uuid) -> anyhow::Result<Option<TuiSessionSnapshot>> {
    let p = session_file_path(id);
    if !p.is_file() {
        return Ok(None);
    }
    let s = std::fs::read_to_string(&p).with_context(|| format!("read {}", p.display()))?;
    let snap: TuiSessionSnapshot =
        serde_json::from_str(&s).with_context(|| format!("parse {}", p.display()))?;
    Ok(Some(snap))
}

/// 后台写入会话（turn 完成时节流调用；失败仅打 log）。
pub(crate) fn spawn_persist_tui_session(
    id: Uuid,
    workspace_root: String,
    agent: String,
    model: String,
    messages: Arc<Mutex<Vec<Message>>>,
) {
    tokio::spawn(async move {
        let vec = messages.lock().await.clone();
        let snap = TuiSessionSnapshot {
            version: 1,
            id,
            workspace_root,
            agent,
            model,
            messages: vec,
        };
        if let Err(e) = save_tui_session(&snap) {
            tracing::warn!(target: "anycode_cli", "tui session persist: {e:#}");
        }
    });
}
