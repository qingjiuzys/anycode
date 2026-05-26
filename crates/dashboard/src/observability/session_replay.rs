//! Aggregated session replay summary for UI recap.

use crate::db::DashboardDb;
use crate::schema::{
    GateRecord, ProjectEvent, SessionReplaySummary, SessionSummaryFailure, TracePhaseSummary,
};
use anyhow::{Context, Result};

pub async fn session_replay(db: &DashboardDb, session_id: &str) -> Result<SessionReplaySummary> {
    let session = db
        .get_session(session_id)
        .await?
        .context("session not found")?;

    let gates = db.list_gates_for_session(session_id).await?;
    let artifacts = db
        .list_artifacts(None, Some(session_id), None, None, false, false, 50)
        .await?;
    let events = db
        .list_session_events(session_id, None, 100, None, None, None)
        .await?;

    let failed_gates: Vec<GateRecord> = gates
        .iter()
        .filter(|g| g.status == "failed")
        .cloned()
        .collect();

    let last_error = events
        .iter()
        .filter(|e| e.severity == "error" || e.event_type.contains("fail"))
        .max_by_key(|e| e.occurred_at.clone())
        .map(|e| SessionSummaryFailure {
            event_id: e.id.clone(),
            title: e.title.clone(),
            event_type: e.event_type.clone(),
            occurred_at: e.occurred_at.clone(),
            body: e.body.clone(),
        });

    let report_artifacts = db
        .list_artifacts(
            Some(&session.project_id),
            Some(session_id),
            Some("report"),
            None,
            false,
            false,
            10,
        )
        .await?;

    let tool_calls_recent: Vec<crate::schema::ToolCallSummary> = events
        .iter()
        .filter(|e| e.event_type.starts_with("tool_call"))
        .rev()
        .take(8)
        .map(|e| {
            let tool_name = extract_tool_name(&e.title, &e.body);
            crate::schema::ToolCallSummary {
                event_id: e.id.clone(),
                title: e.title.clone(),
                body: e.body.clone(),
                occurred_at: e.occurred_at.clone(),
                event_type: e.event_type.clone(),
                tool_name,
            }
        })
        .collect();

    let attempt_count = goal_attempt_count(&session.metadata_json, &session.kind, &events);
    let trace_phases = trace_phase_summary(&events);
    let llm_calls_count = events
        .iter()
        .filter(|e| e.event_type == "llm_response_end")
        .count() as i64;
    let tool_calls_count = events
        .iter()
        .filter(|e| e.event_type == "tool_call_end")
        .count() as i64;
    let budget_events_count = events
        .iter()
        .filter(|e| e.event_type.starts_with("budget_"))
        .count() as i64;
    let budget_status = budget_status(&events);

    let active_agent = if session.status == "running" {
        Some(session.agent_type.clone()).filter(|s| !s.is_empty())
    } else {
        None
    };

    Ok(SessionReplaySummary {
        session_id: session.id.clone(),
        project_id: session.project_id.clone(),
        project_name: session.project_name.clone(),
        title: session.title.clone(),
        status: session.status.clone(),
        trusted_status: session.trusted_status.clone(),
        kind: session.kind.clone(),
        failed_gates,
        last_error,
        artifacts,
        recent_events: events
            .into_iter()
            .rev()
            .take(20)
            .collect::<Vec<ProjectEvent>>(),
        report_artifacts,
        generated_at: chrono::Utc::now().to_rfc3339(),
        attempt_count,
        active_agent,
        tool_calls_recent,
        trace_phases,
        llm_calls_count,
        tool_calls_count,
        budget_events_count,
        budget_status,
    })
}

fn budget_status(events: &[ProjectEvent]) -> String {
    if events.iter().any(|e| e.event_type == "budget_exceeded") {
        "exceeded".into()
    } else if events.iter().any(|e| e.event_type == "budget_degrade") {
        "degraded".into()
    } else if events.iter().any(|e| e.event_type == "budget_warning") {
        "warn".into()
    } else {
        "ok".into()
    }
}

fn trace_phase_summary(events: &[ProjectEvent]) -> Vec<TracePhaseSummary> {
    let phases = [
        ("task", ["task_start", "task_end"].as_slice()),
        ("turns", ["turn_start", "turn_end"].as_slice()),
        ("llm", ["llm_request_start", "llm_response_end"].as_slice()),
        (
            "tools",
            [
                "tool_call_input",
                "tool_call_start",
                "tool_call_end",
                "tool_denied",
                "tool_approval_pending",
                "tool_approval_resolved",
            ]
            .as_slice(),
        ),
        ("gates", ["gate"].as_slice()),
        (
            "budget",
            ["budget_warning", "budget_degrade", "budget_exceeded"].as_slice(),
        ),
    ];

    phases
        .iter()
        .filter_map(|(phase, kinds)| {
            let mut count = 0i64;
            let mut severity = "info";
            for event in events
                .iter()
                .filter(|e| kinds.contains(&e.event_type.as_str()))
            {
                count += 1;
                if event.severity == "error" {
                    severity = "error";
                } else if event.severity == "warn" && severity != "error" {
                    severity = "warn";
                }
            }
            (count > 0).then(|| TracePhaseSummary {
                phase: (*phase).to_string(),
                count,
                severity: severity.to_string(),
            })
        })
        .collect()
}

