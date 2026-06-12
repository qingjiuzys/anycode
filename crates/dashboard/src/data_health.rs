//! Read-only database and project health diagnostics.

use crate::db::DashboardDb;
use crate::schema::{DataHealth, HealthCheckItem};
use anyhow::Result;
use chrono::Utc;
use std::path::Path;

pub async fn global_health(db: &DashboardDb) -> Result<DataHealth> {
    let mut checks = Vec::new();
    let db_path = db.path().display().to_string();
    let db_size_bytes = std::fs::metadata(db.path()).map(|m| m.len()).unwrap_or(0);

    if !db.path().is_file() {
        checks.push(HealthCheckItem {
            id: "db_missing".into(),
            name: "Database file".into(),
            status: "error".into(),
            message: "projects.db not found".into(),
            count: 1,
            project_id: None,
            session_id: None,
        });
    } else {
        checks.push(HealthCheckItem {
            id: "db_exists".into(),
            name: "Database file".into(),
            status: "ok".into(),
            message: "Database file present".into(),
            count: 1,
            project_id: None,
            session_id: None,
        });
    }

    let orphan_events: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM project_events e
        WHERE e.session_id IS NOT NULL
          AND NOT EXISTS (SELECT 1 FROM sessions s WHERE s.id = e.session_id)
        "#,
    )
    .fetch_one(db.pool())
    .await
    .unwrap_or(0);
    if orphan_events > 0 {
        checks.push(HealthCheckItem {
            id: "orphan_event_sessions".into(),
            name: "Orphan event session refs".into(),
            status: "warn".into(),
            message: "Events reference missing sessions".into(),
            count: orphan_events,
            project_id: None,
            session_id: None,
        });
    }

    let orphan_artifacts: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM artifacts a
        WHERE a.session_id IS NOT NULL
          AND NOT EXISTS (SELECT 1 FROM sessions s WHERE s.id = a.session_id)
        "#,
    )
    .fetch_one(db.pool())
    .await
    .unwrap_or(0);
    if orphan_artifacts > 0 {
        checks.push(HealthCheckItem {
            id: "orphan_artifact_sessions".into(),
            name: "Orphan artifact session refs".into(),
            status: "warn".into(),
            message: "Artifacts reference missing sessions".into(),
            count: orphan_artifacts,
            project_id: None,
            session_id: None,
        });
    }

    let stale_running: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM sessions
        WHERE status = 'running'
          AND datetime(started_at) < datetime('now', '-1 day')
        "#,
    )
    .fetch_one(db.pool())
    .await
    .unwrap_or(0);
    if stale_running > 0 {
        checks.push(HealthCheckItem {
            id: "stale_running_sessions".into(),
            name: "Stale running sessions".into(),
            status: "warn".into(),
            message: "Sessions running >24h without end".into(),
            count: stale_running,
            project_id: None,
            session_id: None,
        });
    }

    let missing_roots = missing_project_roots(db).await?;
    if missing_roots.count > 0 {
        checks.push(missing_roots);
    }

    let status = aggregate_status(&checks);
    Ok(DataHealth {
        status,
        db_path,
        db_size_bytes,
        generated_at: Utc::now().to_rfc3339(),
        checks,
    })
}

pub async fn project_health(db: &DashboardDb, project_id: &str) -> Result<DataHealth> {
    let mut checks = Vec::new();
    let db_path = db.path().display().to_string();
    let db_size_bytes = std::fs::metadata(db.path()).map(|m| m.len()).unwrap_or(0);

    if let Some(p) = db.get_project(project_id).await? {
        if !Path::new(&p.root_path).exists() {
            checks.push(HealthCheckItem {
                id: "missing_project_root".into(),
                name: "Project root".into(),
                status: "warn".into(),
                message: format!("Root path missing: {}", p.root_path),
                count: 1,
                project_id: Some(project_id.to_string()),
                session_id: None,
            });
        } else {
            checks.push(HealthCheckItem {
                id: "project_root_ok".into(),
                name: "Project root".into(),
                status: "ok".into(),
                message: "Project root exists".into(),
                count: 1,
                project_id: Some(project_id.to_string()),
                session_id: None,
            });
        }
    }

    let failed_gates: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM gates
        WHERE project_id = ? AND required = 1 AND status = 'failed'
        "#,
    )
    .bind(project_id)
    .fetch_one(db.pool())
    .await
    .unwrap_or(0);
    if failed_gates > 0 {
        checks.push(HealthCheckItem {
            id: "failed_required_gates".into(),
            name: "Failed required gates".into(),
            status: "warn".into(),
            message: "Required gates failed for this project".into(),
            count: failed_gates,
            project_id: Some(project_id.to_string()),
            session_id: None,
        });
    }

    let blocked_sessions: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM sessions
        WHERE project_id = ?
          AND trusted_status = 'blocked'
          AND status != 'failed'
        "#,
    )
    .bind(project_id)
    .fetch_one(db.pool())
    .await
    .unwrap_or(0);
    if blocked_sessions > 0 {
        checks.push(HealthCheckItem {
            id: "blocked_sessions".into(),
            name: "Blocked sessions".into(),
            status: "warn".into(),
            message: "Sessions with blocked trust status".into(),
            count: blocked_sessions,
            project_id: Some(project_id.to_string()),
            session_id: None,
        });
    }

    let missing_files = missing_artifact_files(db, project_id).await?;
    if missing_files.count > 0 {
        checks.push(missing_files);
    }

    let stale_index = stale_asset_index(db, project_id).await?;
    if stale_index.count > 0 {
        checks.push(stale_index);
    }

    let status = aggregate_status(&checks);
    Ok(DataHealth {
        status: if checks.is_empty() {
            "ok".into()
        } else {
            status
        },
        db_path,
        db_size_bytes,
        generated_at: Utc::now().to_rfc3339(),
        checks,
    })
}

