//! Read execution trace lines from `~/.anycode/tasks/{task_id}/output.log` on demand.

use crate::log_parser::parse_line;
use crate::schema::SessionDetail;
use anyhow::{Context, Result};
use serde::Serialize;
use std::path::PathBuf;

const DEFAULT_LIMIT: usize = 200;
const MAX_LIMIT: usize = 500;

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionLogResponse {
    pub session_id: String,
    pub task_id: Option<String>,
    pub log_path: Option<String>,
    pub offset: usize,
    pub next_offset: usize,
    pub has_more: bool,
    pub lines: Vec<ExecutionLogLine>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionLogLine {
    pub line_no: usize,
    pub raw: String,
    pub event_type: Option<String>,
    pub severity: Option<String>,
    pub title: Option<String>,
    pub body: Option<String>,
    pub payload: serde_json::Value,
}

#[must_use]
pub fn tasks_root() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".anycode")
        .join("tasks")
}

#[must_use]
pub fn output_log_path(task_id: &str) -> PathBuf {
    tasks_root().join(task_id).join("output.log")
}

pub fn read_execution_log(
    session: &SessionDetail,
    offset: usize,
    limit: Option<usize>,
) -> Result<ExecutionLogResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let task_id = session.task_id.clone();
    let Some(ref tid) = task_id else {
        return Ok(ExecutionLogResponse {
            session_id: session.id.clone(),
            task_id: None,
            log_path: None,
            offset,
            next_offset: offset,
            has_more: false,
            lines: Vec::new(),
        });
    };

    let path = output_log_path(tid);
    if !path.is_file() {
        return Ok(ExecutionLogResponse {
            session_id: session.id.clone(),
            task_id: Some(tid.clone()),
            log_path: Some(path.to_string_lossy().to_string()),
            offset,
            next_offset: offset,
            has_more: false,
            lines: Vec::new(),
        });
    }

    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("read execution log {}", path.display()))?;
    let all_lines: Vec<&str> = content.lines().collect();
    let start = offset.min(all_lines.len());
    let end = (start + limit).min(all_lines.len());
    let slice = &all_lines[start..end];
    let has_more = end < all_lines.len();

    let lines = slice
        .iter()
        .enumerate()
        .map(|(i, raw)| {
            let line_no = start + i + 1;
            let parsed = parse_line(raw);
            ExecutionLogLine {
                line_no,
                raw: (*raw).to_string(),
                event_type: parsed.as_ref().map(|p| p.event_type.clone()),
                severity: parsed.as_ref().map(|p| p.severity.clone()),
                title: parsed.as_ref().map(|p| p.title.clone()),
                body: parsed
                    .as_ref()
                    .filter(|p| !p.body.is_empty())
                    .map(|p| p.body.clone()),
                payload: parsed
                    .as_ref()
                    .map(|p| p.payload.clone())
                    .unwrap_or_else(|| serde_json::Value::Object(Default::default())),
            }
        })
        .collect();

    Ok(ExecutionLogResponse {
        session_id: session.id.clone(),
        task_id: Some(tid.clone()),
        log_path: Some(path.to_string_lossy().to_string()),
        offset: start,
        next_offset: end,
        has_more,
        lines,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::SessionDetail;

    fn sample_session(task_id: &str) -> SessionDetail {
        SessionDetail {
            id: "sess_test".into(),
            project_id: "proj_test".into(),
            project_name: "demo".into(),
            kind: "run".into(),
            task_id: Some(task_id.into()),
            title: "demo".into(),
            prompt_preview: String::new(),
            status: "completed".into(),
            trusted_status: "unverified".into(),
            agent_type: String::new(),
            model: String::new(),
            started_at: String::new(),
            ended_at: None,
            summary: String::new(),
            metadata_json: "{}".into(),
            block_reason: None,
            block_kind: None,
        }
    }

    #[test]
    fn read_execution_log_missing_file_returns_empty() {
        let session = sample_session("00000000-0000-0000-0000-000000000099");
        let resp = read_execution_log(&session, 0, Some(10)).unwrap();
        assert!(resp.lines.is_empty());
        assert!(!resp.has_more);
    }
}
