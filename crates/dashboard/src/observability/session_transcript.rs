//! Normalized session transcript blocks from index events + execution trace.

use super::execution_log::{output_log_path, read_execution_log};
use super::session_trace::session_trace;
use super::transcript_cache::{event_fingerprint, get_cached, invalidate_session, put_cached};
use crate::db::DashboardDb;
use crate::log_parser::parse_prose_sections;
use crate::schema::{ProjectEvent, SessionDetail, SessionTranscriptResponse, TranscriptBlock};
use anyhow::{Context, Result};
use serde_json::json;
use std::collections::HashSet;

const SCHEMA_VERSION: u32 = 1;

const LIFECYCLE_TYPES: &[&str] = &[
    "task_start",
    "task_end",
    "turn_start",
    "turn_end",
    "llm_request_start",
    "llm_response_end",
    "project_root_created",
];

pub async fn session_transcript(
    db: &DashboardDb,
    session_id: &str,
) -> Result<SessionTranscriptResponse> {
    let session = db
        .get_session(session_id)
        .await?
        .context("session not found")?;

    let mut index_events = db
        .list_session_events(session_id, None, 500, None, None, None)
        .await?;
    index_events.sort_by(|a, b| a.occurred_at.cmp(&b.occurred_at));

    let is_running = session.status == "running";
    let (event_count, last_event_id) = event_fingerprint(&index_events);
    if let Some(cached) = get_cached(session_id, event_count, &last_event_id, is_running) {
        return Ok(cached);
    }

    let transcript = if should_use_index_only(&session, &index_events) {
        assemble_transcript(&session, session_id, index_events, Vec::new())?
    } else {
        let task_ids = collect_task_ids(&session, &index_events);
        let session_for_logs = session.clone();
        let mut log_events = tokio::task::spawn_blocking(move || {
            parsed_log_events_for_tasks(&session_for_logs, &task_ids)
        })
        .await
        .context("transcript log parse cancelled")?;
        if log_events.is_empty() {
            if let Ok(trace) = session_trace(db, session_id).await {
                for row in trace.events {
                    log_events.push(ProjectEvent {
                        id: format!("trace:{}", row.event_type),
                        project_id: session.project_id.clone(),
                        session_id: Some(session_id.to_string()),
                        task_id: session.task_id.clone(),
                        agent_id: None,
                        event_type: row.event_type,
                        severity: row.severity,
                        title: row.title,
                        body: row.body,
                        payload: row.payload,
                        occurred_at: if row.occurred_at.is_empty() {
                            session.started_at.clone()
                        } else {
                            row.occurred_at
                        },
                    });
                }
            }
        }
        assemble_transcript(&session, session_id, index_events, log_events)?
    };

    put_cached(
        session_id,
        transcript.clone(),
        event_count,
        last_event_id,
        is_running,
    );
    Ok(transcript)
}

fn should_use_index_only(session: &SessionDetail, index_events: &[ProjectEvent]) -> bool {
    if session.status == "running" {
        return false;
    }
    index_has_conversation(index_events)
}

fn index_has_conversation(events: &[ProjectEvent]) -> bool {
    let mut has_user = false;
    let mut has_assistant = false;
    for event in events {
        match event.event_type.as_str() {
            "user_prompt" | "prompt" => has_user = true,
            "assistant_response" => has_assistant = true,
            _ => {}
        }
        if has_user && has_assistant {
            return true;
        }
    }
    false
}

