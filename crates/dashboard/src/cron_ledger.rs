//! Read `~/.anycode/logs/cron-runs.jsonl` and `orchestration.json` cron jobs (read-only).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronRunRecord {
    pub job_id: String,
    pub session_id: String,
    pub fired_at: String,
    pub status: String,
    pub detail: String,
    pub line_no: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJobRecord {
    pub id: String,
    pub schedule: String,
    pub command: String,
    pub session_id: Option<String>,
    pub failure_destination: Option<String>,
    pub tool_profile: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OrchestrationCronsOnly {
    #[serde(default)]
    crons: Vec<CronJobRecord>,
}

fn home_anycode_dir() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".anycode"))
}

#[must_use]
pub fn cron_runs_path() -> Option<PathBuf> {
    home_anycode_dir().map(|d| d.join("logs").join("cron-runs.jsonl"))
}

#[must_use]
pub fn orchestration_path() -> Option<PathBuf> {
    home_anycode_dir().map(|d| d.join("tasks").join("orchestration.json"))
}

pub fn read_cron_runs(
    limit: usize,
    job_id: Option<&str>,
    session_id: Option<&str>,
) -> Result<Vec<CronRunRecord>> {
    let Some(path) = cron_runs_path() else {
        return Ok(vec![]);
    };
    if !path.is_file() {
        return Ok(vec![]);
    }
    let text = std::fs::read_to_string(&path).with_context(|| path.display().to_string())?;
    let mut rows = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let raw: serde_json::Value =
            serde_json::from_str(line).unwrap_or_else(|_| serde_json::json!({ "raw": line }));
        let jid = raw
            .get("job_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let sid = raw
            .get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if let Some(filter) = job_id {
            if jid != filter {
                continue;
            }
        }
        if let Some(filter) = session_id {
            if sid != filter {
                continue;
            }
        }
        rows.push(CronRunRecord {
            job_id: jid,
            session_id: sid,
            fired_at: raw
                .get("fired_at")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            status: raw
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            detail: raw
                .get("detail")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            line_no: idx + 1,
        });
    }
    let keep = limit.max(1);
    if rows.len() > keep {
        rows = rows.split_off(rows.len() - keep);
    }
    Ok(rows)
}

pub fn read_cron_jobs(path: Option<&Path>) -> Result<Vec<CronJobRecord>> {
    let path = path
        .map(Path::to_path_buf)
        .or_else(orchestration_path)
        .context("orchestration path unavailable")?;
    if !path.is_file() {
        return Ok(vec![]);
    }
    let text = std::fs::read_to_string(&path).with_context(|| path.display().to_string())?;
    let snap: OrchestrationCronsOnly = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("invalid orchestration JSON: {e}"))?;
    Ok(snap.crons)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cron_run_line() {
        let dir = tempfile::tempdir().unwrap();
        let log = dir
            .path()
            .join(".anycode")
            .join("logs")
            .join("cron-runs.jsonl");
        std::fs::create_dir_all(log.parent().unwrap()).unwrap();
        std::fs::write(
            &log,
            r#"{"job_id":"j1","session_id":"sess-a","fired_at":"2026-05-21T00:00:00Z","status":"ok","detail":"done"}"#,
        )
        .unwrap();
        std::env::set_var("HOME", dir.path());
        let rows = read_cron_runs(10, None, None).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].job_id, "j1");
        assert_eq!(rows[0].status, "ok");
    }
}
