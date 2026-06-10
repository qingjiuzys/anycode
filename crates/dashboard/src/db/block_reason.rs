//! Human-readable block / stall reasons for sessions.

use crate::db::DashboardDb;
use anyhow::Result;
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct BlockContext {
    pub reason: Option<String>,
    pub kind: Option<String>,
}

pub async fn resolve_block_context(
    db: &DashboardDb,
    session_id: &str,
    status: &str,
    trusted_status: &str,
    summary: &str,
) -> Result<BlockContext> {
    if status == "pending" {
        return Ok(BlockContext {
            reason: Some(
                "Waiting for the CLI task to attach — the subprocess may still be starting.".into(),
            ),
            kind: Some("pending".into()),
        });
    }

    if trusted_status != "blocked" && status != "failed" {
        return Ok(BlockContext::default());
    }

    let session = db.get_session(session_id).await?;
    let (db_summary, metadata) = session
        .as_ref()
        .map(|s| {
            (
                s.summary.clone(),
                serde_json::from_str::<Value>(&s.metadata_json)
                    .unwrap_or(Value::Object(Default::default())),
            )
        })
        .unwrap_or_default();

    let gates = db.list_gates_for_session(session_id).await?;
    if let Some(g) = gates.iter().find(|g| g.required && g.status == "failed") {
        let excerpt = truncate_field(&g.output_excerpt, 160);
        let reason = if excerpt.is_empty() {
            format!("Required gate '{}' failed", g.name)
        } else {
            format!("Required gate '{}' failed: {excerpt}", g.name)
        };
        return Ok(BlockContext {
            reason: Some(reason),
            kind: Some("gate_failed".into()),
        });
    }

    if status == "failed" {
        let effective_summary = if summary.trim().is_empty() {
            db_summary.trim().to_string()
        } else {
            summary.trim().to_string()
        };
        if !effective_summary.is_empty() {
            return Ok(BlockContext {
                reason: Some(effective_summary),
                kind: Some("session_failed".into()),
            });
        }
        if let Some(excerpt) = trigger_log_excerpt(&metadata) {
            return Ok(BlockContext {
                reason: Some(excerpt),
                kind: Some("trigger_failed".into()),
            });
        }
        return Ok(BlockContext {
            reason: Some("Session ended with failure.".into()),
            kind: Some("session_failed".into()),
        });
    }

    let events = db
        .list_session_events(session_id, None, 30, None, None, None)
        .await?;
    if let Some(e) = events.iter().find(|e| e.event_type == "tool_denied") {
        let body = if e.body.trim().is_empty() {
            e.title.clone()
        } else {
            e.body.clone()
        };
        return Ok(BlockContext {
            reason: Some(body),
            kind: Some("tool_denied".into()),
        });
    }

    if trusted_status == "blocked" {
        return Ok(BlockContext {
            reason: Some(
                "Session delivery is blocked — review gates, tool denials, or session detail."
                    .into(),
            ),
            kind: Some("blocked".into()),
        });
    }

    Ok(BlockContext::default())
}

fn trigger_log_excerpt(metadata: &Value) -> Option<String> {
    let path = metadata
        .get("trigger_log_path")
        .or_else(|| metadata.get("log_path"))
        .and_then(|v| v.as_str())
        .filter(|p| !p.trim().is_empty())?;
    read_log_excerpt(Path::new(path))
}

pub(crate) fn read_log_excerpt(path: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(path).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let interesting: Vec<&str> = trimmed
        .lines()
        .map(str::trim)
        .filter(|line| {
            !line.is_empty()
                && (line.contains("error:")
                    || line.contains("Error:")
                    || line.contains("failed")
                    || line.contains("Unrecognized")
                    || line.contains("goal failed"))
        })
        .collect();
    if !interesting.is_empty() {
        return Some(truncate_field(&interesting.join(" · "), 220));
    }
    Some(truncate_field(trimmed, 220))
}

fn truncate_field(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    s.chars().take(max).collect::<String>() + "…"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::CreateSessionRequest;
    use tempfile::tempdir;

    #[tokio::test]
    async fn failed_session_uses_summary() {
        let dir = tempdir().unwrap();
        let db = DashboardDb::open(dir.path().join("block.db"))
            .await
            .unwrap();
        let project = db
            .upsert_project(crate::schema::UpsertProjectRequest {
                root_path: dir.path().display().to_string(),
                name: Some("p".into()),
                description: None,
                create_root: None,
                ..Default::default()
            })
            .await
            .unwrap();
        let session = db
            .create_session(CreateSessionRequest {
                project_id: project.id,
                kind: "run".into(),
                task_id: None,
                title: "t".into(),
                prompt_preview: None,
                agent_type: None,
                model: None,
                metadata_json: None,
            })
            .await
            .unwrap();
        db.finish_session(
            &session.id,
            "failed",
            Some("Failed to start task: spawn error"),
        )
        .await
        .unwrap();
        let ctx = resolve_block_context(
            &db,
            &session.id,
            "failed",
            "blocked",
            "Failed to start task: spawn error",
        )
        .await
        .unwrap();
        assert_eq!(ctx.kind.as_deref(), Some("session_failed"));
        assert!(ctx
            .reason
            .unwrap_or_default()
            .contains("Failed to start task"));
    }

    #[tokio::test]
    async fn failed_session_reads_trigger_log_from_metadata() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("trigger.log");
        std::fs::write(&log_path, "error: Unrecognized option: 'C'\n").unwrap();
        let db = DashboardDb::open(dir.path().join("block.db"))
            .await
            .unwrap();
        let project = db
            .upsert_project(crate::schema::UpsertProjectRequest {
                root_path: dir.path().display().to_string(),
                name: Some("p".into()),
                description: None,
                create_root: None,
                ..Default::default()
            })
            .await
            .unwrap();
        let session = db
            .create_session(CreateSessionRequest {
                project_id: project.id,
                kind: "run".into(),
                task_id: None,
                title: "t".into(),
                prompt_preview: None,
                agent_type: None,
                model: None,
                metadata_json: Some(
                    serde_json::json!({ "trigger_log_path": log_path.display().to_string() })
                        .to_string(),
                ),
            })
            .await
            .unwrap();
        db.finish_session(&session.id, "failed", None)
            .await
            .unwrap();
        let ctx = resolve_block_context(&db, &session.id, "failed", "blocked", "")
            .await
            .unwrap();
        assert_eq!(ctx.kind.as_deref(), Some("trigger_failed"));
        assert!(ctx
            .reason
            .unwrap_or_default()
            .contains("Unrecognized option"));
    }
}