fn assemble_transcript(
    session: &SessionDetail,
    session_id: &str,
    index_events: Vec<ProjectEvent>,
    log_events: Vec<ProjectEvent>,
) -> Result<SessionTranscriptResponse> {
    let mut lifecycle = Vec::new();
    let mut blocks = Vec::new();

    let mut seen = HashSet::new();
    for event in index_events {
        seen.insert(event_key(&event));
        if LIFECYCLE_TYPES.contains(&event.event_type.as_str()) {
            lifecycle.push(event_to_block(&event, true));
            continue;
        }
        if let Some(block) = event_to_visible_block(&event) {
            blocks.push(block);
        }
    }

    for event in log_events {
        if seen.contains(&event_key(&event)) {
            continue;
        }
        if LIFECYCLE_TYPES.contains(&event.event_type.as_str()) {
            lifecycle.push(event_to_block(&event, true));
            continue;
        }
        if let Some(block) = event_to_visible_block(&event) {
            blocks.push(block);
        }
    }

    blocks.sort_by(|a, b| a.at.cmp(&b.at));
    finalize_conversation_timeline(&mut blocks, &lifecycle, session);

    Ok(SessionTranscriptResponse {
        schema_version: SCHEMA_VERSION,
        session_id: session_id.to_string(),
        blocks,
        lifecycle,
    })
}

/// Invalidate cached transcript after index events change.
pub fn invalidate_transcript_cache(session_id: &str) {
    invalidate_session(session_id);
}

fn collect_task_ids(
    session: &crate::schema::SessionDetail,
    events: &[ProjectEvent],
) -> Vec<String> {
    let mut ids = HashSet::new();
    if let Some(ref tid) = session.task_id {
        if !tid.is_empty() {
            ids.insert(tid.clone());
        }
    }
    for event in events {
        if let Some(ref tid) = event.task_id {
            if !tid.is_empty() {
                ids.insert(tid.clone());
            }
        }
    }
    let mut out: Vec<String> = ids.into_iter().collect();
    out.sort();
    out
}

fn parsed_log_events_for_tasks(
    session: &crate::schema::SessionDetail,
    task_ids: &[String],
) -> Vec<ProjectEvent> {
    if task_ids.is_empty() {
        return parsed_log_events(session);
    }
    let mut out = Vec::new();
    for task_id in task_ids {
        let mut scoped = session.clone();
        scoped.task_id = Some(task_id.clone());
        out.extend(parsed_log_events(&scoped));
        out.extend(prose_events_from_task_log(session, task_id));
    }
    out.sort_by(|a, b| a.occurred_at.cmp(&b.occurred_at));
    out
}

fn prose_events_from_task_log(
    session: &crate::schema::SessionDetail,
    task_id: &str,
) -> Vec<ProjectEvent> {
    let path = output_log_path(task_id);
    let Ok(content) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    parse_prose_sections(&content)
        .into_iter()
        .enumerate()
        .map(|(idx, (line_no, body))| {
            let occurred_at = chrono::DateTime::parse_from_rfc3339(&session.started_at)
                .map(|dt| (dt + chrono::Duration::milliseconds(line_no as i64)).to_rfc3339())
                .unwrap_or_else(|_| session.started_at.clone());
            ProjectEvent {
                id: format!("prose:{task_id}:{line_no}:{idx}"),
                project_id: session.project_id.clone(),
                session_id: Some(session.id.clone()),
                task_id: Some(task_id.to_string()),
                agent_id: None,
                event_type: "assistant_response".into(),
                severity: "info".into(),
                title: "Assistant".into(),
                body,
                payload: json!({ "line_no": line_no, "source": "prose_section" }),
                occurred_at,
            }
        })
        .collect()
}

fn parsed_log_events(session: &crate::schema::SessionDetail) -> Vec<ProjectEvent> {
    let Ok(log) = read_execution_log(session, 0, Some(500)) else {
        return Vec::new();
    };
    log.lines
        .into_iter()
        .filter_map(|line| {
            let event_type = line.event_type?;
            let body = line.body.unwrap_or_default();
            let occurred_at = chrono::DateTime::parse_from_rfc3339(&session.started_at)
                .map(|dt| (dt + chrono::Duration::milliseconds(line.line_no as i64)).to_rfc3339())
                .unwrap_or_else(|_| session.started_at.clone());
            Some(ProjectEvent {
                id: format!("log:{}", line.line_no),
                project_id: session.project_id.clone(),
                session_id: Some(session.id.clone()),
                task_id: session.task_id.clone(),
                agent_id: None,
                event_type,
                severity: line.severity.unwrap_or_else(|| "info".into()),
                title: line.title.unwrap_or_default(),
                body,
                payload: merge_line_no(line.payload, line.line_no),
                occurred_at,
            })
        })
        .collect()
}

