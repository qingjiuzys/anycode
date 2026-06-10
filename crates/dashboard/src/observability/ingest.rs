//! Import historical `~/.anycode/tasks/*/output.log` into SQLite.
//!
//! Deprecated for production: sessions are created by [`crate::recorder::DashboardRecorder`]
//! at task start. Bulk log import is retained for tests only.

use crate::db::DashboardDb;
use crate::log_parser::{parse_line, task_end_status};
use crate::recorder::RunSessionKind;
use crate::schema::{CreateSessionRequest, InsertEventRequest, UpsertProjectRequest};
use anyhow::Result;
use std::path::{Path, PathBuf};

const MAX_TASKS: usize = 30;

pub async fn ingest_recent_disk_tasks(
    db: &DashboardDb,
    tasks_root: &Path,
    workspace_paths: &[String],
) -> Result<usize> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Ok(read) = std::fs::read_dir(tasks_root) {
        for ent in read.flatten() {
            let path = ent.path();
            if path.join("output.log").is_file() {
                dirs.push(path);
            }
        }
    }
    dirs.sort_by_key(|p| {
        std::fs::metadata(p.join("output.log"))
            .and_then(|m| m.modified())
            .ok()
    });
    dirs.reverse();
    dirs.truncate(MAX_TASKS);

    let mut imported = 0usize;
    for dir in dirs {
        let task_id = dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if task_id.is_empty() {
            continue;
        }
        let log_path = dir.join("output.log");
        let content = match std::fs::read_to_string(&log_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let root = infer_project_root(&content, workspace_paths);
        let Some(root) = root else { continue };

        let project = db
            .upsert_project(UpsertProjectRequest {
                root_path: root.clone(),
                name: None,
                description: None,
                create_root: None,
                ..Default::default()
            })
            .await?;

        let existing: Option<String> =
            sqlx::query_scalar("SELECT id FROM sessions WHERE task_id = ? LIMIT 1")
                .bind(&task_id)
                .fetch_optional(db.pool())
                .await?;
        if existing.is_some() {
            continue;
        }

        let agent_type = content
            .lines()
            .find_map(|l| {
                parse_line(l).and_then(|p| {
                    (p.event_type == "task_start")
                        .then(|| p.payload.get("agent_type")?.as_str().map(str::to_string))
                })
            })
            .flatten()
            .unwrap_or_else(|| "general".into());

        let kind = if content.contains("workflow:") || content.contains("## Workflow") {
            RunSessionKind::Workflow
        } else if content.contains("[Scheduled cron") {
            RunSessionKind::Cron
        } else if agent_type == "goal" {
            RunSessionKind::Goal
        } else {
            RunSessionKind::Run
        };

        let title = format!("Imported task {task_id}");
        let session = db
            .create_session(CreateSessionRequest {
                project_id: project.id.clone(),
                kind: kind.as_str().into(),
                task_id: Some(task_id.clone()),
                title,
                prompt_preview: None,
                agent_type: Some(agent_type),
                model: extract_model(&content),
                metadata_json: None,
            })
            .await?;

        for line in content.lines() {
            if let Some(parsed) = parse_line(line) {
                let _ = db
                    .insert_event(InsertEventRequest {
                        project_id: project.id.clone(),
                        session_id: Some(session.id.clone()),
                        task_id: Some(task_id.clone()),
                        agent_id: None,
                        event_type: parsed.event_type,
                        severity: Some(parsed.severity),
                        title: parsed.title,
                        body: Some(parsed.body),
                        payload: Some(parsed.payload),
                    })
                    .await;
            }
        }

        let status = task_end_status(&content.lines().collect::<Vec<_>>())
            .unwrap_or_else(|| "completed".into());
        db.finish_session(&session.id, &status, None).await?;
        imported += 1;
    }
    Ok(imported)
}

fn extract_model(content: &str) -> Option<String> {
    content.lines().find_map(|l| {
        let p = parse_line(l)?;
        if p.event_type == "llm_request_start" {
            p.payload
                .get("model")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        } else {
            None
        }
    })
}

fn infer_project_root(log: &str, workspace_paths: &[String]) -> Option<String> {
    for line in log.lines().take(800) {
        for token in line
            .split('"')
            .filter(|s| s.contains('/') && !s.starts_with("http"))
        {
            for root in workspace_paths {
                let root_path = Path::new(root);
                if token.starts_with('/') {
                    if token.starts_with(root) {
                        return Some(root.clone());
                    }
                    continue;
                }
                if root_path.join(token).exists() {
                    return Some(root.clone());
                }
                if let Some(first) = token.split('/').next() {
                    if root_path.join(first).exists() {
                        return Some(root.clone());
                    }
                }
            }
            if token.starts_with('/') {
                let path = Path::new(token);
                if path.is_dir() {
                    return Some(path.to_string_lossy().to_string());
                }
                if let Some(parent) = path.parent().filter(|p| p.is_dir()) {
                    return Some(parent.to_string_lossy().to_string());
                }
            }
        }
    }
    workspace_paths.first().cloned()
}