fn goal_attempt_count(metadata_json: &str, kind: &str, events: &[ProjectEvent]) -> u32 {
    if let Ok(meta) = serde_json::from_str::<serde_json::Value>(metadata_json) {
        if let Some(n) = meta.get("goal_attempts").and_then(|v| v.as_u64()) {
            return n as u32;
        }
    }
    if kind == "goal" || kind == "workflow" {
        let turns = events.iter().filter(|e| e.event_type == "turn_end").count();
        if turns > 0 {
            return turns as u32;
        }
    }
    events
        .iter()
        .filter(|e| e.event_type == "tool_call_end")
        .count() as u32
}

fn extract_tool_name(title: &str, body: &str) -> Option<String> {
    for src in [title, body] {
        if let Some(rest) = src.strip_prefix("name=") {
            let name = rest.split_whitespace().next()?.trim();
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
        if src.contains("name=") {
            if let Some(idx) = src.find("name=") {
                let rest = &src[idx + 5..];
                let name = rest.split_whitespace().next()?.trim();
                if !name.is_empty() {
                    return Some(name.to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{CreateSessionRequest, UpsertProjectRequest};
    use tempfile::tempdir;

    #[tokio::test]
    async fn replay_empty_session() {
        let dir = tempdir().unwrap();
        let db = DashboardDb::open(dir.path().join("r.db")).await.unwrap();
        let project = db
            .upsert_project(UpsertProjectRequest {
                root_path: "/tmp/replay".into(),
                name: Some("Replay".into()),
                description: None,
                create_root: None,
            })
            .await
            .unwrap();
        let session = db
            .create_session(CreateSessionRequest {
                project_id: project.id.clone(),
                kind: "run".into(),
                task_id: None,
                title: "test".into(),
                prompt_preview: None,
                agent_type: None,
                model: None,
                metadata_json: None,
            })
            .await
            .unwrap();
        let replay = session_replay(&db, &session.id).await.unwrap();
        assert_eq!(replay.session_id, session.id);
        assert!(replay.failed_gates.is_empty());
        assert_eq!(replay.attempt_count, 0);
        assert!(replay.tool_calls_recent.is_empty());
        assert!(replay.trace_phases.is_empty());
    }

    #[tokio::test]
    async fn replay_uses_goal_attempts_metadata() {
        let dir = tempdir().unwrap();
        let db = DashboardDb::open(dir.path().join("meta.db")).await.unwrap();
        let project = db
            .upsert_project(UpsertProjectRequest {
                root_path: "/tmp/meta".into(),
                name: Some("Meta".into()),
                description: None,
                create_root: None,
            })
            .await
            .unwrap();
        let session = db
            .create_session(CreateSessionRequest {
                project_id: project.id.clone(),
                kind: "goal".into(),
                task_id: None,
                title: "goal".into(),
                prompt_preview: None,
                agent_type: None,
                model: None,
                metadata_json: Some(r#"{"goal_attempts": 4}"#.into()),
            })
            .await
            .unwrap();
        let replay = session_replay(&db, &session.id).await.unwrap();
        assert_eq!(replay.attempt_count, 4);
    }

    #[test]
    fn trace_phase_summary_groups_events() {
        let events = vec![
            ProjectEvent {
                id: "e1".into(),
                project_id: "p".into(),
                session_id: Some("s".into()),
                task_id: None,
                agent_id: None,
                event_type: "llm_response_end".into(),
                severity: "info".into(),
                title: "LLM".into(),
                body: String::new(),
                payload: serde_json::json!({}),
                occurred_at: "now".into(),
            },
            ProjectEvent {
                id: "e2".into(),
                project_id: "p".into(),
                session_id: Some("s".into()),
                task_id: None,
                agent_id: None,
                event_type: "tool_call_end".into(),
                severity: "error".into(),
                title: "Tool failed".into(),
                body: String::new(),
                payload: serde_json::json!({}),
                occurred_at: "now".into(),
            },
        ];
        let phases = trace_phase_summary(&events);
        assert_eq!(phases.iter().find(|p| p.phase == "llm").unwrap().count, 1);
        assert_eq!(
            phases.iter().find(|p| p.phase == "tools").unwrap().severity,
            "error"
        );
    }
}