fn merge_line_no(mut payload: serde_json::Value, line_no: usize) -> serde_json::Value {
    match &mut payload {
        serde_json::Value::Object(map) => {
            map.insert("line_no".into(), json!(line_no));
            payload
        }
        _ => json!({ "line_no": line_no }),
    }
}

fn event_key(event: &ProjectEvent) -> String {
    format!(
        "{}:{}:{}",
        event.event_type,
        event.title.trim(),
        event.body.trim()
    )
}

fn event_to_visible_block(event: &ProjectEvent) -> Option<TranscriptBlock> {
    if let Some(block) = index_event_to_block(event) {
        return Some(block);
    }
    trace_event_to_block(event)
}

fn index_event_to_block(event: &ProjectEvent) -> Option<TranscriptBlock> {
    match event.event_type.as_str() {
        "user_prompt" | "prompt" => Some(TranscriptBlock {
            id: event.id.clone(),
            block_type: "user_message".into(),
            at: event.occurred_at.clone(),
            title: "You".into(),
            body: pick_body(event),
            meta: event.payload.clone(),
            collapsible: false,
            default_collapsed: false,
            event_id: Some(event.id.clone()),
        }),
        "assistant_response" => {
            let body = pick_body(event);
            let (collapsible, default_collapsed) = collapse_policy_for_text(&body);
            Some(TranscriptBlock {
                id: event.id.clone(),
                block_type: "assistant_message".into(),
                at: event.occurred_at.clone(),
                title: "Assistant".into(),
                body,
                meta: event.payload.clone(),
                collapsible,
                default_collapsed,
                event_id: Some(event.id.clone()),
            })
        }
        "tool_denied" => Some(TranscriptBlock {
            id: event.id.clone(),
            block_type: "session_error".into(),
            at: event.occurred_at.clone(),
            title: "Tool denied".into(),
            body: error_event_body(event),
            meta: event.payload.clone(),
            collapsible: false,
            default_collapsed: false,
            event_id: Some(event.id.clone()),
        }),
        "tool_approval_pending" => Some(TranscriptBlock {
            id: event.id.clone(),
            block_type: "approval_request".into(),
            at: event.occurred_at.clone(),
            title: event.title.clone(),
            body: pick_body(event),
            meta: event.payload.clone(),
            collapsible: true,
            default_collapsed: false,
            event_id: Some(event.id.clone()),
        }),
        "tool_approval_resolved" => Some(TranscriptBlock {
            id: event.id.clone(),
            block_type: "system_notice".into(),
            at: event.occurred_at.clone(),
            title: event.title.clone(),
            body: pick_body(event),
            meta: event.payload.clone(),
            collapsible: true,
            default_collapsed: true,
            event_id: Some(event.id.clone()),
        }),
        "budget_warning" | "budget_degrade" | "budget_exceeded" | "gate" => Some(TranscriptBlock {
            id: event.id.clone(),
            block_type: "system_notice".into(),
            at: event.occurred_at.clone(),
            title: event.title.clone(),
            body: pick_body(event),
            meta: event.payload.clone(),
            collapsible: true,
            default_collapsed: true,
            event_id: Some(event.id.clone()),
        }),
        "project_root_created" => Some(event_to_block(event, true)),
        "task_end" if event.severity == "error" || event.title.to_lowercase().contains("fail") => {
            Some(TranscriptBlock {
                id: event.id.clone(),
                block_type: "session_error".into(),
                at: event.occurred_at.clone(),
                title: "Task failed".into(),
                body: error_event_body(event),
                meta: event.payload.clone(),
                collapsible: false,
                default_collapsed: false,
                event_id: Some(event.id.clone()),
            })
        }
        _ if event.severity == "error" => Some(TranscriptBlock {
            id: event.id.clone(),
            block_type: "session_error".into(),
            at: event.occurred_at.clone(),
            title: event.title.clone(),
            body: error_event_body(event),
            meta: event.payload.clone(),
            collapsible: false,
            default_collapsed: false,
            event_id: Some(event.id.clone()),
        }),
        _ => None,
    }
}

