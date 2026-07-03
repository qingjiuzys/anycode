//! File-based cancel signals between dashboard API and live CLI runs.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveSessionRecord {
    pub session_id: String,
    pub task_id: String,
    pub pid: u32,
    pub started_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelRequestRecord {
    pub session_id: String,
    pub requested_at: String,
    pub source: String,
}

#[must_use]
pub fn dashboard_state_dir() -> PathBuf {
    if let Ok(p) = std::env::var("ANYCODE_DASHBOARD_STATE_DIR") {
        return PathBuf::from(p);
    }
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".anycode")
        .join("dashboard")
}

fn active_dir() -> PathBuf {
    dashboard_state_dir().join("active")
}

fn cancel_dir() -> PathBuf {
    dashboard_state_dir().join("cancel")
}

pub fn register_active(session_id: &str, task_id: &str) -> Result<()> {
    std::fs::create_dir_all(active_dir())?;
    let rec = ActiveSessionRecord {
        session_id: session_id.to_string(),
        task_id: task_id.to_string(),
        pid: std::process::id(),
        started_at: chrono::Utc::now().to_rfc3339(),
    };
    let path = active_dir().join(format!("{session_id}.json"));
    std::fs::write(&path, serde_json::to_string_pretty(&rec)?)?;
    Ok(())
}

pub fn unregister_active(session_id: &str) {
    let path = active_dir().join(format!("{session_id}.json"));
    let _ = std::fs::remove_file(path);
    consume_cancel(session_id);
}

/// Write a cancel request when the session has a live CLI registration.
pub fn request_cancel(session_id: &str) -> Result<bool> {
    let active = active_dir().join(format!("{session_id}.json"));
    if !active.exists() {
        return Ok(false);
    }
    if get_active(session_id)
        .as_ref()
        .is_some_and(|rec| !process_is_alive(rec.pid))
    {
        unregister_active(session_id);
        return Ok(false);
    }
    std::fs::create_dir_all(cancel_dir())?;
    let path = cancel_dir().join(format!("{session_id}.json"));
    let body = CancelRequestRecord {
        session_id: session_id.to_string(),
        requested_at: chrono::Utc::now().to_rfc3339(),
        source: "dashboard".into(),
    };
    std::fs::write(&path, serde_json::to_string_pretty(&body)?)?;
    Ok(true)
}

#[must_use]
pub fn poll_cancel_requested(session_id: &str) -> bool {
    cancel_dir().join(format!("{session_id}.json")).exists()
}

pub fn consume_cancel(session_id: &str) {
    let path = cancel_dir().join(format!("{session_id}.json"));
    let _ = std::fs::remove_file(path);
}

#[must_use]
pub fn is_active(session_id: &str) -> bool {
    active_dir().join(format!("{session_id}.json")).exists()
}

#[must_use]
pub fn get_active(session_id: &str) -> Option<ActiveSessionRecord> {
    let path = active_dir().join(format!("{session_id}.json"));
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Remove active-session registrations whose process is no longer alive or whose JSON is invalid.
#[must_use]
pub fn sweep_stale_active() -> usize {
    let dir = active_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return 0;
    };
    let mut removed = 0usize;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|x| x != "json") {
            continue;
        }
        let stale = std::fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str::<ActiveSessionRecord>(&raw).ok())
            .is_none_or(|rec| !process_is_alive(rec.pid));
        if stale && std::fs::remove_file(&path).is_ok() {
            removed += 1;
        }
    }
    removed
}

fn process_is_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    if pid == std::process::id() {
        return true;
    }
    #[cfg(unix)]
    {
        Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn cancel_roundtrip() {
        let _guard = crate::test_util::lock_state_dir_env();
        let dir = tempdir().unwrap();
        std::env::set_var("ANYCODE_DASHBOARD_STATE_DIR", dir.path().join("dashboard"));
        register_active("sess_a", "task_1").unwrap();
        assert!(is_active("sess_a"));
        assert!(!poll_cancel_requested("sess_a"));
        assert!(request_cancel("sess_a").unwrap());
        assert!(poll_cancel_requested("sess_a"));
        consume_cancel("sess_a");
        assert!(!poll_cancel_requested("sess_a"));
        unregister_active("sess_a");
        assert!(!is_active("sess_a"));
        assert!(!request_cancel("sess_a").unwrap());
    }

    #[test]
    fn sweep_stale_active_removes_invalid_json() {
        let _guard = crate::test_util::lock_state_dir_env();
        let dir = tempdir().unwrap();
        std::env::set_var("ANYCODE_DASHBOARD_STATE_DIR", dir.path().join("dashboard"));
        std::fs::create_dir_all(active_dir()).unwrap();
        std::fs::write(active_dir().join("bad.json"), "{not json").unwrap();
        assert_eq!(sweep_stale_active(), 1);
    }
}