async fn missing_project_roots(db: &DashboardDb) -> Result<HealthCheckItem> {
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT id, root_path FROM projects ORDER BY updated_at DESC")
            .fetch_all(db.pool())
            .await?;
    let mut missing = 0i64;
    let mut first_id = None;
    for (id, root) in rows {
        if !Path::new(&root).exists() {
            missing += 1;
            if first_id.is_none() {
                first_id = Some(id);
            }
        }
    }
    Ok(HealthCheckItem {
        id: "missing_project_root".into(),
        name: "Missing project roots".into(),
        status: if missing > 0 { "warn" } else { "ok" }.into(),
        message: if missing > 0 {
            "One or more project root paths no longer exist".into()
        } else {
            "All project roots exist".into()
        },
        count: missing,
        project_id: first_id,
        session_id: None,
    })
}

async fn missing_artifact_files(db: &DashboardDb, project_id: &str) -> Result<HealthCheckItem> {
    let project = db
        .get_project(project_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("project not found"))?;
    let root = Path::new(&project.root_path);
    let artifacts = db
        .list_artifacts(
            Some(project_id),
            None,
            Some("file"),
            None,
            None,
            false,
            false,
            false,
            500,
        )
        .await?;
    let mut missing = 0i64;
    for art in &artifacts {
        if !root.join(&art.path).is_file() {
            missing += 1;
        }
    }
    Ok(HealthCheckItem {
        id: "missing_artifact_files".into(),
        name: "Missing artifact files".into(),
        status: if missing > 0 { "warn" } else { "ok" }.into(),
        message: if missing > 0 {
            "Indexed file artifacts no longer exist on disk".into()
        } else {
            "All file artifacts present on disk".into()
        },
        count: missing,
        project_id: Some(project_id.to_string()),
        session_id: None,
    })
}

async fn stale_asset_index(db: &DashboardDb, project_id: &str) -> Result<HealthCheckItem> {
    let file_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM artifacts WHERE project_id = ? AND kind = 'file'")
            .bind(project_id)
            .fetch_one(db.pool())
            .await
            .unwrap_or(0);
    let indexed_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(DISTINCT artifact_id) FROM artifact_versions v
        JOIN artifacts a ON a.id = v.artifact_id
        WHERE a.project_id = ? AND a.kind = 'file'
        "#,
    )
    .bind(project_id)
    .fetch_one(db.pool())
    .await
    .unwrap_or(0);
    let stale = (file_count - indexed_count).max(0);
    Ok(HealthCheckItem {
        id: "stale_asset_index".into(),
        name: "Stale asset index".into(),
        status: if stale > 0 { "warn" } else { "ok" }.into(),
        message: if stale > 0 {
            "Some file artifacts have no indexed version — run index-assets".into()
        } else {
            "Asset index up to date".into()
        },
        count: stale,
        project_id: Some(project_id.to_string()),
        session_id: None,
    })
}

fn aggregate_status(checks: &[HealthCheckItem]) -> String {
    if checks.iter().any(|c| c.status == "error") {
        "error".into()
    } else if checks.iter().any(|c| c.status == "warn") {
        "warn".into()
    } else {
        "ok".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::UpsertProjectRequest;
    use tempfile::tempdir;

    #[tokio::test]
    async fn empty_db_health_ok() {
        let dir = tempdir().unwrap();
        let db = DashboardDb::open(dir.path().join("h.db")).await.unwrap();
        let h = global_health(&db).await.unwrap();
        assert_eq!(h.status, "ok");
    }

    #[tokio::test]
    async fn missing_root_warns() {
        let dir = tempdir().unwrap();
        let db = DashboardDb::open(dir.path().join("h2.db")).await.unwrap();
        let p = db
            .upsert_project(UpsertProjectRequest {
                root_path: "/nonexistent/path/for/health".into(),
                name: Some("Ghost".into()),
                description: None,
                create_root: None,
                ..Default::default()
            })
            .await
            .unwrap();
        let h = project_health(&db, &p.id).await.unwrap();
        assert_eq!(h.status, "warn");
    }
}