fn trace_event_to_block(event: &ProjectEvent) -> Option<TranscriptBlock> {
    match event.event_type.as_str() {
        "tool_call_start" | "tool_call_input" => Some(TranscriptBlock {
            id: event.id.clone(),
            block_type: "tool_call".into(),
            at: event.occurred_at.clone(),
            title: tool_title(event),
            body: tool_body(event),
            meta: json!({
                "tool_name": tool_title(event),
                "phase": "start",
            }),
            collapsible: true,
            default_collapsed: true,
            event_id: Some(event.id.clone()),
        }),
        "tool_call_end" => Some(TranscriptBlock {
            id: event.id.clone(),
            block_type: "tool_result".into(),
            at: event.occurred_at.clone(),
            title: tool_title(event),
            body: truncate(&tool_body(event), 4000),
            meta: json!({ "phase": "end" }),
            collapsible: true,
            default_collapsed: true,
            event_id: Some(event.id.clone()),
        }),
        _ => None,
    }
}

fn tool_title(event: &ProjectEvent) -> String {
    event
        .payload
        .get("name")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| event.title.clone())
}

fn tool_body(event: &ProjectEvent) -> String {
    let body = pick_body(event);
    if !body.is_empty() && body != event.title {
        return body;
    }
    let mut parts = Vec::new();
    for key in ["command", "path", "query", "duration_ms", "error"] {
        if let Some(value) = event.payload.get(key).and_then(|v| v.as_str()) {
            if !value.is_empty() && value != "<none>" {
                parts.push(format!("{key}: {value}"));
            }
        }
    }
    if parts.is_empty() {
        humanize_error_text(&event.title)
    } else {
        parts.join("\n")
    }
}

fn event_to_block(event: &ProjectEvent, collapsed: bool) -> TranscriptBlock {
    TranscriptBlock {
        id: event.id.clone(),
        block_type: "system_notice".into(),
        at: event.occurred_at.clone(),
        title: humanize_event_type(&event.event_type),
        body: pick_body(event),
        meta: json!({ "event_type": event.event_type }),
        collapsible: true,
        default_collapsed: collapsed,
        event_id: Some(event.id.clone()),
    }
}

fn collapse_policy_for_text(body: &str) -> (bool, bool) {
    let trimmed = body.trim();
    let lines = trimmed.lines().count();
    let chars = trimmed.chars().count();
    let long = lines > 8 || chars > 480;
    (long, long)
}

fn is_bare_schema_token(s: &str) -> bool {
    matches!(
        s.trim().to_ascii_lowercase().as_str(),
        "path" | "command" | "query" | "duration_ms"
    )
}

fn humanize_bare_schema_token(s: &str) -> String {
    format!("Tool parameter error: missing or invalid `{s}`")
}

fn humanize_error_text(text: &str) -> String {
    let t = text.trim();
    if t.is_empty() {
        return String::new();
    }
    if is_bare_schema_token(t) {
        humanize_bare_schema_token(t)
    } else {
        t.to_string()
    }
}

fn error_event_body(event: &ProjectEvent) -> String {
    for key in ["error", "message"] {
        if let Some(v) = event.payload.get(key).and_then(|v| v.as_str()) {
            let t = v.trim();
            if !t.is_empty() && t != "<none>" {
                return t.to_string();
            }
        }
    }
    humanize_error_text(&pick_body(event))
}

fn pick_body(event: &ProjectEvent) -> String {
    let body = event.body.trim();
    if !body.is_empty() {
        return body.to_string();
    }
    event.title.trim().to_string()
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    format!("{}…", s.chars().take(max).collect::<String>())
}

fn humanize_event_type(event_type: &str) -> String {
    event_type.replace('_', " ")
}

fn is_reply_block_type(block_type: &str) -> bool {
    matches!(
        block_type,
        "assistant_message" | "session_error" | "tool_call" | "tool_result"
    )
}

