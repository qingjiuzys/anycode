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
) -> Result<Vec<ArtifactRecord>> {
    let mut out = Vec::new();
    let ts = doc.generated_at.replace(':', "-");
    let sid = session_id.unwrap_or("");

    if !doc.markdown.is_empty() {
        let path = format!("dashboard/reports/{}/{}/{}.md", doc.scope, doc.id, ts);
        if let Ok(art) = archive_one(
            db,
            doc,
            project_id,
            sid,
            &path,
            "report",
            &doc.markdown,
            None,
        )
        .await
        {
            out.push(art);
        }
    }

    if let Some(ref html) = doc.html {
        if !html.is_empty() {
            let path = format!("dashboard/reports/{}/{}/{}.html", doc.scope, doc.id, ts);
            if let Ok(art) = archive_one(
                db,
                doc,
                project_id,
                sid,
                &path,
                "report",
                "",
                Some(html.as_str()),
            )
            .await
            {
                out.push(art);
            }
        }
    }

    if out.is_empty() {
        let path = format!("dashboard/reports/{}/{}/{}.md", doc.scope, doc.id, ts);
        let art = archive_one(
            db,
            doc,
            project_id,
            sid,
            &path,
            "report",
            &doc.markdown,
            None,
        )
        .await?;
        out.push(art);
    }

    Ok(out)
}

async fn archive_one(
    db: &DashboardDb,
    doc: &ReportDocument,
    project_id: &str,
    session_id: &str,
    path: &str,
    kind: &str,
    markdown: &str,
    html: Option<&str>,
) -> Result<ArtifactRecord> {
    let title = format!("Report: {}", doc.title);
    let artifact_id = db
        .upsert_artifact(project_id, session_id, path, kind, &title)
        .await?;

    let mut meta = serde_json::json!({
        "scope": doc.scope,
        "report_id": doc.id,
        "generated_at": doc.generated_at,
        "lang": doc.lang,
        "format": doc.format,
        "generation_mode": doc.generation_mode,
        "summary": doc.summary,
        "source_counts": doc.source_counts,
        "highlights": doc.highlights,
        "markdown": markdown,
    });
    if let Some(h) = html {
        meta["html"] = serde_json::Value::String(h.to_string());
    }

    sqlx::query(
        r#"
        UPDATE artifacts SET trust_level = ?, metadata_json = ?, updated_at = datetime('now')
        WHERE id = ?
        "#,
    )
    .bind(&doc.trusted_status)
    .bind(meta.to_string())
    .bind(&artifact_id)
    .execute(db.pool())
    .await?;

    if !session_id.is_empty() {
        let _ = add_artifact_link(db, &artifact_id, "session", Some(session_id), None).await;
    }
    let _ = add_artifact_link(db, &artifact_id, "project", Some(project_id), None).await;

    list_recent_reports(db, Some(project_id), None, 50)
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
        None,
        false,
        false,
        false,
        limit,
    )
    .await
}
