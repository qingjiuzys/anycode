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
              'session_blocked',
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
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .or_else(|| {
                    meta.get("payload")
                        .and_then(|p| p.get("message"))
                        .and_then(|v| v.as_str())
                        .map(str::to_string)
                })
                .unwrap_or_default();
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

fn audit_filter_where(
    project_id: Option<&str>,
    action: Option<&str>,
    risk: Option<&str>,
) -> (String, bool, bool, bool) {
    let mut sql = String::from("WHERE source = ?");
    let has_project = project_id.filter(|s| !s.is_empty()).is_some();
    let has_action = action.filter(|s| !s.is_empty()).is_some();
    let has_risk = risk.filter(|s| !s.is_empty()).is_some();
    if has_project {
        sql.push_str(" AND json_extract(metadata_json, '$.project_id') = ?");
    }
    if has_action {
        sql.push_str(" AND event_type = ?");
    }
    if has_risk {
        sql.push_str(" AND json_extract(metadata_json, '$.risk') = ?");
    }
    (sql, has_project, has_action, has_risk)
}

fn row_to_audit_record(row: &sqlx::sqlite::SqliteRow) -> Option<AuditRecord> {
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
}

pub async fn list_audit_events(
    db: &DashboardDb,
    project_id: Option<&str>,
    action: Option<&str>,
    risk: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<(Vec<AuditRecord>, i64)> {
    let limit = limit.clamp(1, 200);
    let offset = offset.max(0);
    let (where_sql, has_project, has_action, has_risk) =
        audit_filter_where(project_id, action, risk);

    let count_sql = format!("SELECT COUNT(*) AS cnt FROM auth_events {where_sql}");
    let mut count_q = sqlx::query(&count_sql).bind(AUDIT_SOURCE);
    if has_project {
        count_q = count_q.bind(project_id.filter(|s| !s.is_empty()).unwrap());
    }
    if has_action {
        count_q = count_q.bind(action.filter(|s| !s.is_empty()).unwrap());
    }
    if has_risk {
        count_q = count_q.bind(risk.filter(|s| !s.is_empty()).unwrap());
    }
    let total: i64 = count_q.fetch_one(db.pool()).await?.get("cnt");

    let list_sql = format!(
        r#"
        SELECT id, event_type, metadata_json, created_at
        FROM auth_events
        {where_sql}
        ORDER BY created_at DESC
        LIMIT ? OFFSET ?
        "#
    );
    let mut list_q = sqlx::query(&list_sql).bind(AUDIT_SOURCE);
    if has_project {
        list_q = list_q.bind(project_id.filter(|s| !s.is_empty()).unwrap());
    }
    if has_action {
        list_q = list_q.bind(action.filter(|s| !s.is_empty()).unwrap());
    }
    if has_risk {
        list_q = list_q.bind(risk.filter(|s| !s.is_empty()).unwrap());
    }
    let rows = list_q.bind(limit).bind(offset).fetch_all(db.pool()).await?;
    let events = rows
        .iter()
        .filter_map(row_to_audit_record)
        .collect::<Vec<_>>();
    Ok((events, total))
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
        let (all, total) = list_audit_events(&db, None, None, None, 10, 0)
            .await
            .unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(total, 2);
        let (filtered, filtered_total) = list_audit_events(&db, Some("proj_a"), None, None, 10, 0)
            .await
            .unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered_total, 1);
        assert_eq!(filtered[0].action, "project_reindex_requested");
        let (by_action, _) = list_audit_events(&db, None, Some("dashboard_started"), None, 10, 0)
            .await
            .unwrap();
        assert_eq!(by_action.len(), 1);
    }

    #[tokio::test]
    async fn audit_pagination_offset_and_total() {
        let dir = tempdir().unwrap();
        let db = DashboardDb::open(dir.path().join("audit-page.db"))
            .await
            .unwrap();
        for i in 0..5 {
            record_audit(
                &db,
                AuditEventInput::low(format!("audit_page_event_{i}"), json!({ "i": i })),
            )
            .await
            .unwrap();
        }
        let (page1, total) = list_audit_events(&db, None, None, None, 2, 0)
            .await
            .unwrap();
        assert_eq!(total, 5);
        assert_eq!(page1.len(), 2);
        let (page2, total2) = list_audit_events(&db, None, None, None, 2, 2)
            .await
            .unwrap();
        assert_eq!(total2, 5);
        assert_eq!(page2.len(), 2);
        assert_ne!(page1[0].id, page2[0].id);
        let (page3, _) = list_audit_events(&db, None, None, None, 2, 4)
            .await
            .unwrap();
        assert_eq!(page3.len(), 1);
    }
}