fn is_failure_lifecycle(block: &TranscriptBlock) -> bool {
    block
        .meta
        .get("event_type")
        .and_then(|v| v.as_str())
        .is_some_and(|t| t == "task_end")
        && (block.title.to_lowercase().contains("fail")
            || block.title.to_lowercase().contains("cancel")
            || block.body.to_lowercase().contains("status=failed")
            || block.body.to_lowercase().contains("reason="))
}

fn looks_like_error_text(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("llm error")
        || lower.contains("api error")
        || lower.contains("status=400")
        || lower.contains("status=403")
        || lower.contains("status=500")
        || lower.contains("failed_precondition")
        || lower.contains("task failed")
}

fn finalize_conversation_timeline(
    blocks: &mut Vec<TranscriptBlock>,
    lifecycle: &[TranscriptBlock],
    session: &crate::schema::SessionDetail,
) {
    promote_failed_tasks(blocks, lifecycle);
    fill_missing_turn_replies(blocks, lifecycle, session);
    blocks.sort_by(|a, b| a.at.cmp(&b.at));
    uncollapse_final_assistant_replies(blocks);
}

/// The last assistant reply of each user turn is the de-facto summary for that
/// turn — keep it expanded regardless of length. Intermediate long replies
/// keep the regular collapse policy.
fn uncollapse_final_assistant_replies(blocks: &mut [TranscriptBlock]) {
    let mut pending: Option<usize> = None;
    for i in 0..blocks.len() {
        match blocks[i].block_type.as_str() {
            "user_message" => {
                if let Some(p) = pending.take() {
                    blocks[p].default_collapsed = false;
                }
            }
            "assistant_message" => pending = Some(i),
            _ => {}
        }
    }
    if let Some(p) = pending {
        blocks[p].default_collapsed = false;
    }
}

fn promote_failed_tasks(blocks: &mut Vec<TranscriptBlock>, lifecycle: &[TranscriptBlock]) {
    for lc in lifecycle {
        if !is_failure_lifecycle(lc) {
            continue;
        }
        let id = format!("promoted:{}", lc.id);
        if blocks.iter().any(|b| b.id == id) {
            continue;
        }
        blocks.push(TranscriptBlock {
            id,
            block_type: "session_error".into(),
            at: lc.at.clone(),
            title: "Task failed".into(),
            body: pick_failure_body(lc),
            meta: json!({ "source": "task_end", "event_type": "task_end" }),
            collapsible: false,
            default_collapsed: false,
            event_id: lc.event_id.clone(),
        });
    }
}

fn pick_failure_body(block: &TranscriptBlock) -> String {
    let body = block.body.trim();
    if !body.is_empty() {
        return humanize_error_text(body);
    }
    humanize_error_text(&block.title)
}

fn fill_missing_turn_replies(
    blocks: &mut Vec<TranscriptBlock>,
    lifecycle: &[TranscriptBlock],
    session: &crate::schema::SessionDetail,
) {
    let mut inserts: Vec<(usize, TranscriptBlock)> = Vec::new();
    for i in 0..blocks.len() {
        if blocks[i].block_type != "user_message" {
            continue;
        }
        let end = blocks[i + 1..]
            .iter()
            .position(|b| b.block_type == "user_message")
            .map(|p| i + 1 + p)
            .unwrap_or(blocks.len());
        let has_reply = blocks[i + 1..end]
            .iter()
            .any(|b| is_reply_block_type(&b.block_type));
        if has_reply {
            continue;
        }
        let user_at = blocks[i].at.clone();
        let next_at = blocks.get(end).map(|b| b.at.as_str());
        if let Some(failure) = find_failure_between(lifecycle, blocks, &user_at, next_at) {
            inserts.push((end, failure));
            continue;
        }
        if end == blocks.len() && !session.summary.trim().is_empty() {
            inserts.push((end, summary_turn_block(session)));
            continue;
        }
        if end == blocks.len() && session.status == "running" {
            continue;
        }
        inserts.push((end, missing_reply_block(&user_at)));
    }
    for (idx, block) in inserts.into_iter().rev() {
        blocks.insert(idx, block);
    }
}

