//! Skill registry detail, permissions, and run history.

use crate::db::DashboardDb;
use crate::schema::{SkillDetailRecord, SkillProjectLink, SkillRunRecord};
use anyhow::Result;
use serde_json::{json, Value};
use sqlx::Row;
use std::path::Path;
use uuid::Uuid;

pub async fn get_skill_detail(
    db: &DashboardDb,
    skill_id: &str,
) -> Result<Option<SkillDetailRecord>> {
    let row = sqlx::query(
        r#"
        SELECT s.id, s.name, s.description, s.description_zh, s.source_path, s.category, s.permissions_json,
               (SELECT COUNT(*) FROM project_skills ps WHERE ps.skill_id = s.id AND ps.enabled = 1) AS projects_count
        FROM skills s WHERE s.id = ?
        "#,
    )
    .bind(skill_id)
    .fetch_optional(db.pool())
    .await?;
    let Some(r) = row else {
        return Ok(None);
    };
    let permissions_json: String = r.get("permissions_json");
    let mut permissions: Value = serde_json::from_str(&permissions_json).unwrap_or(json!({}));
    let source_path: String = r.get("source_path");
    if permissions.is_null() || permissions.as_object().is_some_and(|o| o.is_empty()) {
        permissions = parse_skill_permissions(Path::new(&source_path));
    }
    let runs = list_skill_runs(db, skill_id, 10).await?;
    let projects = list_skill_projects(db, skill_id).await?;
    Ok(Some(SkillDetailRecord {
        id: r.get("id"),
        name: r.get("name"),
        description: r.get("description"),
        description_zh: r.get("description_zh"),
        source_path,
        category: r.get("category"),
        permissions,
        projects_count: r.get("projects_count"),
        projects,
        recent_runs: runs,
    }))
}

pub async fn list_skill_projects(
    db: &DashboardDb,
    skill_id: &str,
) -> Result<Vec<SkillProjectLink>> {
    let rows = sqlx::query(
        r#"
        SELECT p.id, p.name, COALESCE(ps.enabled, 0) AS enabled
        FROM projects p
        LEFT JOIN project_skills ps ON ps.project_id = p.id AND ps.skill_id = ?
        ORDER BY p.name COLLATE NOCASE
        "#,
    )
    .bind(skill_id)
    .fetch_all(db.pool())
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| SkillProjectLink {
            project_id: r.get("id"),
            project_name: r.get("name"),
            enabled: r.get::<i64, _>("enabled") != 0,
        })
        .collect())
}

pub fn parse_skill_permissions(skill_path: &Path) -> Value {
    let skill_md = skill_path.join("SKILL.md");
    let Ok(text) = std::fs::read_to_string(skill_md) else {
        return json!({ "read_dirs": [], "write_dirs": [], "network": false });
    };
    let mut read_dirs = Vec::new();
    let mut write_dirs = Vec::new();
    let mut network = false;
    for line in text.lines().take(80) {
        let l = line.trim();
        if l.starts_with("read:") || l.contains("read-only") {
            read_dirs.push(l.trim_start_matches("read:").trim());
        }
        if l.starts_with("write:") {
            write_dirs.push(l.trim_start_matches("write:").trim());
        }
        if l.contains("network") && !l.contains("no network") {
            network = true;
        }
    }
    json!({ "read_dirs": read_dirs, "write_dirs": write_dirs, "network": network })
}

pub async fn set_project_skill(
    db: &DashboardDb,
    project_id: &str,
    skill_id: &str,
    enabled: bool,
) -> Result<()> {
    db.link_project_skill(project_id, skill_id, enabled).await?;
    crate::audit::record_audit(
        db,
        crate::audit::AuditEventInput {
            project_id: Some(project_id.into()),
            session_id: None,
            action: if enabled {
                "project_skill_enabled".into()
            } else {
                "project_skill_disabled".into()
            },
            risk: "low".into(),
            detail: json!({ "skill_id": skill_id }),
        },
    )
    .await?;
    Ok(())
}

/// Enable or disable a skill on every registered project (creates links if missing).
pub async fn set_skill_all_projects(
    db: &DashboardDb,
    skill_id: &str,
    enabled: bool,
) -> Result<u64> {
    let projects = db.list_projects().await?;
    let mut updated = 0u64;
    for p in &projects {
        db.link_project_skill(&p.id, skill_id, enabled).await?;
        updated += 1;
    }
    crate::audit::record_audit(
        db,
        crate::audit::AuditEventInput {
            project_id: None,
            session_id: None,
            action: if enabled {
                "skill_enabled_all_projects".into()
            } else {
                "skill_disabled_all_projects".into()
            },
            risk: "low".into(),
            detail: json!({ "skill_id": skill_id, "projects": updated }),
        },
    )
    .await?;
    Ok(updated)
}

pub async fn record_skill_run(
    db: &DashboardDb,
    skill_id: &str,
    project_id: Option<&str>,
    session_id: Option<&str>,
    status: &str,
) -> Result<SkillRunRecord> {
    let id = format!("sr_{}", Uuid::new_v4());
    sqlx::query(
        r#"
        INSERT INTO skill_runs (id, skill_id, project_id, session_id, status, ended_at)
        VALUES (?, ?, ?, ?, ?, CASE WHEN ? IN ('ok', 'failed') THEN datetime('now') ELSE NULL END)
        "#,
    )
    .bind(&id)
    .bind(skill_id)
    .bind(project_id)
    .bind(session_id)
    .bind(status)
    .bind(status)
    .execute(db.pool())
    .await?;
    Ok(SkillRunRecord {
        id,
        skill_id: skill_id.into(),
        project_id: project_id.map(str::to_string),
        session_id: session_id.map(str::to_string),
        status: status.into(),
        started_at: chrono::Utc::now().to_rfc3339(),
        ended_at: if status == "ok" || status == "failed" {
            Some(chrono::Utc::now().to_rfc3339())
        } else {
            None
        },
    })
}

pub async fn list_skill_runs(
    db: &DashboardDb,
    skill_id: &str,
    limit: i64,
) -> Result<Vec<SkillRunRecord>> {
    let rows = sqlx::query(
        r#"
        SELECT id, skill_id, project_id, session_id, status, started_at, ended_at
        FROM skill_runs WHERE skill_id = ? ORDER BY started_at DESC LIMIT ?
        "#,
    )
    .bind(skill_id)
    .bind(limit)
    .fetch_all(db.pool())
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| SkillRunRecord {
            id: r.get("id"),
            skill_id: r.get("skill_id"),
            project_id: r.get("project_id"),
            session_id: r.get("session_id"),
            status: r.get("status"),
            started_at: r.get("started_at"),
            ended_at: r.get("ended_at"),
        })
        .collect())
}
