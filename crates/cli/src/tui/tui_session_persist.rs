//! TUI 全屏会话持久化（`~/.anycode/tui-sessions/<uuid>.json`），供 `--resume` 恢复。
//!
//! `~/.anycode/tui-sessions/.last-session.json` 记录全局最近一次成功落盘的会话 id，供无参 `/session` 回退。

use anycode_core::Message;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::Mutex;
use uuid::Uuid;

const LAST_SESSION_FILE: &str = ".last-session.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TuiSessionSnapshot {
    pub version: u32,
    pub id: Uuid,
    pub workspace_root: String,
    pub agent: String,
    pub model: String,
    pub messages: Vec<Message>,
}

pub(crate) fn sessions_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".anycode")
        .join("tui-sessions")
}

fn last_session_pointer_path() -> PathBuf {
    sessions_dir().join(LAST_SESSION_FILE)
}

fn last_session_pointer_path_in(sessions_root: &Path) -> PathBuf {
    sessions_root.join(LAST_SESSION_FILE)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LastSessionPointer {
    id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    written_at: Option<String>,
}

fn write_last_session_pointer(id: Uuid) -> anyhow::Result<()> {
    let dir = sessions_dir();
    fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    let path = last_session_pointer_path();
    let ptr = LastSessionPointer {
        id,
        written_at: Some(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
    };
    let tmp = path.with_extension("json.tmp");
    let j = serde_json::to_string_pretty(&ptr).context("serialize last-session pointer")?;
    fs::write(&tmp, j).with_context(|| format!("write {}", tmp.display()))?;
    fs::rename(&tmp, &path).with_context(|| format!("rename to {}", path.display()))?;
    Ok(())
}

fn read_last_session_pointer_in(sessions_root: &Path) -> anyhow::Result<Option<Uuid>> {
    let p = last_session_pointer_path_in(sessions_root);
    if !p.is_file() {
        return Ok(None);
    }
    let s = fs::read_to_string(&p).with_context(|| format!("read {}", p.display()))?;
    let ptr: LastSessionPointer =
        serde_json::from_str(&s).with_context(|| format!("parse {}", p.display()))?;
    Ok(Some(ptr.id))
}

/// 与 TUI resume 一致：路径在磁盘上则 canonical 后比较，否则规范化尾部 `/` 再比字符串。
pub(crate) fn workspace_paths_equal_for_session(a: &str, b: &str) -> bool {
    let pa = Path::new(a);
    let pb = Path::new(b);
    if let (Ok(ca), Ok(cb)) = (pa.canonicalize(), pb.canonicalize()) {
        return ca == cb;
    }
    let na = a.trim_end_matches(['/', '\\']);
    let nb = b.trim_end_matches(['/', '\\']);
    na == nb
}

#[derive(Debug, Clone)]
pub(crate) struct SessionIndexEntry {
    pub id: Uuid,
    pub workspace_root: String,
    pub agent: String,
    pub model: String,
    pub mtime: SystemTime,
}

fn parse_snapshot_header(path: &Path) -> Option<(Uuid, String, String, String)> {
    let name = path.file_name()?.to_str()?;
    let id_str = name.strip_suffix(".json")?;
    if id_str.starts_with('.') {
        return None;
    }
    let id = Uuid::parse_str(id_str).ok()?;
    let raw = fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let workspace_root = v
        .get("workspace_root")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let agent = v
        .get("agent")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let model = v
        .get("model")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    Some((id, workspace_root, agent, model))
}

/// 枚举 `tui-sessions` 下会话快照（排除 `.last-session.json`），带 mtime。
pub(crate) fn list_session_index_entries(
    sessions_root: &Path,
) -> anyhow::Result<Vec<SessionIndexEntry>> {
    if !sessions_root.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for ent in fs::read_dir(sessions_root)
        .with_context(|| format!("read_dir {}", sessions_root.display()))?
    {
        let ent = ent?;
        let path = ent.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };
        if name.starts_with('.') || name == LAST_SESSION_FILE || !name.ends_with(".json") {
            continue;
        }
        let Some((id, workspace_root, agent, model)) = parse_snapshot_header(&path) else {
            continue;
        };
        let mtime = ent
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        out.push(SessionIndexEntry {
            id,
            workspace_root,
            agent,
            model,
            mtime,
        });
    }
    Ok(out)
}

/// 当前目录优先（`workspace_root` 与 `cwd` 匹配且 mtime 最新），否则全局 `.last-session.json`，再否则全局 mtime 最新。
pub(crate) fn resolve_session_in_dir(sessions_root: &Path, cwd: &str) -> anyhow::Result<Uuid> {
    let mut entries = list_session_index_entries(sessions_root)?;
    if entries.is_empty() {
        anyhow::bail!("no_saved_sessions");
    }
    #[allow(clippy::unnecessary_sort_by)] // SystemTime is not Ord; Reverse key does not apply
    entries.sort_by(|a, b| b.mtime.cmp(&a.mtime));

    let cwd_matches: Vec<&SessionIndexEntry> = entries
        .iter()
        .filter(|e| workspace_paths_equal_for_session(cwd, &e.workspace_root))
        .collect();
    if let Some(best) = cwd_matches.first() {
        return Ok(best.id);
    }

    if let Ok(Some(id)) = read_last_session_pointer_in(sessions_root) {
        let p = sessions_root.join(format!("{id}.json"));
        if p.is_file() {
            return Ok(id);
        }
    }

    Ok(entries[0].id)
}

pub(crate) fn resolve_session_for_reopen(cwd: &str) -> anyhow::Result<Uuid> {
    resolve_session_in_dir(&sessions_dir(), cwd)
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
    if let Err(e) = write_last_session_pointer(snap.id) {
        tracing::warn!(target: "anycode_cli", "last-session pointer: {e:#}");
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    fn minimal_snapshot_json(id: Uuid, ws: &str) -> String {
        format!(
            r#"{{"version":1,"id":"{}","workspace_root":{},"agent":"general-purpose","model":"m","messages":[]}}"#,
            id,
            serde_json::to_string(ws).unwrap()
        )
    }

    #[test]
    fn resolve_prefers_matching_cwd_newest_mtime() {
        let dir = tempfile::tempdir().unwrap();
        let sd = dir.path().join("tui-sessions");
        fs::create_dir_all(&sd).unwrap();
        let ws = "/tmp/anycode-resolve-ws-test";
        let id_old = Uuid::new_v4();
        let id_new = Uuid::new_v4();
        fs::write(
            sd.join(format!("{id_old}.json")),
            minimal_snapshot_json(id_old, ws),
        )
        .unwrap();
        thread::sleep(Duration::from_millis(1200));
        fs::write(
            sd.join(format!("{id_new}.json")),
            minimal_snapshot_json(id_new, ws),
        )
        .unwrap();
        let got = resolve_session_in_dir(&sd, ws).unwrap();
        assert_eq!(got, id_new);
    }

    #[test]
    fn resolve_falls_back_to_last_pointer_when_no_cwd_match() {
        let dir = tempfile::tempdir().unwrap();
        let sd = dir.path().join("tui-sessions");
        fs::create_dir_all(&sd).unwrap();
        let id_a = Uuid::new_v4();
        fs::write(
            sd.join(format!("{id_a}.json")),
            minimal_snapshot_json(id_a, "/other/project"),
        )
        .unwrap();
        let ptr = LastSessionPointer {
            id: id_a,
            written_at: None,
        };
        fs::write(
            sd.join(".last-session.json"),
            serde_json::to_string(&ptr).unwrap(),
        )
        .unwrap();
        let got = resolve_session_in_dir(&sd, "/unrelated/cwd").unwrap();
        assert_eq!(got, id_a);
    }

    #[test]
    fn workspace_paths_equal_trims_slash() {
        assert!(workspace_paths_equal_for_session("/foo/bar", "/foo/bar/"));
    }
}