fn find_failure_between(
    lifecycle: &[TranscriptBlock],
    blocks: &[TranscriptBlock],
    user_at: &str,
    next_user_at: Option<&str>,
) -> Option<TranscriptBlock> {
    for lc in lifecycle {
        if !is_failure_lifecycle(lc) {
            continue;
        }
        if !time_in_range(&lc.at, user_at, next_user_at) {
            continue;
        }
        let id = format!("gap:{}", lc.id);
        if blocks.iter().any(|b| b.id == id) {
            continue;
        }
        return Some(TranscriptBlock {
            id,
            block_type: "session_error".into(),
            at: lc.at.clone(),
            title: "Task failed".into(),
            body: pick_failure_body(lc),
            meta: json!({ "source": "task_end_gap" }),
            collapsible: false,
            default_collapsed: false,
            event_id: lc.event_id.clone(),
        });
    }
    None
}

fn summary_turn_block(session: &crate::schema::SessionDetail) -> TranscriptBlock {
    let body = session.summary.trim().to_string();
    let block_type = if looks_like_error_text(&body) {
        "session_error"
    } else {
        "assistant_message"
    };
    TranscriptBlock {
        id: format!("summary:{}", session.id),
        block_type: block_type.into(),
        at: session
            .ended_at
            .clone()
            .unwrap_or_else(|| session.started_at.clone()),
        title: if block_type == "session_error" {
            "Error".into()
        } else {
            "Assistant".into()
        },
        body,
        meta: json!({ "source": "session_summary" }),
        collapsible: false,
        default_collapsed: false,
        event_id: None,
    }
}

fn missing_reply_block(user_at: &str) -> TranscriptBlock {
    TranscriptBlock {
        id: format!("missing:{user_at}"),
        block_type: "system_notice".into(),
        at: user_at.to_string(),
        title: "No reply recorded".into(),
        body: String::new(),
        meta: json!({ "source": "missing_turn" }),
        collapsible: false,
        default_collapsed: false,
        event_id: None,
    }
}

