//! File-based pending tool approvals between live CLI and dashboard Web UI.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant, UNIX_EPOCH};
use uuid::Uuid;

pub const SESSION_ENV: &str = "ANYCODE_DASHBOARD_SESSION_ID";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingApprovalRecord {
    pub approval_id: String,
    pub session_id: String,
    pub tool: String,
    pub input_preview: String,
    pub created_at: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalResponseRecord {
    pub approval_id: String,
    pub decision: String,
    pub source: String,
    pub responded_at: String,
}

#[must_use]
pub fn web_approvals_enabled() -> bool {
    !matches!(
        std::env::var("ANYCODE_DASHBOARD_WEB_APPROVAL").as_deref(),
        Ok("0") | Ok("false") | Ok("off")
    )
}

#[must_use]
pub fn respond_allowed(host: &str) -> bool {
    if !web_approvals_enabled() {
        return false;
    }
    if crate::service_governance::is_loopback_host(host) {
        return true;
    }
    std::env::var("ANYCODE_DASHBOARD_WEB_APPROVAL_REMOTE")
        .ok()
        .is_some_and(|v| v == "1")
}

fn pending_dir() -> PathBuf {
    crate::cancel_ipc::dashboard_state_dir().join("approvals/pending")
}

fn response_dir() -> PathBuf {
    crate::cancel_ipc::dashboard_state_dir().join("approvals/responses")
}

fn auto_approve_dir() -> PathBuf {
    crate::cancel_ipc::dashboard_state_dir().join("approvals/auto")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionAutoApproveRecord {
    pub session_id: String,
    pub enabled: bool,
    pub set_at: String,
}

/// Enable / disable session-level approval delegation ("托管模式"): while
/// enabled, the live CLI auto-allows tool approvals for this session (except
/// high-risk commands, which still require explicit confirmation).
pub fn set_session_auto_approve(session_id: &str, enabled: bool) -> Result<()> {
    let dir = auto_approve_dir();
    let path = dir.join(format!("{session_id}.json"));
    if !enabled {
        let _ = std::fs::remove_file(&path);
        return Ok(());
    }
    std::fs::create_dir_all(&dir)?;
    let rec = SessionAutoApproveRecord {
        session_id: session_id.to_string(),
        enabled: true,
        set_at: chrono::Utc::now().to_rfc3339(),
    };
    std::fs::write(&path, serde_json::to_string_pretty(&rec)?)?;
    Ok(())
}

#[must_use]
pub fn session_auto_approve_enabled(session_id: &str) -> bool {
    let path = auto_approve_dir().join(format!("{session_id}.json"));
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    serde_json::from_str::<SessionAutoApproveRecord>(&raw)
        .map(|r| r.enabled)
        .unwrap_or(false)
}

/// High-risk inputs keep explicit confirmation even under session delegation.
#[must_use]
pub fn input_is_high_risk(tool: &str, input_preview: &str) -> bool {
    if tool != "Bash" {
        return false;
    }
    let lower = input_preview.to_lowercase();
    const PATTERNS: [&str; 8] = [
        "rm -rf",
        "rm -fr",
        "sudo ",
        "mkfs",
        "dd if=",
        "git push --force",
        "shutdown",
        ":(){",
    ];
    PATTERNS.iter().any(|p| lower.contains(p))
}

pub fn register_pending(session_id: &str, tool: &str, input_preview: &str) -> Result<String> {
    std::fs::create_dir_all(pending_dir())?;
    let approval_id = format!("apr_{}", Uuid::new_v4().simple());
    let rec = PendingApprovalRecord {
        approval_id: approval_id.clone(),
        session_id: session_id.to_string(),
        tool: tool.to_string(),
        input_preview: truncate_preview(input_preview, 4000),
        created_at: chrono::Utc::now().to_rfc3339(),
        status: "pending".into(),
    };
    let path = pending_dir().join(format!("{approval_id}.json"));
    std::fs::write(&path, serde_json::to_string_pretty(&rec)?)?;
    Ok(approval_id)
}

pub fn list_pending(limit: usize) -> Vec<PendingApprovalRecord> {
    list_pending_for_session(None, limit)
}

pub fn list_pending_for_session(
    session_id: Option<&str>,
    limit: usize,
) -> Vec<PendingApprovalRecord> {
    let _ = sweep_stale_pending(STALE_PENDING_MAX_AGE_SECS);
    let dir = pending_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return vec![];
    };
    let mut rows: Vec<(std::time::SystemTime, PendingApprovalRecord)> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| {
            let raw = std::fs::read_to_string(e.path()).ok()?;
            let rec: PendingApprovalRecord = serde_json::from_str(&raw).ok()?;
            if rec.status != "pending" {
                return None;
            }
            if let Some(sid) = session_id {
                if rec.session_id != sid {
                    return None;
                }
            }
            let mtime = e.metadata().ok()?.modified().ok()?;
            Some((mtime, rec))
        })
        .collect();
    rows.sort_by(|a, b| b.0.cmp(&a.0));
    rows.into_iter().take(limit).map(|(_, r)| r).collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingApprovalSessionCount {
    pub session_id: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingApprovalSummary {
    pub pending_total: usize,
    pub by_session: Vec<PendingApprovalSessionCount>,
}

static PENDING_SUMMARY_CACHE: Mutex<Option<(Instant, PendingApprovalSummary)>> = Mutex::new(None);
const PENDING_SUMMARY_TTL: Duration = Duration::from_secs(2);

pub fn invalidate_pending_summary_cache() {
    if let Ok(mut guard) = PENDING_SUMMARY_CACHE.lock() {
        *guard = None;
    }
}

#[must_use]
pub fn pending_summary() -> PendingApprovalSummary {
    let now = Instant::now();
    if let Ok(guard) = PENDING_SUMMARY_CACHE.lock() {
        if let Some((at, summary)) = guard.as_ref() {
            if now.duration_since(*at) < PENDING_SUMMARY_TTL {
                return summary.clone();
            }
        }
    }
    let summary = build_pending_summary();
    if let Ok(mut guard) = PENDING_SUMMARY_CACHE.lock() {
        *guard = Some((now, summary.clone()));
    }
    summary
}

fn build_pending_summary() -> PendingApprovalSummary {
    let rows = list_pending(500);
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for row in &rows {
        *counts.entry(row.session_id.clone()).or_default() += 1;
    }
    let mut by_session: Vec<PendingApprovalSessionCount> = counts
        .into_iter()
        .map(|(session_id, count)| PendingApprovalSessionCount { session_id, count })
        .collect();
    by_session.sort_by(|a, b| b.count.cmp(&a.count));
    PendingApprovalSummary {
        pending_total: rows.len(),
        by_session,
    }
}

pub fn get_pending(approval_id: &str) -> Option<PendingApprovalRecord> {
    let path = pending_dir().join(format!("{approval_id}.json"));
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

pub fn submit_response(approval_id: &str, decision: &str) -> Result<()> {
    match decision {
        "allow_once" | "deny" | "allow_tool" => {}
        _ => bail!("decision must be allow_once, deny, or allow_tool"),
    }
    if get_pending(approval_id).is_none() {
        bail!("approval not found or already resolved");
    }
    std::fs::create_dir_all(response_dir())?;
    let body = ApprovalResponseRecord {
        approval_id: approval_id.to_string(),
        decision: decision.to_string(),
        source: "dashboard".into(),
        responded_at: chrono::Utc::now().to_rfc3339(),
    };
    let path = response_dir().join(format!("{approval_id}.json"));
    std::fs::write(&path, serde_json::to_string_pretty(&body)?)?;
    clear_pending(approval_id);
    invalidate_pending_summary_cache();
    Ok(())
}

#[must_use]
pub fn poll_response(approval_id: &str) -> Option<String> {
    let path = response_dir().join(format!("{approval_id}.json"));
    let raw = std::fs::read_to_string(&path).ok()?;
    let rec: ApprovalResponseRecord = serde_json::from_str(&raw).ok()?;
    let _ = std::fs::remove_file(&path);
    clear_pending(approval_id);
    Some(rec.decision)
}

pub fn clear_pending(approval_id: &str) {
    let path = pending_dir().join(format!("{approval_id}.json"));
    let _ = std::fs::remove_file(path);
}

/// Default max age for orphan pending approval files (CLI crash / timeout).
pub const STALE_PENDING_MAX_AGE_SECS: u64 = 30 * 60;

/// Remove pending approval files older than `max_age_secs` or with invalid JSON.
#[must_use]
pub fn sweep_stale_pending(max_age_secs: u64) -> usize {
    let dir = pending_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return 0;
    };
    let cutoff = std::time::SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(max_age_secs))
        .unwrap_or(UNIX_EPOCH);
    let mut removed = 0usize;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|x| x != "json") {
            continue;
        }
        let stale = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .map(|mtime| mtime < cutoff)
            .unwrap_or(true);
        let invalid = match std::fs::read_to_string(&path) {
            Ok(raw) => serde_json::from_str::<PendingApprovalRecord>(&raw)
                .map(|rec| rec.status != "pending")
                .unwrap_or(true),
            Err(_) => true,
        };
        if stale || invalid {
            if std::fs::remove_file(&path).is_ok() {
                removed += 1;
            }
        }
    }
    removed
}

