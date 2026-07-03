//! Load project-scoped enabled skills from the dashboard SQLite DB.

use sha2::{Digest, Sha256};
use sqlx::sqlite::SqlitePoolOptions;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

fn default_db_path() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".anycode")
        .join("projects.db")
}

fn project_id_for_root(root_path: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(root_path.as_bytes());
    let digest = hasher.finalize();
    let hex = digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    format!("proj_{}", &hex[..32])
}

fn normalize_project_root(root: &Path) -> Option<PathBuf> {
    if root.is_dir() {
        return std::fs::canonicalize(root).ok();
    }
    let absolute = if root.is_absolute() {
        root.to_path_buf()
    } else {
        std::env::current_dir().ok()?.join(root)
    };
    Some(absolute)
}

async fn open_default_db_if_exists() -> Option<sqlx::SqlitePool> {
    let path = default_db_path();
    if !path.is_file() {
        return None;
    }
    let url = format!("sqlite:{}?mode=ro", path.display());
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .ok()
}

/// Enabled skill ids for `cwd` when the dashboard SQLite DB exists.
pub async fn load_project_enabled_skills(cwd: &Path) -> Option<HashSet<String>> {
    let pool = open_default_db_if_exists().await?;
    let root = normalize_project_root(cwd)?.to_string_lossy().to_string();
    let project_id = project_id_for_root(&root);
    let rows = sqlx::query_scalar::<_, String>(
        r#"
        SELECT ps.skill_id
        FROM project_skills ps
        WHERE ps.project_id = ? AND ps.enabled = 1
        "#,
    )
    .bind(&project_id)
    .fetch_all(&pool)
    .await
    .ok()?;
    if rows.is_empty() {
        return None;
    }
    Some(rows.into_iter().collect())
}
