//! Low-risk dashboard action audit log (stored in `auth_events`).

use crate::db::DashboardDb;
use crate::schema::{AuditRecord, LOCAL_ORG_ID, LOCAL_USER_ID};
use anyhow::Result;
use serde_json::{json, Value};
use sqlx::Row;
use uuid::Uuid;

pub const AUDIT_SOURCE: &str = "dashboard";

#[derive(Debug, Clone)]
pub struct AuditEventInput {
    pub project_id: Option<String>,
    pub session_id: Option<String>,
    pub action: String,
    pub risk: String,
    pub detail: Value,
}

impl AuditEventInput {
    pub fn low(action: impl Into<String>, detail: Value) -> Self {
        Self {
            project_id: None,
            session_id: None,
            action: action.into(),
            risk: "low".into(),
            detail,
        }
    }

    pub fn with_project(mut self, project_id: impl Into<String>) -> Self {
        self.project_id = Some(project_id.into());
        self
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }
}

pub async fn record_audit(db: &DashboardDb, input: AuditEventInput) -> Result<()> {
    let id = format!("audit_{}", Uuid::new_v4());
    let metadata = json!({
        "project_id": input.project_id,
        "session_id": input.session_id,
        "risk": input.risk,
        "actor": "local",
        "detail": input.detail,
    });
    sqlx::query(
        r#"
        INSERT INTO auth_events (id, organization_id, user_id, event_type, source, metadata_json)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(LOCAL_ORG_ID)
    .bind(LOCAL_USER_ID)
    .bind(&input.action)
    .bind(AUDIT_SOURCE)
    .bind(metadata.to_string())
    .execute(db.pool())
    .await?;
    Ok(())
}

pub async fn list_recent_notifications(
    db: &DashboardDb,
    limit: i64,
) -> Result<Vec<crate::schema::RecentNotification>> {
    use crate::schema::RecentNotification;
    let rows = sqlx::query(
        r#"
        SELECT id, event_type, metadata_json, created_at
        FROM auth_events
        WHERE source = ?
          AND (
            event_type LIKE 'notification_%'
            OR event_type IN (
              'gate_failed',
              'session_completed',
              'session_report_generated',
              'blocked_threshold_exceeded'
            )
          )
        ORDER BY created_at DESC
        LIMIT ?
        "#,
    )
    .bind(AUDIT_SOURCE)
    .bind(limit.clamp(1, 50))
    .fetch_all(db.pool())
    .await?;

    Ok(rows
        .into_iter()
        .filter_map(|r| {
            let meta: serde_json::Value =
                serde_json::from_str(r.get::<String, _>("metadata_json").as_str())
                    .unwrap_or_default();
            let action: String = r.get("event_type");
            let title = meta
                .get("title")
                .or_else(|| meta.get("event_type"))
                .and_then(|v| v.as_str())
                .unwrap_or(&action)
                .to_string();
            let detail = meta
                .get("detail")
                .or_else(|| meta.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Some(RecentNotification {
                id: r.get("id"),
                action,
                title,
                detail,
                created_at: r.get("created_at"),
                project_id: meta
                    .get("project_id")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
            })
        })
        .collect())
}

pub async fn list_audit_events(
    db: &DashboardDb,
    project_id: Option<&str>,
    action: Option<&str>,
    risk: Option<&str>,
    limit: i64,
) -> Result<Vec<AuditRecord>> {
    let mut sql = String::from(
        r#"
        SELECT id, event_type, metadata_json, created_at
        FROM auth_events
        WHERE source = ?
        "#,
    );
    if project_id.filter(|s| !s.is_empty()).is_some() {
        sql.push_str(" AND json_extract(metadata_json, '$.project_id') = ?");
    }
    if action.filter(|s| !s.is_empty()).is_some() {
        sql.push_str(" AND event_type = ?");
    }
    if risk.filter(|s| !s.is_empty()).is_some() {
        sql.push_str(" AND json_extract(metadata_json, '$.risk') = ?");
    }
    sql.push_str(" ORDER BY created_at DESC LIMIT ?");
    let mut q = sqlx::query(&sql).bind(AUDIT_SOURCE);
    if let Some(pid) = project_id.filter(|s| !s.is_empty()) {
        q = q.bind(pid);
    }
    if let Some(act) = action.filter(|s| !s.is_empty()) {
        q = q.bind(act);
    }
    if let Some(r) = risk.filter(|s| !s.is_empty()) {
        q = q.bind(r);
    }
    let rows = q.bind(limit).fetch_all(db.pool()).await?;
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let metadata_json: String = row.get("metadata_json");
            let meta: Value = serde_json::from_str(&metadata_json).ok()?;
            Some(AuditRecord {
                id: row.get("id"),
                project_id: meta
                    .get("project_id")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                session_id: meta
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                actor: meta
                    .get("actor")
                    .and_then(|v| v.as_str())
                    .unwrap_or("local")
                    .to_string(),
                action: row.get("event_type"),
                risk: meta
                    .get("risk")
                    .and_then(|v| v.as_str())
                    .unwrap_or("low")
                    .to_string(),
                detail: meta.get("detail").cloned().unwrap_or(Value::Null),
                created_at: row.get("created_at"),
            })
        })
        .collect())
}

pub fn policy_summary(host: &str, port: u16) -> crate::schema::PolicySummary {
    let remote = host != "127.0.0.1" && host != "localhost" && host != "::1";
    crate::schema::PolicySummary {
        mode: if remote {
            "local_authenticated".into()
        } else {
            "local_trusted".into()
        },
        host_binding: format!("{host}:{port}"),
        remote_access_allowed: false,
        write_actions_allowed: false,
        safe_actions: vec![
            "reindex".into(),
            "report_export".into(),
            "skills_rescan".into(),
            "tool_approval".into(),
        ],
        blocked_actions: vec![
            "edit_files".into(),
            "delete_files".into(),
            "git_push".into(),
            "deploy".into(),
            "stop_task".into(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[tokio::test]
    async fn audit_write_and_filter() {
        let dir = tempdir().unwrap();
        let db = DashboardDb::open(dir.path().join("audit.db"))
            .await
            .unwrap();
        record_audit(
            &db,
            AuditEventInput::low("dashboard_started", json!({ "version": "test" })),
        )
        .await
        .unwrap();
        record_audit(
            &db,
            AuditEventInput::low("project_reindex_requested", json!({})).with_project("proj_a"),
        )
        .await
        .unwrap();
        let all = list_audit_events(&db, None, None, None, 10).await.unwrap();
        assert_eq!(all.len(), 2);
        let filtered = list_audit_events(&db, Some("proj_a"), None, None, 10)
            .await
            .unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].action, "project_reindex_requested");
        let by_action = list_audit_events(&db, None, Some("dashboard_started"), None, 10)
            .await
            .unwrap();
        assert_eq!(by_action.len(), 1);
    }
}
