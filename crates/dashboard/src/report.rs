//! Project and session Markdown report generation.

use crate::audit::{record_audit, AuditEventInput};
use crate::db::DashboardDb;
use crate::schema::{ReportDocument, ReportSourceCounts, ReportSummary};
use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::json;

#[derive(Debug, Clone)]
pub struct ReportOptions {
    pub events_limit: i64,
    pub artifacts_limit: i64,
}

impl Default for ReportOptions {
    fn default() -> Self {
        Self {
            events_limit: 50,
            artifacts_limit: 30,
        }
    }
}

pub async fn project_report(
    db: &DashboardDb,
    project_id: &str,
    opts: ReportOptions,
    write_audit: bool,
) -> Result<ReportDocument> {
    let project = db
        .get_project(project_id)
        .await?
        .context("project not found")?;
    let sessions = db.list_sessions(project_id, 100).await?;
    let events = db
        .list_project_events(project_id, opts.events_limit, None, None, None)
        .await?;
    let gates = db.list_gates_for_project(project_id).await?;
    let artifacts = db
        .list_artifacts(
            Some(project_id),
            None,
            None,
            None,
            false,
            false,
            opts.artifacts_limit,
        )
        .await?;
    let stats = db.get_project_stats(project_id).await?;

    let failed_gates = gates.iter().filter(|g| g.status == "failed").count() as i64;
    let source_counts = ReportSourceCounts {
        sessions: sessions.len() as i64,
        events: events.len() as i64,
        gates: gates.len() as i64,
        artifacts: artifacts.len() as i64,
    };
    let trusted_status = if gates.iter().any(|g| g.required && g.status == "failed") {
        "blocked"
    } else if sessions.iter().any(|s| s.trusted_status == "verified") {
        "verified"
    } else {
        "unverified"
    };

    let summary = ReportSummary {
        sessions: sessions.len() as i64,
        events: events.len() as i64,
        failed_gates,
        artifacts: artifacts.len() as i64,
    };

    let mut md = String::new();
    md.push_str("# anycode Digital Workbench Report\n\n");
    md.push_str("**Scope:** project\n");
    md.push_str(&format!(
        "**Project:** {} (`{}`)\n",
        project.name, project.id
    ));
    md.push_str(&format!("**Root:** `{}`\n", project.root_path));
    md.push_str(&format!("**Generated:** {}\n\n", Utc::now().to_rfc3339()));

    md.push_str("## Summary\n\n");
    md.push_str(&format!(
        "- Sessions: {}\n- Events (recent): {}\n- Failed gates: {}\n- Artifacts: {}\n\n",
        summary.sessions, summary.events, summary.failed_gates, summary.artifacts
    ));

    md.push_str("## Trust Status\n\n");
    md.push_str(&format!("Overall trust: **{trusted_status}**\n\n"));

    md.push_str("## Sessions\n\n");
    if sessions.is_empty() {
        md.push_str("_No sessions recorded._\n\n");
    } else {
        for s in sessions.iter().take(20) {
            md.push_str(&format!(
                "- **{}** · {} · {} · trusted: {}\n",
                s.title, s.kind, s.status, s.trusted_status
            ));
        }
        md.push('\n');
    }

    md.push_str("## Required Gates\n\n");
    if gates.is_empty() {
        md.push_str("_No gate records._\n\n");
    } else {
        for g in &gates {
            let req = if g.required { "required" } else { "optional" };
            md.push_str(&format!(
                "- **{}** · {} · {} · {}\n",
                g.name,
                g.status,
                req,
                g.output_excerpt.chars().take(80).collect::<String>()
            ));
        }
        md.push('\n');
    }

    md.push_str("## Recent Failures and Warnings\n\n");
    if stats.recent_failures.is_empty() {
        md.push_str("_None in recent events._\n\n");
    } else {
        for f in &stats.recent_failures {
            md.push_str(&format!(
                "- **{}** · {} · {}\n",
                f.title, f.event_type, f.occurred_at
            ));
        }
        md.push('\n');
    }

    md.push_str("## Artifacts\n\n");
    if artifacts.is_empty() {
        md.push_str("_No artifacts tracked._\n\n");
    } else {
        for a in &artifacts {
            md.push_str(&format!(
                "- `{}` · {} · trust: {}\n",
                a.path, a.kind, a.trust_level
            ));
        }
        md.push('\n');
    }

    md.push_str("## Reproduction Commands\n\n");
    md.push_str(&format!(
        "```bash\ncd \"{}\"\nanycode run --help\nanycode dashboard --open\n```\n\n",
        project.root_path
    ));

    let generated_at = Utc::now().to_rfc3339();
    let doc = ReportDocument {
        scope: "project".into(),
        id: project_id.to_string(),
        title: project.name.clone(),
        format: "markdown".into(),
        generated_at: generated_at.clone(),
        trusted_status: trusted_status.into(),
        markdown: md,
        summary,
        source_counts,
    };

    if write_audit {
        record_audit(
            db,
            AuditEventInput::low(
                "project_report_generated",
                json!({ "project_id": project_id, "format": "markdown" }),
            )
            .with_project(project_id),
        )
        .await?;
        let _ = crate::report_archive::archive_report(db, &doc, project_id, None).await;
        let _ = crate::notifications::emit_local_log(
            db,
            Some(project_id),
            None,
            "project_report_generated",
            json!({ "project_id": project_id }),
        )
        .await;
    }

    Ok(doc)
}

