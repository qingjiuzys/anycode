//! Shared helpers for web-chat conversation handlers.

use crate::audit;
use crate::db::DashboardDb;
use crate::schema::InsertEventRequest;
use serde_json::json;
use std::path::{Path, PathBuf};

#[must_use]
pub fn dashboard_loopback_url(host: &str, port: u16) -> String {
    let host = match host {
        "0.0.0.0" | "::" => "127.0.0.1",
        other => other,
    };
    format!("http://{host}:{port}")
}

pub async fn ensure_chat_project_root(
    db: &DashboardDb,
    project_id: &str,
    session_id: Option<&str>,
    root_path: &Path,
    source: &str,
) -> Result<(PathBuf, bool), String> {
    let (root, created_root) =
        crate::project_root::ensure_project_root_for_chat(root_path).map_err(|e| e.to_string())?;
    if created_root {
        let _ = db
            .insert_event(InsertEventRequest {
                project_id: project_id.to_string(),
                session_id: session_id.map(str::to_string),
                task_id: None,
                agent_id: None,
                event_type: "project_root_created".into(),
                severity: Some("info".into()),
                title: "Project root created".into(),
                body: Some(root.display().to_string()),
                payload: Some(json!({ "source": source })),
            })
            .await;
        let _ = audit::record_audit(
            db,
            audit::AuditEventInput {
                project_id: Some(project_id.to_string()),
                session_id: session_id.map(str::to_string),
                action: "project_root_created".into(),
                risk: "medium".into(),
                detail: json!({
                    "root_path": root.display().to_string(),
                    "source": source,
                }),
            },
        )
        .await;
    }
    Ok((root, created_root))
}
