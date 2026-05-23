//! Security-related project events (denied tools, pending approvals).

use crate::db::DashboardDb;
use crate::schema::SecurityEventRecord;
use anyhow::Result;
use sqlx::Row;

pub async fn list_security_events(
    db: &DashboardDb,
    project_id: Option<&str>,
    limit: i64,
) -> Result<Vec<SecurityEventRecord>> {
    let limit = limit.clamp(1, 200);
    let rows = if let Some(pid) = project_id.filter(|p| !p.is_empty()) {
        sqlx::query(
            r#"
            SELECT e.id, e.project_id, p.name AS project_name, e.session_id,
                   e.event_type, e.severity, e.title, e.body, e.payload_json, e.occurred_at
            FROM project_events e
            JOIN projects p ON p.id = e.project_id
            WHERE e.project_id = ?
              AND e.event_type IN ('tool_denied', 'tool_approval_pending', 'tool_approval_resolved')
            ORDER BY e.occurred_at DESC
            LIMIT ?
            "#,
        )
        .bind(pid)
        .bind(limit)
        .fetch_all(db.pool())
        .await?
    } else {
        sqlx::query(
            r#"
            SELECT e.id, e.project_id, p.name AS project_name, e.session_id,
                   e.event_type, e.severity, e.title, e.body, e.payload_json, e.occurred_at
            FROM project_events e
            JOIN projects p ON p.id = e.project_id
            WHERE e.event_type IN ('tool_denied', 'tool_approval_pending', 'tool_approval_resolved')
            ORDER BY e.occurred_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(db.pool())
        .await?
    };

    Ok(rows
        .into_iter()
        .map(|r| {
            let payload_json: String = r.get("payload_json");
            let payload: serde_json::Value =
                serde_json::from_str(&payload_json).unwrap_or(serde_json::Value::Null);
            let tool_name = payload
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let reason = payload
                .get("reason")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .filter(|s| !s.is_empty())
                .or_else(|| {
                    let body: String = r.get("body");
                    if body.is_empty() {
                        None
                    } else {
                        Some(body)
                    }
                });
            SecurityEventRecord {
                id: r.get("id"),
                project_id: r.get("project_id"),
                project_name: r.get("project_name"),
                session_id: r.get("session_id"),
                event_type: r.get("event_type"),
                severity: r.get("severity"),
                title: r.get("title"),
                tool_name,
                reason,
                occurred_at: r.get("occurred_at"),
            }
        })
        .collect())
}

pub async fn security_event_counts(db: &DashboardDb) -> Result<(i64, i64)> {
    let denied: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM project_events WHERE event_type = 'tool_denied'")
            .fetch_one(db.pool())
            .await?;
    let pending: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM project_events WHERE event_type = 'tool_approval_pending'",
    )
    .fetch_one(db.pool())
    .await?;
    Ok((denied, pending))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::InsertEventRequest;
    use tempfile::tempdir;

    #[tokio::test]
    async fn lists_security_events_only() {
        let dir = tempdir().unwrap();
        let db = DashboardDb::open(dir.path().join("sec.db")).await.unwrap();
        let p = db
            .upsert_project(crate::schema::UpsertProjectRequest {
                root_path: dir.path().to_string_lossy().into(),
                name: Some("sec-test".into()),
                description: None,
            })
            .await
            .unwrap();
        let s = db
            .create_session(crate::schema::CreateSessionRequest {
                project_id: p.id.clone(),
                task_id: Some("t1".into()),
                title: "run".into(),
                kind: "run".into(),
                agent_type: Some("default".into()),
                prompt_preview: None,
                model: None,
                metadata_json: None,
            })
            .await
            .unwrap();
        db.insert_event(InsertEventRequest {
            project_id: p.id.clone(),
            session_id: Some(s.id.clone()),
            task_id: Some("t1".into()),
            agent_id: None,
            event_type: "tool_denied".into(),
            severity: Some("warn".into()),
            title: "Bash denied".into(),
            body: Some("User denied".into()),
            payload: Some(serde_json::json!({ "name": "Bash", "reason": "User denied" })),
        })
        .await
        .unwrap();
        db.insert_event(InsertEventRequest {
            project_id: p.id.clone(),
            session_id: Some(s.id.clone()),
            task_id: Some("t1".into()),
            agent_id: None,
            event_type: "tool_call_end".into(),
            severity: Some("info".into()),
            title: "Bash finished".into(),
            body: None,
            payload: Some(serde_json::json!({ "name": "Bash" })),
        })
        .await
        .unwrap();

        let events = list_security_events(&db, None, 10).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "tool_denied");
        assert_eq!(events[0].tool_name, "Bash");
        assert_eq!(events[0].reason.as_deref(), Some("User denied"));

        let (denied, pending) = security_event_counts(&db).await.unwrap();
        assert_eq!(denied, 1);
        assert_eq!(pending, 0);
    }
}