pub async fn session_report(
    db: &DashboardDb,
    session_id: &str,
    opts: ReportOptions,
    write_audit: bool,
) -> Result<ReportDocument> {
    let session = db
        .get_session(session_id)
        .await?
        .context("session not found")?;
    let events = db
        .list_session_events(session_id, None, opts.events_limit, None, None, None)
        .await?;
    let gates = db.list_gates_for_session(session_id).await?;
    let artifacts = db
        .list_artifacts(
            None,
            Some(session_id),
            None,
            None,
            false,
            false,
            opts.artifacts_limit,
        )
        .await?;

    let failed_gates = gates.iter().filter(|g| g.status == "failed").count() as i64;
    let summary = ReportSummary {
        sessions: 1,
        events: events.len() as i64,
        failed_gates,
        artifacts: artifacts.len() as i64,
    };
    let source_counts = ReportSourceCounts {
        sessions: 1,
        events: events.len() as i64,
        gates: gates.len() as i64,
        artifacts: artifacts.len() as i64,
    };

    let mut md = String::new();
    md.push_str("# anycode Digital Workbench Report\n\n");
    md.push_str("**Scope:** session\n");
    md.push_str(&format!(
        "**Session:** {} (`{}`)\n",
        session.title, session.id
    ));
    md.push_str(&format!(
        "**Project:** {} · **Kind:** {} · **Status:** {}\n",
        session.project_name, session.kind, session.status
    ));
    md.push_str(&format!("**Generated:** {}\n\n", Utc::now().to_rfc3339()));

    md.push_str("## Summary\n\n");
    md.push_str(&format!(
        "- Trusted: {}\n- Events: {}\n- Failed gates: {}\n- Artifacts: {}\n\n",
        session.trusted_status, summary.events, summary.failed_gates, summary.artifacts
    ));

    md.push_str("## Trust Status\n\n");
    md.push_str(&format!(
        "Session trusted_status: **{}**\n\n",
        session.trusted_status
    ));

    if !session.prompt_preview.is_empty() {
        md.push_str("## Prompt Preview\n\n");
        md.push_str(&format!("```\n{}\n```\n\n", session.prompt_preview));
    }

    if !session.summary.is_empty() {
        md.push_str("## Summary Text\n\n");
        md.push_str(&format!("{}\n\n", session.summary));
    }

    md.push_str("## Gates\n\n");
    if gates.is_empty() {
        md.push_str("_No gate records._\n\n");
    } else {
        for g in &gates {
            md.push_str(&format!(
                "- **{}** · {} · required: {}\n",
                g.name, g.status, g.required
            ));
        }
        md.push('\n');
    }

    md.push_str("## Recent Events\n\n");
    if events.is_empty() {
        md.push_str("_No events._\n\n");
    } else {
        for e in events.iter().rev().take(30) {
            md.push_str(&format!(
                "- **{}** · {} · {} · {}\n",
                e.title, e.event_type, e.severity, e.occurred_at
            ));
        }
        md.push('\n');
    }

    md.push_str("## Artifacts\n\n");
    if artifacts.is_empty() {
        md.push_str("_No artifacts._\n\n");
    } else {
        for a in &artifacts {
            md.push_str(&format!(
                "- `{}` · {} · trust: {}\n",
                a.path, a.kind, a.trust_level
            ));
        }
        md.push('\n');
    }

    md.push_str("## Reproduction Commands\n\n");
    md.push_str("```bash\nanycode dashboard --open\n");
    md.push_str(&format!("# session id: {}\n", session.id));
    if let Some(ref tid) = session.task_id {
        md.push_str(&format!("# task id: {tid}\n"));
    }
    md.push_str("```\n\n");

    let generated_at = Utc::now().to_rfc3339();
    let doc = ReportDocument {
        scope: "session".into(),
        id: session_id.to_string(),
        title: session.title.clone(),
        format: "markdown".into(),
        generated_at,
        trusted_status: session.trusted_status.clone(),
        markdown: md,
        summary,
        source_counts,
    };

    if write_audit {
        record_audit(
            db,
            AuditEventInput::low(
                "session_report_generated",
                json!({ "session_id": session_id, "format": "markdown" }),
            )
            .with_project(&session.project_id)
            .with_session(session_id),
        )
        .await?;
        let _ =
            crate::report_archive::archive_report(db, &doc, &session.project_id, Some(session_id))
                .await;
        let _ = crate::notifications::emit_local_log(
            db,
            Some(&session.project_id),
            Some(session_id),
            "session_report_generated",
            json!({ "session_id": session_id }),
        )
        .await;
    }

    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{CreateSessionRequest, UpsertProjectRequest};
    use tempfile::tempdir;

    #[tokio::test]
    async fn project_report_includes_gate_failure() {
        let dir = tempdir().unwrap();
        let db = DashboardDb::open(dir.path().join("r.db")).await.unwrap();
        let project = db
            .upsert_project(UpsertProjectRequest {
                root_path: "/tmp/report-test".into(),
                name: Some("ReportTest".into()),
                description: None,
                create_root: None,
            })
            .await
            .unwrap();
        let session = db
            .create_session(CreateSessionRequest {
                project_id: project.id.clone(),
                kind: "goal".into(),
                task_id: Some("t1".into()),
                title: "gate test".into(),
                prompt_preview: None,
                agent_type: None,
                model: None,
                metadata_json: None,
            })
            .await
            .unwrap();
        db.upsert_gate(
            &project.id,
            &session.id,
            "cargo test",
            "cargo test",
            "failed",
            true,
            "test failed",
        )
        .await
        .unwrap();
        let doc = project_report(&db, &project.id, ReportOptions::default(), false)
            .await
            .unwrap();
        assert!(doc.markdown.contains("cargo test"));
        assert_eq!(doc.trusted_status, "blocked");
        assert_eq!(doc.source_counts.gates, 1);
    }

    #[tokio::test]
    async fn project_report_archives_markdown() {
        let dir = tempdir().unwrap();
        let db = DashboardDb::open(dir.path().join("archive.db"))
            .await
            .unwrap();
        let project = db
            .upsert_project(UpsertProjectRequest {
                root_path: "/tmp/archive-test".into(),
                name: Some("ArchiveTest".into()),
                description: None,
                create_root: None,
            })
            .await
            .unwrap();
        let doc = project_report(&db, &project.id, ReportOptions::default(), false)
            .await
            .unwrap();
        let archived = crate::report_archive::archive_report(&db, &doc, &project.id, None)
            .await
            .unwrap();
        let raw: String = sqlx::query_scalar("SELECT metadata_json FROM artifacts WHERE id = ?")
            .bind(&archived.id)
            .fetch_one(db.pool())
            .await
            .unwrap();
        let meta: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(meta["markdown"].as_str(), Some(doc.markdown.as_str()));
        assert!(meta["source_counts"]["gates"].is_number());
    }

    #[tokio::test]
    async fn session_report_lists_artifacts() {
        let dir = tempdir().unwrap();
        let db = DashboardDb::open(dir.path().join("s.db")).await.unwrap();
        let project = db
            .upsert_project(UpsertProjectRequest {
                root_path: "/tmp/art".into(),
                name: Some("Art".into()),
                description: None,
                create_root: None,
            })
            .await
            .unwrap();
        let session = db
            .create_session(CreateSessionRequest {
                project_id: project.id.clone(),
                kind: "run".into(),
                task_id: None,
                title: "run".into(),
                prompt_preview: None,
                agent_type: None,
                model: None,
                metadata_json: None,
            })
            .await
            .unwrap();
        db.upsert_artifact(&project.id, &session.id, "src/main.rs", "file", "main.rs")
            .await
            .unwrap();
        let doc = session_report(&db, &session.id, ReportOptions::default(), false)
            .await
            .unwrap();
        assert!(doc.markdown.contains("src/main.rs"));
        assert!(doc.summary.artifacts >= 1);
    }
}
