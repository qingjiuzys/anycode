//! Read project-scoped enabled skills from the dashboard SQLite DB (sync helper for CLI).

use crate::db::DashboardDb;
use crate::project_root::{normalize_project_root, project_id_for_root};
use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;

/// Enabled skill ids for the project matching `cwd`, when `projects.db` exists.
pub async fn enabled_skill_ids_for_path(
    db: &DashboardDb,
    cwd: &Path,
) -> Result<Option<HashSet<String>>> {
    let root = match normalize_project_root(cwd) {
        Ok(r) => r.to_string_lossy().to_string(),
        Err(_) => return Ok(None),
    };
    let project_id = project_id_for_root(&root);
    let rows = sqlx::query_scalar::<_, String>(
        r#"
        SELECT ps.skill_id
        FROM project_skills ps
        WHERE ps.project_id = ? AND ps.enabled = 1
        "#,
    )
    .bind(&project_id)
    .fetch_all(db.pool())
    .await?;
    if rows.is_empty() {
        return Ok(None);
    }
    Ok(Some(rows.into_iter().collect()))
}

/// Open DB at default path if present; otherwise `None`.
pub async fn open_default_db_if_exists() -> Result<Option<DashboardDb>> {
    let path = crate::server::default_db_path();
    if !path.is_file() {
        return Ok(None);
    }
    Ok(Some(DashboardDb::open(&path).await?))
}
