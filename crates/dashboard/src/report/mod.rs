//! Project and session report generation (template + optional LLM).

mod builder;
mod compose;
mod html;
mod llm_writer;
mod locale;
mod markdown;
mod snapshot;

use crate::audit::{record_audit, AuditEventInput};
use crate::db::DashboardDb;
use crate::report::compose::{compose_document, ComposeOptions};
use crate::schema::ReportDocument;
use anyhow::{Context, Result};
use chrono::Utc;
use locale::Lang;
use serde_json::json;

pub use builder::{
    aggregate_failures, build_project_snapshot, build_session_snapshot, is_imported_session,
    overall_trusted_status, partition_sessions, trust_counts,
};

#[derive(Debug, Clone)]
pub struct ReportOptions {
    pub events_limit: i64,
    pub artifacts_limit: i64,
    pub lang: Lang,
    /// Skip LLM even when preferences request it (tests / offline).
    pub force_template: bool,
}

impl Default for ReportOptions {
    fn default() -> Self {
        Self {
            events_limit: 50,
            artifacts_limit: 30,
            lang: Lang::En,
            force_template: true,
        }
    }
}

impl ReportOptions {
    pub fn from_lang_param(lang: Option<&str>) -> Self {
        let mut o = Self {
            force_template: false,
            ..Self::default()
        };
        if let Some(l) = lang {
            o.lang = Lang::parse(l);
        }
        o
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

    let generated_at = Utc::now().to_rfc3339();
    let snap = builder::build_project_snapshot(builder::ProjectReportBuildInput {
        lang: opts.lang,
        project_id,
        project_name: &project.name,
        root_path: &project.root_path,
        generated_at: &generated_at,
        sessions: &sessions,
        gates: &gates,
        artifacts: &artifacts,
        recent_failures: &stats.recent_failures,
        events_sample_limit: opts.events_limit,
        events_sampled: events.len() as i64,
    });

    let doc = compose_document(
        snap,
        ComposeOptions {
            force_template: opts.force_template,
        },
    )
    .await;

    if write_audit {
        record_audit(
            db,
            AuditEventInput::low(
                "project_report_generated",
                json!({
                    "project_id": project_id,
                    "format": doc.format,
                    "generation_mode": doc.generation_mode,
                    "lang": doc.lang,
                }),
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

    let snap = builder::build_session_snapshot(builder::SessionReportBuildInput {
        lang: opts.lang,
        session: &session,
        gates: &gates,
        artifacts: &artifacts,
        events: &events,
        events_sample_limit: opts.events_limit,
    });

    let doc = compose_document(
        snap,
        ComposeOptions {
            force_template: opts.force_template,
        },
    )
    .await;

    if write_audit {
        record_audit(
            db,
            AuditEventInput::low(
                "session_report_generated",
                json!({
                    "session_id": session_id,
                    "format": doc.format,
                    "generation_mode": doc.generation_mode,
                    "lang": doc.lang,
                }),
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
    use crate::report::locale::Lang;
    use crate::schema::{CreateSessionRequest, UpsertProjectRequest};

    #[tokio::test]
    async fn project_report_includes_gate_failure() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::DashboardDb::open(dir.path().join("r.db"))
            .await
            .unwrap();
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
        let doc = project_report(
            &db,
            &project.id,
            ReportOptions {
                lang: Lang::En,
                force_template: true,
                ..ReportOptions::default()
            },
            false,
        )
        .await
        .unwrap();
        assert!(doc.markdown.contains("cargo test"));
        assert_eq!(doc.trusted_status, "blocked");
        assert_eq!(doc.source_counts.gates, 1);
        assert!(!doc.highlights.verdict.is_empty());
        assert_eq!(doc.generation_mode, "template");
    }

    #[tokio::test]
    async fn project_report_zh_title() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::DashboardDb::open(dir.path().join("zh.db"))
            .await
            .unwrap();
        let project = db
            .upsert_project(UpsertProjectRequest {
                root_path: "/tmp/zh".into(),
                name: Some("Zh".into()),
                description: None,
                create_root: None,
            })
            .await
            .unwrap();
        let doc = project_report(
            &db,
            &project.id,
            ReportOptions {
                lang: Lang::Zh,
                force_template: true,
                ..ReportOptions::default()
            },
            false,
        )
        .await
        .unwrap();
        assert!(doc.markdown.contains("数字工作台报告"));
        assert_eq!(doc.lang, "zh");
    }

    #[tokio::test]
    async fn project_report_archives_markdown() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::DashboardDb::open(dir.path().join("archive.db"))
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
        let doc = project_report(
            &db,
            &project.id,
            ReportOptions {
                force_template: true,
                ..ReportOptions::default()
            },
            false,
        )
        .await
        .unwrap();
        let archived = crate::report_archive::archive_report(&db, &doc, &project.id, None)
            .await
            .unwrap();
        let archived = archived.first().expect("archived");
        let raw: String = sqlx::query_scalar("SELECT metadata_json FROM artifacts WHERE id = ?")
            .bind(&archived.id)
            .fetch_one(db.pool())
            .await
            .unwrap();
        let meta: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(meta["markdown"].as_str(), Some(doc.markdown.as_str()));
        assert!(meta["highlights"]["verdict"].is_string());
    }

    #[tokio::test]
    async fn session_report_lists_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::DashboardDb::open(dir.path().join("s.db"))
            .await
            .unwrap();
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
        let doc = session_report(
            &db,
            &session.id,
            ReportOptions {
                force_template: true,
                ..ReportOptions::default()
            },
            false,
        )
        .await
        .unwrap();
        assert!(doc.markdown.contains("src/main.rs"));
        assert!(doc.summary.artifacts >= 1);
    }

    #[tokio::test]
    async fn imported_sessions_collapsed_in_doc() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::DashboardDb::open(dir.path().join("imp.db"))
            .await
            .unwrap();
        let project = db
            .upsert_project(UpsertProjectRequest {
                root_path: "/tmp/imp".into(),
                name: Some("Imp".into()),
                description: None,
                create_root: None,
            })
            .await
            .unwrap();
        db.create_session(CreateSessionRequest {
            project_id: project.id.clone(),
            kind: "run".into(),
            task_id: None,
            title: "Imported task uuid-here".into(),
            prompt_preview: None,
            agent_type: None,
            model: None,
            metadata_json: None,
        })
        .await
        .unwrap();
        db.create_session(CreateSessionRequest {
            project_id: project.id.clone(),
            kind: "repl".into(),
            task_id: None,
            title: "Human session".into(),
            prompt_preview: None,
            agent_type: None,
            model: None,
            metadata_json: None,
        })
        .await
        .unwrap();
        let doc = project_report(
            &db,
            &project.id,
            ReportOptions {
                lang: Lang::Zh,
                force_template: true,
                ..ReportOptions::default()
            },
            false,
        )
        .await
        .unwrap();
        assert_eq!(doc.sessions_imported_count, 1);
        assert_eq!(doc.sessions_recent.len(), 1);
        assert_eq!(doc.sessions_recent[0].title, "Human session");
        assert!(doc.markdown.contains("折叠") || doc.markdown.contains("imported"));
    }

    #[tokio::test]
    async fn template_html_when_both_pref() {
        use crate::preferences::save_preferences;
        use crate::schema::DashboardPreferences;

        let dir = tempfile::tempdir().unwrap();
        let prefs_path = dir.path().join("dashboard_preferences.json");
        std::env::set_var(
            "ANYCODE_DASHBOARD_PREFERENCES_PATH",
            prefs_path.display().to_string(),
        );
        save_preferences(&DashboardPreferences {
            host: "127.0.0.1".into(),
            port: 43180,
            db_path: dir.path().join("p.db").display().to_string(),
            asset_read_strict: false,
            report_output_format: "both".into(),
            report_generation_mode: "template".into(),
            updated_at: Utc::now().to_rfc3339(),
        })
        .unwrap();

        let db = crate::db::DashboardDb::open(dir.path().join("p.db"))
            .await
            .unwrap();
        let project = db
            .upsert_project(UpsertProjectRequest {
                root_path: "/tmp/html".into(),
                name: Some("Html".into()),
                description: None,
                create_root: None,
            })
            .await
            .unwrap();
        let doc = project_report(
            &db,
            &project.id,
            ReportOptions {
                force_template: true,
                ..ReportOptions::default()
            },
            false,
        )
        .await
        .unwrap();
        assert!(doc
            .html
            .as_ref()
            .is_some_and(|h| h.contains("<!DOCTYPE html>")));
        std::env::remove_var("ANYCODE_DASHBOARD_PREFERENCES_PATH");
    }
}
