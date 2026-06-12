//! File-based pending `AskUserQuestion` between live CLI and dashboard Web UI.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::UNIX_EPOCH;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionOptionRecord {
    pub label: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingQuestionRecord {
    pub question_id: String,
    pub session_id: String,
    pub question: String,
    pub header: String,
    pub options: Vec<QuestionOptionRecord>,
    pub multi_select: bool,
    pub created_at: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionResponseRecord {
    pub question_id: String,
    pub selected_labels: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub other_text: Option<String>,
    pub responded_at: String,
}

#[must_use]
pub fn web_questions_enabled() -> bool {
    !matches!(
        std::env::var("ANYCODE_DASHBOARD_WEB_QUESTION").as_deref(),
        Ok("0") | Ok("false") | Ok("off")
    )
}

#[must_use]
pub fn respond_allowed(host: &str) -> bool {
    if !web_questions_enabled() {
        return false;
    }
    if crate::service_governance::is_loopback_host(host) {
        return true;
    }
    std::env::var("ANYCODE_DASHBOARD_WEB_QUESTION_REMOTE")
        .ok()
        .is_some_and(|v| v == "1")
}

fn pending_dir() -> PathBuf {
    crate::cancel_ipc::dashboard_state_dir().join("questions/pending")
}

fn response_dir() -> PathBuf {
    crate::cancel_ipc::dashboard_state_dir().join("questions/responses")
}

pub fn register_pending(
    session_id: &str,
    question: &str,
    header: &str,
    options: &[QuestionOptionRecord],
    multi_select: bool,
) -> Result<String> {
    if options.is_empty() {
        bail!("AskUserQuestion requires at least one option");
    }
    std::fs::create_dir_all(pending_dir())?;
    let question_id = format!("q_{}", Uuid::new_v4().simple());
    let rec = PendingQuestionRecord {
        question_id: question_id.clone(),
        session_id: session_id.to_string(),
        question: question.to_string(),
        header: header.to_string(),
        options: options.to_vec(),
        multi_select,
        created_at: chrono::Utc::now().to_rfc3339(),
        status: "pending".into(),
    };
    let path = pending_dir().join(format!("{question_id}.json"));
    std::fs::write(&path, serde_json::to_string_pretty(&rec)?)?;
    Ok(question_id)
}

pub fn list_pending(limit: usize) -> Vec<PendingQuestionRecord> {
    list_pending_for_session(None, limit)
}

pub fn list_pending_for_session(
    session_id: Option<&str>,
    limit: usize,
) -> Vec<PendingQuestionRecord> {
    let _ = sweep_stale_pending(STALE_PENDING_MAX_AGE_SECS);
    let dir = pending_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return vec![];
    };
    let mut rows: Vec<(std::time::SystemTime, PendingQuestionRecord)> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| {
            let raw = std::fs::read_to_string(e.path()).ok()?;
            let rec: PendingQuestionRecord = serde_json::from_str(&raw).ok()?;
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

pub fn get_pending(question_id: &str) -> Option<PendingQuestionRecord> {
    let path = pending_dir().join(format!("{question_id}.json"));
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

pub fn submit_response(
    question_id: &str,
    selected_labels: &[String],
    other_text: Option<&str>,
) -> Result<()> {
    if get_pending(question_id).is_none() {
        bail!("question not found or already resolved");
    }
    if selected_labels.is_empty() && other_text.filter(|t| !t.trim().is_empty()).is_none() {
        bail!("at least one selection or other_text is required");
    }
    std::fs::create_dir_all(response_dir())?;
    let body = QuestionResponseRecord {
        question_id: question_id.to_string(),
        selected_labels: selected_labels.to_vec(),
        other_text: other_text.map(str::to_string),
        responded_at: chrono::Utc::now().to_rfc3339(),
    };
    let path = response_dir().join(format!("{question_id}.json"));
    std::fs::write(&path, serde_json::to_string_pretty(&body)?)?;
    Ok(())
}

#[must_use]
pub fn poll_response(question_id: &str) -> Option<QuestionResponseRecord> {
    let path = response_dir().join(format!("{question_id}.json"));
    let raw = std::fs::read_to_string(&path).ok()?;
    let rec: QuestionResponseRecord = serde_json::from_str(&raw).ok()?;
    let _ = std::fs::remove_file(&path);
    clear_pending(question_id);
    Some(rec)
}

pub fn clear_pending(question_id: &str) {
    let path = pending_dir().join(format!("{question_id}.json"));
    let _ = std::fs::remove_file(path);
}

pub const STALE_PENDING_MAX_AGE_SECS: u64 = 30 * 60;

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
            Ok(raw) => serde_json::from_str::<PendingQuestionRecord>(&raw)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util;
    use tempfile::tempdir;

    fn test_state(dir: &tempfile::TempDir) {
        std::env::set_var("ANYCODE_DASHBOARD_STATE_DIR", dir.path().join("dashboard"));
    }

    #[test]
    fn question_roundtrip() {
        let _guard = test_util::lock_state_dir_env();
        let dir = tempdir().unwrap();
        test_state(&dir);
        let opts = vec![QuestionOptionRecord {
            label: "A".into(),
            description: "first".into(),
        }];
        let id = register_pending("sess_1", "Pick one?", "Choice", &opts, false).unwrap();
        assert_eq!(list_pending(10).len(), 1);
        submit_response(&id, &["A".into()], None).unwrap();
        let resp = poll_response(&id).unwrap();
        assert_eq!(resp.selected_labels, vec!["A"]);
        assert!(list_pending(10).is_empty());
    }
}
