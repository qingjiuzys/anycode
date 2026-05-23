//! DB maintenance, backup suggestions, and migration observability.

use crate::data_health::global_health;
use crate::db::DashboardDb;
use crate::schema::{DbOperations, DbTableStat, MigrationInfo};
use crate::service_governance::suggest_backup_path;
use anyhow::Result;
use sqlx::Row;
use std::path::Path;

pub async fn db_operations(db: &DashboardDb) -> Result<DbOperations> {
    let path = db.path().display().to_string();
    let size = std::fs::metadata(db.path()).map(|m| m.len()).unwrap_or(0);
    let migrations = list_migrations(db).await?;
    let tables = table_stats(db).await?;
    let backup_suggestion = suggest_backup_path(db.path()).display().to_string();
    let health = global_health(db).await?;

    let events_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project_events")
        .fetch_one(db.pool())
        .await?;
    let artifacts_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM artifacts")
        .fetch_one(db.pool())
        .await?;
    let audit_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM auth_events WHERE source = 'dashboard'")
            .fetch_one(db.pool())
            .await?;

    let mut growth_warnings = Vec::new();
    if events_count > 100_000 {
        growth_warnings.push(format!(
            "project_events has {events_count} rows — consider archival"
        ));
    }
    if artifacts_count > 10_000 {
        growth_warnings.push(format!("artifacts has {artifacts_count} rows"));
    }
    if audit_count > 5_000 {
        growth_warnings.push(format!("dashboard audit events: {audit_count}"));
    }
    if size > 100 * 1024 * 1024 {
        growth_warnings.push(format!(
            "DB size {:.1} MB — VACUUM may help",
            size as f64 / 1_048_576.0
        ));
    }

    Ok(DbOperations {
        db_path: path,
        db_size_bytes: size,
        migrations,
        tables,
        backup_suggestion,
        growth_warnings,
        health_status: health.status,
        generated_at: chrono::Utc::now().to_rfc3339(),
    })
}

async fn list_migrations(db: &DashboardDb) -> Result<Vec<MigrationInfo>> {
    let rows = sqlx::query(
        r#"
        SELECT version, description AS name, installed_on AS applied_at
        FROM _sqlx_migrations WHERE success = 1 ORDER BY version
        "#,
    )
    .fetch_all(db.pool())
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| MigrationInfo {
            version: r.get("version"),
            name: r.get("name"),
            applied_at: r.get::<String, _>("applied_at"),
        })
        .collect())
}

async fn table_stats(db: &DashboardDb) -> Result<Vec<DbTableStat>> {
    let tables = [
        "projects",
        "sessions",
        "project_events",
        "gates",
        "artifacts",
        "auth_events",
        "skills",
        "automation_policies",
    ];
    let mut out = Vec::new();
    for t in tables {
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {t}"))
            .fetch_one(db.pool())
            .await
            .unwrap_or(0);
        out.push(DbTableStat {
            name: t.into(),
            row_count: count,
        });
    }
    Ok(out)
}

pub fn backup_db(src: &Path, dest: &Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(src, dest)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn db_ops_empty() {
        let dir = tempdir().unwrap();
        let db = DashboardDb::open(dir.path().join("o.db")).await.unwrap();
        let ops = db_operations(&db).await.unwrap();
        assert!(!ops.migrations.is_empty());
    }
}
