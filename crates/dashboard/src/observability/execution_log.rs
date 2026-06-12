//! Read execution trace lines from `~/.anycode/tasks/{task_id}/output.log` on demand.

use crate::log_parser::parse_line;
use crate::schema::SessionDetail;
use anyhow::{Context, Result};
use serde::Serialize;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const DEFAULT_LIMIT: usize = 200;
const MAX_LIMIT: usize = 500;
/// When offset is 0 (live tail), read at most this many bytes from EOF instead of the whole file.
const TAIL_READ_CHUNK: u64 = 512 * 1024;

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

    let (all_lines, tail_only) = if offset == 0 {
        read_tail_line_strings(&path, limit)?
    } else {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("read execution log {}", path.display()))?;
        (content.lines().map(str::to_string).collect(), false)
    };
    let start = offset.min(all_lines.len());
    let end = if tail_only {
        all_lines.len()
    } else {
        (start + limit).min(all_lines.len())
    };
    let slice_start = if tail_only {
        all_lines.len().saturating_sub(limit)
    } else {
        start
    };
    let slice = &all_lines[slice_start..end];
    let has_more = if tail_only {
        false
    } else {
        end < all_lines.len()
    };

    let lines = slice
        .iter()
        .enumerate()
        .map(|(i, raw)| {
            let line_no = slice_start + i + 1;
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
        offset: slice_start,
        next_offset: end,
        has_more,
        lines,
    })
}

/// Read the last `limit` lines without loading the entire log (for live session polling).
fn read_tail_line_strings(path: &Path, limit: usize) -> Result<(Vec<String>, bool)> {
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("read execution log {}", path.display()))?;
    let len = file.metadata()?.len();
    if len == 0 {
        return Ok((Vec::new(), true));
    }
    let read_from = len.saturating_sub(TAIL_READ_CHUNK);
    file.seek(SeekFrom::Start(read_from))?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    let mut lines: Vec<String> = buf.lines().map(str::to_string).collect();
    if read_from > 0 && !lines.is_empty() {
        lines.remove(0);
    }
    let start = lines.len().saturating_sub(limit);
    Ok((lines[start..].to_vec(), true))
}

pub async fn read_execution_log_async(
    session: SessionDetail,
    offset: usize,
    limit: Option<usize>,
) -> Result<ExecutionLogResponse> {
    tokio::task::spawn_blocking(move || read_execution_log(&session, offset, limit))
        .await
        .context("execution log task cancelled")?
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
