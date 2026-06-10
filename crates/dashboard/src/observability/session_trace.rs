//! Execution trace events for a session (stack detail from output.log, DB fallback).

use super::event_tier::is_trace_event_type;
use super::execution_log::read_execution_log;
use crate::db::DashboardDb;
use crate::schema::{ProjectEvent, SessionDetail};
use anycode_core::EXECUTION_TRACE_SCHEMA_VERSION;
use anyhow::{Context, Result};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SessionTraceResponse {
    pub schema_version: u32,
    pub session_id: String,
    pub source: String,
    pub events: Vec<TraceEventRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraceEventRow {
    pub event_type: String,
    pub severity: String,
    pub title: String,
    pub body: String,
    pub payload: serde_json::Value,
    pub occurred_at: String,
}

pub async fn session_trace(db: &DashboardDb, session_id: &str) -> Result<SessionTraceResponse> {
    let session = db
        .get_session(session_id)
        .await?
        .context("session not found")?;

    let log_events = trace_events_from_log(&session);
    if !log_events.is_empty() {
        return Ok(SessionTraceResponse {
            schema_version: EXECUTION_TRACE_SCHEMA_VERSION,
            session_id: session_id.to_string(),
            source: "output.log".into(),
            events: log_events,
        });
    }

    let events = db
        .list_session_events(session_id, None, 500, None, None, None)
        .await?;
    let trace_events: Vec<TraceEventRow> = events
        .into_iter()
        .filter(|e| is_trace_event_type(&e.event_type))
        .map(project_event_to_trace_row)
        .collect();
    Ok(SessionTraceResponse {
        schema_version: EXECUTION_TRACE_SCHEMA_VERSION,
        session_id: session_id.to_string(),
        source: "database".into(),
        events: trace_events,
    })
}

fn trace_events_from_log(session: &SessionDetail) -> Vec<TraceEventRow> {
    let Ok(resp) = read_execution_log(session, 0, Some(500)) else {
        return Vec::new();
    };
    resp.lines
        .into_iter()
        .filter_map(|line| {
            let event_type = line.event_type?;
            if !is_trace_event_type(&event_type) {
                return None;
            }
            Some(TraceEventRow {
                event_type,
                severity: line.severity.unwrap_or_else(|| "info".into()),
                title: line.title.unwrap_or_default(),
                body: line.body.unwrap_or_default(),
                payload: line.payload,
                occurred_at: String::new(),
            })
        })
        .collect()
}

fn project_event_to_trace_row(event: ProjectEvent) -> TraceEventRow {
    TraceEventRow {
        event_type: event.event_type,
        severity: event.severity,
        title: event.title,
        body: event.body,
        payload: event.payload,
        occurred_at: event.occurred_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{CreateSessionRequest, InsertEventRequest, UpsertProjectRequest};
    use tempfile::tempdir;

    #[test]
    fn trace_event_type_filter_uses_event_tier() {
        assert!(is_trace_event_type("tool_call_end"));
        assert!(!is_trace_event_type("user_prompt"));
        assert!(!is_trace_event_type("budget_exceeded"));
    }

    #[tokio::test]
    async fn falls_back_to_db_trace_events_when_log_missing() {
        let dir = tempdir().unwrap();
        let db = DashboardDb::open(dir.path().join("trace.db"))
            .await
            .unwrap();
        let project = db
            .upsert_project(UpsertProjectRequest {
                root_path: "/tmp/trace".into(),
                name: Some("Trace".into()),
                description: None,
                create_root: None,
                ..Default::default()
            })
            .await
            .unwrap();
        let session = db
            .create_session(CreateSessionRequest {
                project_id: project.id.clone(),
                kind: "run".into(),
                task_id: Some("missing-log-task".into()),
                title: "trace".into(),
                prompt_preview: None,
                agent_type: None,
                model: None,
                metadata_json: None,
            })
            .await
            .unwrap();
        db.insert_event(InsertEventRequest {
            project_id: project.id.clone(),
            session_id: Some(session.id.clone()),
            task_id: Some("missing-log-task".into()),
            agent_id: None,
            event_type: "turn_start".into(),
            severity: Some("info".into()),
            title: "Turn 1".into(),
            body: Some(String::new()),
            payload: None,
        })
        .await
        .unwrap();

        let trace = session_trace(&db, &session.id).await.unwrap();
        assert_eq!(trace.source, "database");
        assert_eq!(trace.events.len(), 1);
        assert_eq!(trace.events[0].event_type, "turn_start");
    }
}
