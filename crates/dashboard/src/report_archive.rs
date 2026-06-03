//! Archive generated reports as dashboard artifacts with evidence links.

use crate::asset_index::add_artifact_link;
use crate::db::DashboardDb;
use crate::schema::{ArtifactRecord, ReportDocument};
use anyhow::Result;

pub async fn archive_report(
    db: &DashboardDb,
    doc: &ReportDocument,
    project_id: &str,
    session_id: Option<&str>,
) -> Result<ArtifactRecord> {
    let path = format!(
        "dashboard/reports/{}/{}/{}.md",
        doc.scope,
        doc.id,
        doc.generated_at.replace(':', "-")
    );
    let title = format!("Report: {}", doc.title);
    let sid = session_id.unwrap_or("");
    let artifact_id = db
        .upsert_artifact(project_id, sid, &path, "report", &title)
        .await?;

    sqlx::query(
        r#"
        UPDATE artifacts SET trust_level = ?, metadata_json = ?, updated_at = datetime('now')
        WHERE id = ?
        "#,
    )
    .bind(&doc.trusted_status)
    .bind(
        serde_json::json!({
            "scope": doc.scope,
            "report_id": doc.id,
            "generated_at": doc.generated_at,
            "summary": doc.summary,
            "source_counts": doc.source_counts,
            "markdown": doc.markdown,
        })
        .to_string(),
    )
    .bind(&artifact_id)
    .execute(db.pool())
    .await?;

    if let Some(sid) = session_id.filter(|s| !s.is_empty()) {
        let _ = add_artifact_link(db, &artifact_id, "session", Some(sid), None).await;
    }
    let _ = add_artifact_link(db, &artifact_id, "project", Some(project_id), None).await;

    list_recent_reports(db, Some(project_id), session_id, 1)
        .await?
        .into_iter()
        .find(|a| a.id == artifact_id)
        .ok_or_else(|| anyhow::anyhow!("report artifact missing after archive"))
}

pub async fn list_recent_reports(
    db: &DashboardDb,
    project_id: Option<&str>,
    session_id: Option<&str>,
    limit: i64,
) -> Result<Vec<ArtifactRecord>> {
    db.list_artifacts(
        project_id,
        session_id,
        Some("report"),
        None,
        false,
        false,
        limit,
    )
    .await
}