fn truncate_preview(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    s.chars().take(max).collect::<String>() + "…"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util;
    use tempfile::tempdir;

    fn test_state(dir: &tempfile::TempDir) {
        std::env::set_var("ANYCODE_DASHBOARD_STATE_DIR", dir.path().join("dashboard"));
    }

    #[test]
    fn approval_roundtrip() {
        let _guard = test_util::lock_state_dir_env();
        let dir = tempdir().unwrap();
        test_state(&dir);
        let id = register_pending("sess_1", "Bash", "{ \"command\": \"rm -rf /\" }").unwrap();
        assert_eq!(list_pending(10).len(), 1);
        submit_response(&id, "deny").unwrap();
        assert!(list_pending(10).is_empty(), "pending cleared on submit");
        assert!(poll_response(&id).as_deref() == Some("deny"));
        assert!(list_pending(10).is_empty());
    }

    #[test]
    fn pending_summary_groups_by_session() {
        let _guard = test_util::lock_state_dir_env();
        let dir = tempdir().unwrap();
        test_state(&dir);
        let _ = register_pending("sess_a", "Bash", "{}").unwrap();
        let _ = register_pending("sess_a", "Edit", "{}").unwrap();
        let _ = register_pending("sess_b", "Bash", "{}").unwrap();
        let summary = pending_summary();
        assert_eq!(summary.pending_total, 3);
        assert_eq!(summary.by_session.len(), 2);
        assert_eq!(summary.by_session[0].session_id, "sess_a");
        assert_eq!(summary.by_session[0].count, 2);
    }

    #[test]
    fn session_auto_approve_roundtrip() {
        let _guard = test_util::lock_state_dir_env();
        let dir = tempdir().unwrap();
        test_state(&dir);
        assert!(!session_auto_approve_enabled("sess_x"));
        set_session_auto_approve("sess_x", true).unwrap();
        assert!(session_auto_approve_enabled("sess_x"));
        set_session_auto_approve("sess_x", false).unwrap();
        assert!(!session_auto_approve_enabled("sess_x"));
    }

    #[test]
    fn high_risk_inputs_detected() {
        assert!(input_is_high_risk(
            "Bash",
            "{\"command\": \"rm -rf /tmp/x\"}"
        ));
        assert!(input_is_high_risk("Bash", "sudo reboot"));
        assert!(!input_is_high_risk("Bash", "ls -la"));
        assert!(!input_is_high_risk("Edit", "rm -rf mention in text"));
    }

    #[test]
    fn list_pending_filters_session() {
        let _guard = test_util::lock_state_dir_env();
        let dir = tempdir().unwrap();
        test_state(&dir);
        let _ = register_pending("sess_a", "Bash", "{}").unwrap();
        let _ = register_pending("sess_b", "Edit", "{}").unwrap();
        assert_eq!(list_pending_for_session(Some("sess_a"), 10).len(), 1);
    }

    #[test]
    fn sweep_stale_pending_removes_invalid_json() {
        let _guard = test_util::lock_state_dir_env();
        let dir = tempdir().unwrap();
        test_state(&dir);
        std::fs::create_dir_all(pending_dir()).unwrap();
        std::fs::write(pending_dir().join("apr_bad.json"), "{not json").unwrap();
        assert_eq!(sweep_stale_pending(STALE_PENDING_MAX_AGE_SECS), 1);
    }
}
