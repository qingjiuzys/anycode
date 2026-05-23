//! Project automation policy CRUD.

use crate::audit::{record_audit, AuditEventInput};
use crate::db::DashboardDb;
use crate::schema::AutomationPolicyRecord;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use sqlx::Row;
use uuid::Uuid;

pub async fn list_policies(
    db: &DashboardDb,
    project_id: &str,
) -> Result<Vec<AutomationPolicyRecord>> {
    let rows = sqlx::query(
        r#"
        SELECT id, project_id, name, enabled, policy_type, config_json, created_at, updated_at
        FROM automation_policies WHERE project_id = ? ORDER BY updated_at DESC
        "#,
    )
    .bind(project_id)
    .fetch_all(db.pool())
    .await?;
    Ok(rows.into_iter().map(row_to_policy).collect())
}

pub async fn upsert_policy(
    db: &DashboardDb,
    project_id: &str,
    name: &str,
    policy_type: &str,
    config: Value,
    enabled: bool,
    id: Option<&str>,
) -> Result<AutomationPolicyRecord> {
    let policy_id = id
        .map(str::to_string)
        .unwrap_or_else(|| format!("pol_{}", Uuid::new_v4()));
    sqlx::query(
        r#"
        INSERT INTO automation_policies (id, project_id, name, enabled, policy_type, config_json, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, datetime('now'))
        ON CONFLICT(id) DO UPDATE SET
          name = excluded.name,
          enabled = excluded.enabled,
          policy_type = excluded.policy_type,
          config_json = excluded.config_json,
          updated_at = datetime('now')
        "#,
    )
    .bind(&policy_id)
    .bind(project_id)
    .bind(name)
    .bind(if enabled { 1i64 } else { 0 })
    .bind(policy_type)
    .bind(config.to_string())
    .execute(db.pool())
    .await?;
    record_audit(
        db,
        AuditEventInput {
            project_id: Some(project_id.into()),
            session_id: None,
            action: "automation_policy_updated".into(),
            risk: "medium".into(),
            detail: json!({ "policy_id": policy_id, "policy_type": policy_type }),
        },
    )
    .await?;
    get_policy(db, &policy_id)
        .await?
        .ok_or_else(|| anyhow!("policy not found after upsert"))
}

pub async fn get_policy(db: &DashboardDb, id: &str) -> Result<Option<AutomationPolicyRecord>> {
    let row = sqlx::query(
        r#"
        SELECT id, project_id, name, enabled, policy_type, config_json, created_at, updated_at
        FROM automation_policies WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(db.pool())
    .await?;
    Ok(row.map(row_to_policy))
}

pub async fn delete_policy(db: &DashboardDb, id: &str) -> Result<bool> {
    let row = get_policy(db, id).await?;
    let Some(p) = row else {
        return Ok(false);
    };
    let res = sqlx::query("DELETE FROM automation_policies WHERE id = ?")
        .bind(id)
        .execute(db.pool())
        .await?;
    if res.rows_affected() > 0 {
        record_audit(
            db,
            AuditEventInput {
                project_id: Some(p.project_id),
                session_id: None,
                action: "automation_policy_deleted".into(),
                risk: "medium".into(),
                detail: json!({ "policy_id": id }),
            },
        )
        .await?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// When enabled `gate_block` policies exist, emit `gate_failed` local_log notifications.
pub async fn handle_gate_outcome(
    db: &DashboardDb,
    project_id: &str,
    session_id: &str,
    gate_name: &str,
    status: &str,
    required: bool,
) -> Result<()> {
    if !required || status != "failed" {
        return Ok(());
    }
    let policies = list_policies(db, project_id).await?;
    if !policies
        .iter()
        .any(|p| p.enabled && p.policy_type == "gate_block")
    {
        return Ok(());
    }
    crate::notifications::emit_local_log(
        db,
        Some(project_id),
        Some(session_id),
        "gate_failed",
        json!({ "gate": gate_name, "source": "automation_policy" }),
    )
    .await
}

/// When enabled `report_on_complete` policies exist, notify after verified completion.
pub async fn handle_session_completed(
    db: &DashboardDb,
    project_id: &str,
    session_id: &str,
    session_status: &str,
    trusted_status: &str,
) -> Result<()> {
    if session_status != "completed" || trusted_status != "verified" {
        return Ok(());
    }
    let policies = list_policies(db, project_id).await?;
    if !policies
        .iter()
        .any(|p| p.enabled && p.policy_type == "report_on_complete")
    {
        return Ok(());
    }
    crate::notifications::emit_local_log(
        db,
        Some(project_id),
        Some(session_id),
        "session_report_generated",
        json!({ "trigger": "automation_policy" }),
    )
    .await
}

/// Emit `session_blocked` when trust becomes blocked and `gate_block` policy is enabled.
pub async fn handle_trust_blocked(
    db: &DashboardDb,
    project_id: &str,
    session_id: &str,
    trusted_status: &str,
) -> Result<()> {
    if trusted_status != "blocked" {
        return Ok(());
    }
    let policies = list_policies(db, project_id).await?;
    if !policies
        .iter()
        .any(|p| p.enabled && p.policy_type == "gate_block")
    {
        return Ok(());
    }
    crate::notifications::emit_local_log(
        db,
        Some(project_id),
        Some(session_id),
        "session_blocked",
        json!({ "source": "automation_policy" }),
    )
    .await
}

fn row_to_policy(r: sqlx::sqlite::SqliteRow) -> AutomationPolicyRecord {
    let config_json: String = r.get("config_json");
    let config: Value = serde_json::from_str(&config_json).unwrap_or(Value::Null);
    AutomationPolicyRecord {
        id: r.get("id"),
        project_id: r.get("project_id"),
        name: r.get("name"),
        enabled: r.get::<i64, _>("enabled") != 0,
        policy_type: r.get("policy_type"),
        config,
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{CreateSessionRequest, UpsertProjectRequest};
    use tempfile::tempdir;

    #[tokio::test]
    async fn gate_block_policy_emits_gate_failed_notification() {
        let dir = tempdir().unwrap();
        let db = DashboardDb::open(dir.path().join("auto.db")).await.unwrap();
        let project = db
            .upsert_project(UpsertProjectRequest {
                root_path: "/tmp/auto".into(),
                name: Some("Auto".into()),
                description: None,
            })
            .await
            .unwrap();
        let session = db
            .create_session(CreateSessionRequest {
                project_id: project.id.clone(),
                kind: "goal".into(),
                task_id: None,
                title: "auto".into(),
                prompt_preview: None,
                agent_type: None,
                model: None,
                metadata_json: None,
            })
            .await
            .unwrap();
        upsert_policy(
            &db,
            &project.id,
            "block on fail",
            "gate_block",
            json!({}),
            true,
            None,
        )
        .await
        .unwrap();
        crate::notifications::upsert_notification_policy(
            &db,
            Some(&project.id),
            "gate_failed",
            "local_log",
            json!({}),
            true,
            None,
        )
        .await
        .unwrap();
        handle_gate_outcome(&db, &project.id, &session.id, "cargo test", "failed", true)
            .await
            .unwrap();
        let audit = crate::audit::list_audit_events(&db, None, None, None, 20)
            .await
            .unwrap();
        assert!(
            audit.iter().any(|e| e.action == "notification_local_log"),
            "expected notification audit entry"
        );
    }
}
