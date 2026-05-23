//! Notification policies and read-only external connector references.

use crate::audit::{record_audit, AuditEventInput};
use crate::db::DashboardDb;
use crate::schema::{ConnectorRecord, NotificationPolicyRecord};
use anyhow::Result;
use serde_json::{json, Value};
use sqlx::Row;
use uuid::Uuid;

pub async fn list_notification_policies(
    db: &DashboardDb,
    project_id: Option<&str>,
) -> Result<Vec<NotificationPolicyRecord>> {
    let mut sql = String::from(
        r#"
        SELECT id, project_id, event_type, channel, config_json, enabled, created_at, updated_at
        FROM notification_policies WHERE 1=1
        "#,
    );
    if project_id.filter(|s| !s.is_empty()).is_some() {
        sql.push_str(" AND project_id = ?");
    }
    sql.push_str(" ORDER BY updated_at DESC");
    let mut q = sqlx::query(&sql);
    if let Some(pid) = project_id.filter(|s| !s.is_empty()) {
        q = q.bind(pid);
    }
    let rows = q.fetch_all(db.pool()).await?;
    Ok(rows.into_iter().map(row_to_notification).collect())
}

pub async fn upsert_notification_policy(
    db: &DashboardDb,
    project_id: Option<&str>,
    event_type: &str,
    channel: &str,
    config: Value,
    enabled: bool,
    id: Option<&str>,
) -> Result<NotificationPolicyRecord> {
    let policy_id = id
        .map(str::to_string)
        .unwrap_or_else(|| format!("ntf_{}", Uuid::new_v4()));
    sqlx::query(
        r#"
        INSERT INTO notification_policies (id, project_id, event_type, channel, config_json, enabled, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, datetime('now'))
        ON CONFLICT(id) DO UPDATE SET
          event_type = excluded.event_type,
          channel = excluded.channel,
          config_json = excluded.config_json,
          enabled = excluded.enabled,
          updated_at = datetime('now')
        "#,
    )
    .bind(&policy_id)
    .bind(project_id)
    .bind(event_type)
    .bind(channel)
    .bind(config.to_string())
    .bind(if enabled { 1i64 } else { 0 })
    .execute(db.pool())
    .await?;
    record_audit(
        db,
        AuditEventInput {
            project_id: project_id.map(str::to_string),
            session_id: None,
            action: "notification_policy_updated".into(),
            risk: "low".into(),
            detail: json!({ "policy_id": policy_id, "event_type": event_type }),
        },
    )
    .await?;
    get_notification_policy(db, &policy_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("policy missing"))
}

pub async fn delete_notification_policy(db: &DashboardDb, id: &str) -> Result<()> {
    let n = sqlx::query("DELETE FROM notification_policies WHERE id = ?")
        .bind(id)
        .execute(db.pool())
        .await?
        .rows_affected();
    if n == 0 {
        anyhow::bail!("policy not found");
    }
    record_audit(
        db,
        AuditEventInput {
            project_id: None,
            session_id: None,
            action: "notification_policy_deleted".into(),
            risk: "low".into(),
            detail: json!({ "policy_id": id }),
        },
    )
    .await?;
    Ok(())
}

pub async fn set_notification_policy_enabled(
    db: &DashboardDb,
    id: &str,
    enabled: bool,
) -> Result<NotificationPolicyRecord> {
    sqlx::query(
        "UPDATE notification_policies SET enabled = ?, updated_at = datetime('now') WHERE id = ?",
    )
    .bind(if enabled { 1i64 } else { 0 })
    .bind(id)
    .execute(db.pool())
    .await?;
    get_notification_policy(db, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("policy not found"))
}

async fn get_notification_policy(
    db: &DashboardDb,
    id: &str,
) -> Result<Option<NotificationPolicyRecord>> {
    let row = sqlx::query(
        r#"
        SELECT id, project_id, event_type, channel, config_json, enabled, created_at, updated_at
        FROM notification_policies WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(db.pool())
    .await?;
    Ok(row.map(row_to_notification))
}

pub async fn list_connectors(
    db: &DashboardDb,
    project_id: Option<&str>,
) -> Result<Vec<ConnectorRecord>> {
    let mut sql = String::from(
        r#"
        SELECT id, project_id, source_type, name, config_json, enabled, created_at, updated_at
        FROM asset_sources WHERE 1=1
        "#,
    );
    if project_id.filter(|s| !s.is_empty()).is_some() {
        sql.push_str(" AND project_id = ?");
    }
    sql.push_str(" ORDER BY name");
    let mut q = sqlx::query(&sql);
    if let Some(pid) = project_id.filter(|s| !s.is_empty()) {
        q = q.bind(pid);
    }
    let rows = q.fetch_all(db.pool()).await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let config_json: String = r.get("config_json");
            let config: Value = serde_json::from_str(&config_json).unwrap_or(Value::Null);
            ConnectorRecord {
                id: r.get("id"),
                project_id: r.get("project_id"),
                source_type: r.get("source_type"),
                name: r.get("name"),
                enabled: r.get::<i64, _>("enabled") != 0,
                config_summary: summarize_connector_config(&config),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            }
        })
        .collect())
}

pub async fn get_connector_config(db: &DashboardDb, id: &str) -> Result<Option<(String, Value)>> {
    let row =
        sqlx::query("SELECT source_type, config_json FROM asset_sources WHERE id = ? LIMIT 1")
            .bind(id)
            .fetch_optional(db.pool())
            .await?;
    Ok(row.map(|r| {
        let config_json: String = r.get("config_json");
        let config: Value = serde_json::from_str(&config_json).unwrap_or(Value::Null);
        (r.get::<String, _>("source_type"), config)
    }))
}