fn time_in_range(at: &str, start: &str, end: Option<&str>) -> bool {
    match (
        chrono::DateTime::parse_from_rfc3339(at),
        chrono::DateTime::parse_from_rfc3339(start),
        end.and_then(|e| chrono::DateTime::parse_from_rfc3339(e).ok()),
    ) {
        (Ok(point), Ok(start_dt), Some(end_dt)) => point >= start_dt && point < end_dt,
        (Ok(point), Ok(start_dt), None) => point >= start_dt,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn humanize_error_text_bare_path() {
        assert_eq!(
            humanize_error_text("path"),
            "Tool parameter error: missing or invalid `path`"
        );
        assert_eq!(humanize_error_text("real failure"), "real failure");
    }
    use crate::schema::{
        CreateSessionRequest, InsertEventRequest, SessionDetail, UpsertProjectRequest,
    };
    use std::sync::{Mutex, OnceLock};
    use tempfile::tempdir;

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[tokio::test]
    async fn completed_session_uses_index_only_without_log() {
        let dir = tempdir().unwrap();
        let db = DashboardDb::open(dir.path().join("transcript-index-only.db"))
            .await
            .unwrap();
        let project = db
            .upsert_project(UpsertProjectRequest {
                root_path: dir.path().join("proj-index").display().to_string(),
                name: Some("IndexOnly".into()),
                description: None,
                create_root: Some(true),
                ..Default::default()
            })
            .await
            .unwrap();
        let session = db
            .create_session(CreateSessionRequest {
                project_id: project.id.clone(),
                kind: "repl".into(),
                task_id: Some("task-no-log".into()),
                title: "chat".into(),
                prompt_preview: Some("hello".into()),
                agent_type: None,
                model: None,
                metadata_json: None,
            })
            .await
            .unwrap();
        db.insert_event(InsertEventRequest {
            project_id: project.id.clone(),
            session_id: Some(session.id.clone()),
            task_id: None,
            agent_id: None,
            event_type: "user_prompt".into(),
            severity: Some("info".into()),
            title: "User prompt".into(),
            body: Some("hello".into()),
            payload: None,
        })
        .await
        .unwrap();
        db.insert_event(InsertEventRequest {
            project_id: project.id.clone(),
            session_id: Some(session.id.clone()),
            task_id: None,
            agent_id: None,
            event_type: "assistant_response".into(),
            severity: Some("info".into()),
            title: "Assistant".into(),
            body: Some("world".into()),
            payload: None,
        })
        .await
        .unwrap();
        db.finish_session(&session.id, "completed", None)
            .await
            .unwrap();

        let transcript = session_transcript(&db, &session.id).await.unwrap();
        assert_eq!(transcript.blocks.len(), 2);
        assert_eq!(transcript.blocks[0].block_type, "user_message");
        assert_eq!(transcript.blocks[1].block_type, "assistant_message");
    }

    #[tokio::test]
    async fn builds_transcript_from_index_events() {
        let dir = tempdir().unwrap();
        let db = DashboardDb::open(dir.path().join("transcript.db"))
            .await
            .unwrap();
        let project = db
            .upsert_project(UpsertProjectRequest {
                root_path: dir.path().join("proj").display().to_string(),
                name: Some("T".into()),
                description: None,
                create_root: Some(true),
                ..Default::default()
            })
            .await
            .unwrap();
        let session = db
            .create_session(CreateSessionRequest {
                project_id: project.id.clone(),
                kind: "repl".into(),
                task_id: None,
                title: "chat".into(),
                prompt_preview: Some("hi".into()),
                agent_type: None,
                model: None,
                metadata_json: None,
            })
            .await
            .unwrap();
        db.insert_event(InsertEventRequest {
            project_id: project.id.clone(),
            session_id: Some(session.id.clone()),
            task_id: None,
            agent_id: None,
            event_type: "task_start".into(),
            severity: Some("info".into()),
            title: "Task start".into(),
            body: None,
            payload: None,
        })
        .await
        .unwrap();
        db.insert_event(InsertEventRequest {
            project_id: project.id.clone(),
            session_id: Some(session.id.clone()),
            task_id: None,
            agent_id: None,
            event_type: "user_prompt".into(),
            severity: Some("info".into()),
            title: "User prompt".into(),
            body: Some("hello".into()),
            payload: None,
        })
        .await
        .unwrap();
        db.insert_event(InsertEventRequest {
            project_id: project.id.clone(),
            session_id: Some(session.id.clone()),
            task_id: None,
            agent_id: None,
            event_type: "assistant_response".into(),
            severity: Some("info".into()),
            title: "Assistant".into(),
            body: Some("world".into()),
            payload: None,
        })
        .await
        .unwrap();

        let transcript = session_transcript(&db, &session.id).await.unwrap();
        assert_eq!(transcript.blocks.len(), 2);
        assert_eq!(transcript.blocks[0].block_type, "user_message");
        assert_eq!(transcript.blocks[1].block_type, "assistant_message");
        assert!(transcript
            .lifecycle
            .iter()
            .any(|b| b.title.contains("task")));
    }

    #[test]
    fn final_assistant_reply_per_turn_is_never_default_collapsed() {
        let mk = |id: &str, block_type: &str, at: &str, collapsed: bool| TranscriptBlock {
            id: id.into(),
            block_type: block_type.into(),
            at: at.into(),
            title: String::new(),
            body: "x".into(),
            meta: json!({}),
            collapsible: collapsed,
            default_collapsed: collapsed,
            event_id: None,
        };
        let mut blocks = vec![
            mk("u1", "user_message", "2026-01-01T00:00:00Z", false),
            mk("a1", "assistant_message", "2026-01-01T00:01:00Z", true),
            mk("a2", "assistant_message", "2026-01-01T00:02:00Z", true),
            mk("u2", "user_message", "2026-01-01T00:03:00Z", false),
            mk("a3", "assistant_message", "2026-01-01T00:04:00Z", true),
        ];
        uncollapse_final_assistant_replies(&mut blocks);
        // Intermediate reply keeps the collapse policy.
        assert!(blocks[1].default_collapsed);
        // Final reply of each turn is expanded.
        assert!(!blocks[2].default_collapsed);
        assert!(!blocks[4].default_collapsed);
    }

    #[test]
    fn fills_missing_replies_between_user_messages() {
        let mut blocks = vec![
            TranscriptBlock {
                id: "u1".into(),
                block_type: "user_message".into(),
                at: "2026-01-01T00:00:00Z".into(),
                title: "You".into(),
                body: "hello".into(),
                meta: json!({}),
                collapsible: false,
                default_collapsed: false,
                event_id: None,
            },
            TranscriptBlock {
                id: "u2".into(),
                block_type: "user_message".into(),
                at: "2026-01-01T01:00:00Z".into(),
                title: "You".into(),
                body: "continue".into(),
                meta: json!({}),
                collapsible: false,
                default_collapsed: false,
                event_id: None,
            },
            TranscriptBlock {
                id: "a1".into(),
                block_type: "assistant_message".into(),
                at: "2026-01-01T01:30:00Z".into(),
                title: "Assistant".into(),
                body: "done".into(),
                meta: json!({}),
                collapsible: false,
                default_collapsed: false,
                event_id: None,
            },
        ];
        let session = SessionDetail {
            id: "sess".into(),
            project_id: "proj".into(),
            project_name: "p".into(),
            kind: "repl".into(),
            task_id: None,
            title: "t".into(),
            prompt_preview: String::new(),
            status: "completed".into(),
            trusted_status: "unverified".into(),
            agent_type: String::new(),
            model: String::new(),
            started_at: "2026-01-01T00:00:00Z".into(),
            ended_at: Some("2026-01-01T02:00:00Z".into()),
            summary: String::new(),
            metadata_json: "{}".into(),
            block_reason: None,
            block_kind: None,
        };
        finalize_conversation_timeline(&mut blocks, &[], &session);
        let missing = blocks
            .iter()
            .filter(|b| b.meta.get("source") == Some(&json!("missing_turn")))
            .count();
        assert_eq!(
            missing, 1,
            "first user turn should get a missing-reply placeholder"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn builds_transcript_from_output_log_assistant_and_tools() {
        let _guard = env_lock().lock().unwrap();
        let dir = tempdir().unwrap();
        let old_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", dir.path());
        let db = DashboardDb::open(dir.path().join("transcript-log.db"))
            .await
            .unwrap();
        let project_root = dir.path().join("proj-log");
        std::fs::create_dir_all(&project_root).unwrap();
        let project = db
            .upsert_project(UpsertProjectRequest {
                root_path: project_root.display().to_string(),
                name: Some("Log".into()),
                description: None,
                create_root: None,
                ..Default::default()
            })
            .await
            .unwrap();
        let task_id = "task-transcript-log";
        let session = db
            .create_session(CreateSessionRequest {
                project_id: project.id.clone(),
                kind: "repl".into(),
                task_id: Some(task_id.into()),
                title: "chat".into(),
                prompt_preview: Some("hi".into()),
                agent_type: None,
                model: None,
                metadata_json: None,
            })
            .await
            .unwrap();
        let log_dir = dir.path().join(".anycode/tasks").join(task_id);
        std::fs::create_dir_all(&log_dir).unwrap();
        std::fs::write(
            log_dir.join("output.log"),
            [
                anycode_core::format_user_prompt_log_line("hello"),
                "[tool_call_start] turn=1 idx=1 name=Bash command=ls".to_string(),
                "[tool_call_end] turn=1 idx=1 name=Bash elapsed_ms=10 error=<none>".to_string(),
                anycode_core::format_assistant_response_log_line(1, "done"),
            ]
            .join("\n"),
        )
        .unwrap();

        let transcript = session_transcript(&db, &session.id).await.unwrap();
        assert!(transcript
            .blocks
            .iter()
            .any(|b| b.block_type == "assistant_message" && b.body == "done"));
        assert!(transcript
            .blocks
            .iter()
            .any(|b| b.block_type == "tool_call"));
        assert!(transcript
            .blocks
            .iter()
            .any(|b| b.block_type == "tool_result"));

        if let Some(home) = old_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }
    }
}
