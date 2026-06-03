//! Read-only asset indexing and evidence chain.

use crate::audit::{record_audit, AuditEventInput};
use crate::db::DashboardDb;
use crate::schema::{ArtifactDetail, ArtifactLinkRecord, ArtifactVersionRecord, IndexAssetsResult};
use anyhow::{anyhow, Result};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const MAX_INDEX_BYTES: u64 = 512 * 1024;

pub async fn index_project_assets(db: &DashboardDb, project_id: &str) -> Result<IndexAssetsResult> {
    let project = db
        .get_project(project_id)
        .await?
        .ok_or_else(|| anyhow!("project not found"))?;
    let root = PathBuf::from(&project.root_path);
    if !root.is_dir() {
        return Err(anyhow!("project root missing: {}", project.root_path));
    }

    let job_id = format!("idx_{}", Uuid::new_v4());
    sqlx::query(
        r#"
        INSERT INTO asset_index_jobs (id, project_id, status, started_at)
        VALUES (?, ?, 'running', datetime('now'))
        "#,
    )
    .bind(&job_id)
    .bind(project_id)
    .execute(db.pool())
    .await?;

    let artifacts = db
        .list_artifacts(Some(project_id), None, None, None, false, false, 500)
        .await?;
    let mut indexed = 0usize;
    let mut missing = 0usize;
    let mut skipped = 0usize;
    for art in &artifacts {
        if art.kind != "file" {
            skipped += 1;
            continue;
        }
        let full = root.join(&art.path);
        if !full.is_file() {
            missing += 1;
            continue;
        }
        if let Ok((hash, size, summary)) = file_fingerprint(&full) {
            let ver_id = format!("ver_{}", Uuid::new_v4());
            sqlx::query(
                r#"
                INSERT INTO artifact_versions (id, artifact_id, hash, size_bytes, summary)
                VALUES (?, ?, ?, ?, ?)
                "#,
            )
            .bind(&ver_id)
            .bind(&art.id)
            .bind(&hash)
            .bind(size as i64)
            .bind(&summary)
            .execute(db.pool())
            .await?;
            sqlx::query("UPDATE artifacts SET hash = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(&hash)
                .bind(&art.id)
                .execute(db.pool())
                .await?;
            indexed += 1;
        }
    }

    sqlx::query(
        r#"
        UPDATE asset_index_jobs
        SET status = 'completed', finished_at = datetime('now'), artifacts_indexed = ?
        WHERE id = ?
        "#,
    )
    .bind(indexed as i64)
    .bind(&job_id)
    .execute(db.pool())
    .await?;

    record_audit(
        db,
        AuditEventInput::low(
            "asset_index_completed",
            json!({
                "project_id": project_id,
                "indexed": indexed,
                "missing": missing,
                "skipped": skipped,
            }),
        )
        .with_project(project_id),
    )
    .await?;

    Ok(IndexAssetsResult {
        indexed,
        missing,
        skipped,
        total: artifacts.len(),
        job_id,
    })
}

fn file_fingerprint(path: &Path) -> Result<(String, u64, String)> {
    let meta = std::fs::metadata(path)?;
    let size = meta.len();
    let mut hasher = Sha256::new();
    if size <= MAX_INDEX_BYTES {
        let bytes = std::fs::read(path)?;
        hasher.update(&bytes);
    } else {
        hasher.update(path.to_string_lossy().as_bytes());
        hasher.update(size.to_le_bytes());
    }
    let hash = format!("{:x}", hasher.finalize());
    let summary = format!("{} bytes", size);
    Ok((hash, size, summary))
}

pub async fn get_artifact_detail(
    db: &DashboardDb,
    artifact_id: &str,
) -> Result<Option<ArtifactDetail>> {
    let rows = db
        .list_artifacts(None, None, None, None, false, false, 1000)
        .await?;
    let Some(base) = rows.into_iter().find(|a| a.id == artifact_id) else {
        return Ok(None);
    };

    let versions = sqlx::query(
        r#"
        SELECT id, artifact_id, hash, size_bytes, indexed_at, summary
        FROM artifact_versions WHERE artifact_id = ? ORDER BY indexed_at DESC LIMIT 20
        "#,
    )
    .bind(artifact_id)
    .fetch_all(db.pool())
    .await?
    .into_iter()
    .map(|r| ArtifactVersionRecord {
        id: r.get("id"),
        artifact_id: r.get("artifact_id"),
        hash: r.get("hash"),
        size_bytes: r.get("size_bytes"),
        indexed_at: r.get("indexed_at"),
        summary: r.get("summary"),
    })
    .collect();

    let links = sqlx::query(
        r#"
        SELECT id, artifact_id, link_type, target_id, target_url, created_at
        FROM artifact_links WHERE artifact_id = ? ORDER BY created_at DESC
        "#,
    )
    .bind(artifact_id)
    .fetch_all(db.pool())
    .await?
    .into_iter()
    .map(|r| ArtifactLinkRecord {
        id: r.get("id"),
        artifact_id: r.get("artifact_id"),
        link_type: r.get("link_type"),
        target_id: r.get("target_id"),
        target_url: r.get("target_url"),
        created_at: r.get("created_at"),
    })
    .collect();

    let report_markdown = if base.kind == "report" {
        sqlx::query_scalar::<_, String>("SELECT metadata_json FROM artifacts WHERE id = ?")
            .bind(artifact_id)
            .fetch_optional(db.pool())
            .await?
            .and_then(|raw| {
                serde_json::from_str::<serde_json::Value>(&raw)
                    .ok()
                    .and_then(|v| {
                        v.get("markdown")
                            .and_then(|m| m.as_str())
                            .map(str::to_string)
                    })
            })
    } else {
        None
    };

    Ok(Some(ArtifactDetail {
        artifact: base,
        versions,
        links,
        report_markdown,
    }))
}

pub async fn add_artifact_link(
    db: &DashboardDb,
    artifact_id: &str,
    link_type: &str,
    target_id: Option<&str>,
    target_url: Option<&str>,
) -> Result<ArtifactLinkRecord> {
    let id = format!("lnk_{}", Uuid::new_v4());
    sqlx::query(
        r#"
        INSERT INTO artifact_links (id, artifact_id, link_type, target_id, target_url)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(artifact_id)
    .bind(link_type)
    .bind(target_id)
    .bind(target_url)
    .execute(db.pool())
    .await?;
    Ok(ArtifactLinkRecord {
        id,
        artifact_id: artifact_id.into(),
        link_type: link_type.into(),
        target_id: target_id.map(str::to_string),
        target_url: target_url.map(str::to_string),
        created_at: chrono::Utc::now().to_rfc3339(),
    })
}