pub async fn upsert_connector(
    db: &DashboardDb,
    project_id: Option<&str>,
    source_type: &str,
    name: &str,
    config: Value,
    enabled: bool,
    id: Option<&str>,
) -> Result<ConnectorRecord> {
    let conn_id = id
        .map(str::to_string)
        .unwrap_or_else(|| format!("con_{}", Uuid::new_v4()));
    let safe_config = redact_secrets(config);
    sqlx::query(
        r#"
        INSERT INTO asset_sources (id, project_id, source_type, name, config_json, enabled, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, datetime('now'))
        ON CONFLICT(id) DO UPDATE SET
          source_type = excluded.source_type,
          name = excluded.name,
          config_json = excluded.config_json,
          enabled = excluded.enabled,
          updated_at = datetime('now')
        "#,
    )
    .bind(&conn_id)
    .bind(project_id)
    .bind(source_type)
    .bind(name)
    .bind(safe_config.to_string())
    .bind(if enabled { 1i64 } else { 0 })
    .execute(db.pool())
    .await?;
    record_audit(
        db,
        AuditEventInput {
            project_id: project_id.map(str::to_string),
            session_id: None,
            action: "connector_updated".into(),
            risk: "medium".into(),
            detail: json!({ "connector_id": conn_id, "source_type": source_type }),
        },
    )
    .await?;
    list_connectors(db, project_id)
        .await?
        .into_iter()
        .find(|c| c.id == conn_id)
        .ok_or_else(|| anyhow::anyhow!("connector missing"))
}

pub async fn delete_connector(db: &DashboardDb, id: &str) -> Result<()> {
    let n = sqlx::query("DELETE FROM asset_sources WHERE id = ?")
        .bind(id)
        .execute(db.pool())
        .await?
        .rows_affected();
    if n == 0 {
        anyhow::bail!("connector not found");
    }
    record_audit(
        db,
        AuditEventInput {
            project_id: None,
            session_id: None,
            action: "connector_deleted".into(),
            risk: "medium".into(),
            detail: json!({ "connector_id": id }),
        },
    )
    .await?;
    Ok(())
}

pub async fn set_connector_enabled(
    db: &DashboardDb,
    id: &str,
    enabled: bool,
) -> Result<ConnectorRecord> {
    sqlx::query("UPDATE asset_sources SET enabled = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(if enabled { 1i64 } else { 0 })
        .bind(id)
        .execute(db.pool())
        .await?;
    record_audit(
        db,
        AuditEventInput {
            project_id: None,
            session_id: None,
            action: "connector_updated".into(),
            risk: "low".into(),
            detail: json!({ "connector_id": id, "enabled": enabled }),
        },
    )
    .await?;
    list_connectors(db, None)
        .await?
        .into_iter()
        .find(|c| c.id == id)
        .ok_or_else(|| anyhow::anyhow!("connector missing"))
}

fn row_to_notification(r: sqlx::sqlite::SqliteRow) -> NotificationPolicyRecord {
    let config_json: String = r.get("config_json");
    let config: Value = serde_json::from_str(&config_json).unwrap_or(Value::Null);
    NotificationPolicyRecord {
        id: r.get("id"),
        project_id: r.get("project_id"),
        event_type: r.get("event_type"),
        channel: r.get("channel"),
        enabled: r.get::<i64, _>("enabled") != 0,
        config,
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }
}

fn summarize_connector_config(config: &Value) -> String {
    if let Some(url) = config.get("url").and_then(|v| v.as_str()) {
        return url.to_string();
    }
    if let Some(repo) = config.get("repo").and_then(|v| v.as_str()) {
        return repo.to_string();
    }
    if let Some(key) = config.get("team_key").and_then(|v| v.as_str()) {
        return format!("team:{key}");
    }
    if let Some(id) = config.get("team_id").and_then(|v| v.as_str()) {
        return format!("team_id:{id}");
    }
    "configured".into()
}

fn redact_secrets(mut config: Value) -> Value {
    if let Some(obj) = config.as_object_mut() {
        for key in ["token", "secret", "api_key", "password"] {
            if obj.contains_key(key) {
                obj.insert(key.into(), json!("***redacted***"));
            }
        }
    }
    config
}

pub async fn emit_local_log(
    db: &DashboardDb,
    project_id: Option<&str>,
    session_id: Option<&str>,
    event_type: &str,
    detail: Value,
) -> Result<()> {
    let policies = list_notification_policies(db, project_id).await?;
    let matches: Vec<_> = policies
        .iter()
        .filter(|p| p.enabled && p.channel == "local_log" && p.event_type == event_type)
        .collect();
    if matches.is_empty() {
        return Ok(());
    }
    record_audit(
        db,
        AuditEventInput {
            project_id: project_id.map(str::to_string),
            session_id: session_id.map(str::to_string),
            action: "notification_local_log".into(),
            risk: "low".into(),
            detail: json!({
                "event_type": event_type,
                "channel": "local_log",
                "payload": detail,
            }),
        },
    )
    .await?;
    Ok(())
}

pub async fn send_test_notification(
    db: &DashboardDb,
    project_id: Option<&str>,
    event_type: &str,
) -> Result<()> {
    emit_local_log(
        db,
        project_id,
        None,
        event_type,
        json!({ "test": true, "message": "dashboard test notification" }),
    )
    .await?;
    record_audit(
        db,
        AuditEventInput {
            project_id: project_id.map(str::to_string),
            session_id: None,
            action: "notification_test_sent".into(),
            risk: "low".into(),
            detail: json!({ "event_type": event_type, "channel": "local_log" }),
        },
    )
    .await?;
    Ok(())
}
