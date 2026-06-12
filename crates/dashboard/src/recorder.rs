//! Bridge `output.log` tailing to SQLite sessions and events.

use crate::db::DashboardDb;
use crate::log_parser::{parse_line, task_end_status};
use crate::notify;
use crate::observability::event_tier::is_index_event_type;
use crate::schema::{CreateSessionRequest, InsertEventRequest, ProjectEvent, UpsertProjectRequest};
use crate::server::default_db_path;
use anycode_core::{DiskTaskOutput, GoalProgress, Task, TaskId};
use anyhow::Result;
use serde_json::Value;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunSessionKind {
    Run,
    Goal,
    Workflow,
    Repl,
    Cron,
}

impl RunSessionKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Run => "run",
            Self::Goal => "goal",
            Self::Workflow => "workflow",
            Self::Repl => "repl",
            Self::Cron => "cron",
        }
    }
}

/// Records task runs into `projects.db` (best-effort; never fails the agent loop).
const ARTIFACT_TOOLS: &[&str] = &["FileWrite", "Edit", "NotebookEdit", "Bash"];

#[derive(Clone)]
pub struct DashboardRecorder {
    db: Arc<DashboardDb>,
    session_id: String,
    project_id: String,
    project_root: PathBuf,
    task_id: String,
    log_offset: u64,
    pending_tool_name: Option<String>,
    pending_tool_json: Option<String>,
    started_at: SystemTime,
}

impl DashboardRecorder {
    #[must_use]
    pub fn enabled() -> bool {
        !matches!(
            std::env::var("ANYCODE_DASHBOARD_RECORD").as_deref(),
            Ok("0") | Ok("false") | Ok("off")
        )
    }

    pub async fn open() -> Option<Arc<DashboardDb>> {
        if !Self::enabled() {
            return None;
        }
        let path = std::env::var("ANYCODE_DASHBOARD_DB")
            .map(PathBuf::from)
            .unwrap_or_else(|_| default_db_path());
        match DashboardDb::open(&path).await {
            Ok(db) => Some(Arc::new(db)),
            Err(e) => {
                tracing::debug!(error = %e, "dashboard recorder: db open skipped");
                None
            }
        }
    }

    pub async fn begin(
        db: Arc<DashboardDb>,
        kind: RunSessionKind,
        task: &Task,
        title_hint: &str,
    ) -> Result<Self> {
        let root = std::fs::canonicalize(&task.context.working_directory)
            .unwrap_or_else(|_| PathBuf::from(&task.context.working_directory));
        let root_str = root.to_string_lossy().to_string();
        let project = db
            .upsert_project(UpsertProjectRequest {
                root_path: root_str,
                name: None,
                description: None,
                create_root: None,
                ..Default::default()
            })
            .await?;
        let metadata_json = session_metadata_json(kind, task);
        let prompt_preview = truncate(&task.prompt, 240);
        let agent_type = task.agent_type.as_str().to_string();

        let session = if let Ok(pre_id) = std::env::var(crate::ipc::approval_ipc::SESSION_ENV) {
            let pre_id = pre_id.trim().to_string();
            if !pre_id.is_empty() {
                if let Some(existing) = db.get_session(&pre_id).await? {
                    db.attach_task_to_session(
                        &pre_id,
                        &task.id.to_string(),
                        Some(agent_type.as_str()),
                        Some(prompt_preview.as_str()),
                    )
                    .await?;
                    existing
                } else {
                    db.create_or_get_session_by_task_id(CreateSessionRequest {
                        project_id: project.id.clone(),
                        kind: kind.as_str().into(),
                        task_id: Some(task.id.to_string()),
                        title: truncate(title_hint, 120),
                        prompt_preview: Some(prompt_preview.clone()),
                        agent_type: Some(agent_type.clone()),
                        model: None,
                        metadata_json: metadata_json.clone(),
                    })
                    .await?
                }
            } else {
                db.create_or_get_session_by_task_id(CreateSessionRequest {
                    project_id: project.id.clone(),
                    kind: kind.as_str().into(),
                    task_id: Some(task.id.to_string()),
                    title: truncate(title_hint, 120),
                    prompt_preview: Some(prompt_preview.clone()),
                    agent_type: Some(agent_type.clone()),
                    model: None,
                    metadata_json: metadata_json.clone(),
                })
                .await?
            }
        } else {
            db.create_or_get_session_by_task_id(CreateSessionRequest {
                project_id: project.id.clone(),
                kind: kind.as_str().into(),
                task_id: Some(task.id.to_string()),
                title: truncate(title_hint, 120),
                prompt_preview: Some(prompt_preview.clone()),
                agent_type: Some(agent_type.clone()),
                model: None,
                metadata_json,
            })
            .await?
        };

        if !task.prompt.trim().is_empty() {
            let existing_prompt = sqlx::query_scalar::<_, i64>(
                r#"
                SELECT COUNT(*) FROM project_events
                WHERE session_id = ?
                  AND event_type = 'user_prompt'
                  AND (task_id = ? OR body = ?)
                "#,
            )
            .bind(&session.id)
            .bind(task.id.to_string())
            .bind(truncate(&task.prompt, 8000))
            .fetch_one(db.pool())
            .await
            .unwrap_or(0);
            if existing_prompt == 0 {
                if let Ok(evt) = db
                    .insert_event(InsertEventRequest {
                        project_id: project.id.clone(),
                        session_id: Some(session.id.clone()),
                        task_id: Some(task.id.to_string()),
                        agent_id: None,
                        event_type: "user_prompt".into(),
                        severity: Some("info".into()),
                        title: "User prompt".into(),
                        body: Some(truncate(&task.prompt, 8000)),
                        payload: None,
                    })
                    .await
                {
                    Self::notify_sse(evt);
                }
            }
        }
        if let Err(e) = crate::cancel_ipc::register_active(&session.id, &task.id.to_string()) {
            tracing::debug!(error = %e, "dashboard cancel_ipc register skipped");
        }
        Ok(Self {
            db,
            session_id: session.id,
            project_id: project.id,
            project_root: root,
            task_id: task.id.to_string(),
            log_offset: 0,
            pending_tool_name: None,
            pending_tool_json: None,
            started_at: SystemTime::now(),
        })
    }

    async fn scan_workspace_artifacts(&self) {
        if let Err(e) = crate::workspace_scan::scan_and_register_artifacts(
            &self.db,
            &self.project_id,
            &self.session_id,
            &self.project_root,
            self.started_at,
        )
        .await
        {
            tracing::debug!(error = %e, session_id = %self.session_id, "workspace artifact scan");
        }
    }

    pub async fn ingest_delta(&mut self, disk: &DiskTaskOutput, task_id: TaskId) {
        let Ok((delta, new_offset)) = disk.read_delta(task_id, self.log_offset, 64 * 1024) else {
            return;
        };
        if delta.is_empty() {
            return;
        }
        self.log_offset = new_offset;
        if let Err(e) = self.ingest_text(&delta).await {
            tracing::debug!(error = %e, "dashboard ingest_delta");
        }
    }

    pub async fn ingest_full_log(&mut self, disk: &DiskTaskOutput, task_id: TaskId) {
        let path = disk.output_path(task_id);
        let Ok(content) = std::fs::read_to_string(&path) else {
            return;
        };
        self.log_offset = content.len() as u64;
        if let Err(e) = self.ingest_text(&content).await {
            tracing::debug!(error = %e, "dashboard ingest_full_log");
        }
    }

    async fn ingest_text(&mut self, text: &str) -> Result<()> {
        let mut dedup = HashSet::new();
        for line in text.lines() {
            if line.starts_with('{')
                && parse_line(line).is_none()
                && self.pending_tool_name.is_some()
            {
                self.pending_tool_json = Some(line.to_string());
                continue;
            }
            let Some(parsed) = parse_line(line) else {
                continue;
            };
            if parsed.event_type == "tool_call_input" {
                self.pending_tool_name = parsed
                    .payload
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
                self.pending_tool_json = None;
            }
            if parsed.event_type == "tool_call_end" {
                self.maybe_record_artifact(&parsed).await;
                self.pending_tool_name = None;
                self.pending_tool_json = None;
            }
            if parsed.event_type == "llm_request_start" {
                if let Some(model) = parsed.payload.get("model").and_then(|v| v.as_str()) {
                    let _ = self.db.update_session_model(&self.session_id, model).await;
                }
            }
            let is_gate = parsed.event_type == "gate";
            if is_gate {
                let name = parsed
                    .payload
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("gate");
                let status = parsed
                    .payload
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let cmd = parsed
                    .payload
                    .get("command")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let _ = self
                    .db
                    .upsert_gate(
                        &self.project_id,
                        &self.session_id,
                        name,
                        cmd,
                        status,
                        true,
                        &parsed.body,
                    )
                    .await;
            }
            let key = format!("{}:{}", parsed.event_type, line);
            if !dedup.insert(key) {
                continue;
            }
            // Gate rows live in `gates`; skip duplicate timeline events.
            if is_gate {
                continue;
            }
            if !is_index_event_type(&parsed.event_type) {
                continue;
            }
            if let Ok(evt) = self
                .db
                .insert_event(InsertEventRequest {
                    project_id: self.project_id.clone(),
                    session_id: Some(self.session_id.clone()),
                    task_id: Some(self.task_id.clone()),
                    agent_id: None,
                    event_type: parsed.event_type,
                    severity: Some(parsed.severity),
                    title: parsed.title,
                    body: Some(parsed.body),
                    payload: Some(parsed.payload),
                })
                .await
            {
                Self::notify_sse(evt);
            }
        }
        Ok(())
    }

    fn notify_sse(evt: ProjectEvent) {
        notify::spawn_publish_event(evt);
    }

    async fn maybe_record_artifact(&self, _parsed: &crate::log_parser::ParsedLine) {
        let Some(tool) = self.pending_tool_name.as_deref() else {
            return;
        };
        if !ARTIFACT_TOOLS.contains(&tool) {
            return;
        }
        let Some(json) = self.pending_tool_json.as_deref() else {
            return;
        };
        if tool == "Bash" {
            for rel in extract_bash_output_paths(json) {
                self.record_artifact_rel(&rel, "file").await;
            }
            return;
        }
        let Some(rel) = extract_artifact_path(json) else {
            return;
        };
        let kind = if tool == "NotebookEdit" {
            "notebook"
        } else {
            "file"
        };
        self.record_artifact_rel(&rel, kind).await;
    }

    async fn record_artifact_rel(&self, rel: &str, kind: &str) {
        let path = self.project_root.join(rel).to_string_lossy().to_string();
        let title = Path::new(rel)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(rel)
            .to_string();
        let _ = self
            .db
            .upsert_artifact(&self.project_id, &self.session_id, &path, kind, &title)
            .await;
    }

    pub async fn finish_with_status(&self, status: &str, summary: Option<&str>) {
        self.scan_workspace_artifacts().await;
        if let Err(e) = self
            .db
            .finish_session(&self.session_id, status, summary)
            .await
        {
            tracing::debug!(error = %e, "dashboard finish_with_status");
        }
        crate::cancel_ipc::unregister_active(&self.session_id);
    }

    pub async fn finish_run(&self, disk: &DiskTaskOutput, task_id: TaskId, summary: Option<&str>) {
        let path = disk.output_path(task_id);
        let status = std::fs::read_to_string(&path)
            .ok()
            .and_then(|c| task_end_status(&c.lines().collect::<Vec<_>>()))
            .unwrap_or_else(|| "completed".into());
        self.scan_workspace_artifacts().await;
        if let Err(e) = self
            .db
            .finish_session(&self.session_id, &status, summary)
            .await
        {
            tracing::debug!(error = %e, "dashboard finish_run");
        }
        crate::cancel_ipc::unregister_active(&self.session_id);
    }

    pub async fn finish_goal(
        &mut self,
        disk: &DiskTaskOutput,
        task_id: TaskId,
        progress: &GoalProgress,
        done_when: Option<&str>,
        working_dir: &Path,
    ) {
        self.ingest_full_log(disk, task_id).await;
        self.record_goal_gates(progress, done_when, working_dir)
            .await;
        let status = if progress.completed {
            "completed"
        } else {
            "failed"
        };
        let summary = progress
            .last_error
            .as_deref()
            .or(progress.last_output.as_deref());
        if let Err(e) = self
            .db
            .merge_session_metadata(
                &self.session_id,
                &serde_json::json!({ "goal_attempts": progress.attempts }),
            )
            .await
        {
            tracing::debug!(error = %e, "dashboard goal_attempts metadata");
        }
        self.scan_workspace_artifacts().await;
        if let Err(e) = self
            .db
            .finish_session(&self.session_id, status, summary)
            .await
        {
            tracing::debug!(error = %e, "dashboard finish_goal");
        }
        crate::cancel_ipc::unregister_active(&self.session_id);
    }

    async fn record_goal_gates(
        &self,
        progress: &GoalProgress,
        done_when: Option<&str>,
        working_dir: &Path,
    ) {
        if let Some(marker) = done_when.filter(|m| !m.is_empty()) {
            let (status, excerpt) = if progress.completed {
                ("passed", "")
            } else {
                (
                    "failed",
                    progress
                        .last_error
                        .as_deref()
                        .unwrap_or("README marker or objective not satisfied"),
                )
            };
            let _ = self
                .db
                .upsert_gate(
                    &self.project_id,
                    &self.session_id,
                    &format!("README `{marker}`"),
                    "readme_marker",
                    status,
                    true,
                    excerpt,
                )
                .await;
        }

        let flutter_scope =
            working_dir.join("pubspec.yaml").is_file() || working_dir.join("test").is_dir();
        if flutter_scope {
            let err = progress.last_error.as_deref().unwrap_or("");
            if progress.completed {
                for (name, cmd) in [
                    ("flutter analyze", "flutter analyze"),
                    ("flutter test", "flutter test"),
                ] {
                    let _ = self
                        .db
                        .upsert_gate(
                            &self.project_id,
                            &self.session_id,
                            name,
                            cmd,
                            "passed",
                            true,
                            "",
                        )
                        .await;
                }
            } else {
                if err.contains("flutter analyze") {
                    let _ = self
                        .db
                        .upsert_gate(
                            &self.project_id,
                            &self.session_id,
                            "flutter analyze",
                            "flutter analyze",
                            "failed",
                            true,
                            &truncate(err, 2000),
                        )
                        .await;
                }
                if err.contains("flutter test") {
                    let _ = self
                        .db
                        .upsert_gate(
                            &self.project_id,
                            &self.session_id,
                            "flutter test",
                            "flutter test",
                            "failed",
                            true,
                            &truncate(err, 2000),
                        )
                        .await;
                }
            }
        }

        if let Some(marker) = done_when.filter(|m| {
            let lower = m.to_lowercase();
            lower.contains("browser")
                || lower.contains("manual")
                || lower.contains("visual")
                || lower.contains("screenshot")
        }) {
            let (status, excerpt) = if progress.completed {
                ("passed", "Manual/browser verification recorded")
            } else {
                (
                    "failed",
                    progress
                        .last_error
                        .as_deref()
                        .unwrap_or("Manual verification not satisfied"),
                )
            };
            let _ = self
                .db
                .upsert_gate(
                    &self.project_id,
                    &self.session_id,
                    "Manual / browser verification",
                    "manual_verification",
                    status,
                    true,
                    &truncate(excerpt, 2000),
                )
                .await;
            let _ = marker;
        }
    }

    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    #[must_use]
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    /// Workflow step marker for the session timeline (not a verification gate).
    pub async fn log_workflow_step(&self, step_id: &str, title: &str, status: &str) {
        let payload = serde_json::json!({ "step_id": step_id, "status": status });
        if let Ok(evt) = self
            .db
            .insert_event(InsertEventRequest {
                project_id: self.project_id.clone(),
                session_id: Some(self.session_id.clone()),
                task_id: Some(self.task_id.clone()),
                agent_id: None,
                event_type: "workflow_step".into(),
                severity: Some(if status == "failed" {
                    "error".into()
                } else {
                    "info".into()
                }),
                title: title.to_string(),
                body: None,
                payload: Some(payload),
            })
            .await
        {
            Self::notify_sse(evt);
        }
    }
}

fn extract_artifact_path(json: &str) -> Option<String> {
    let v: Value = serde_json::from_str(json).ok()?;
    for key in ["file_path", "path", "notebook_path", "target_file"] {
        if let Some(p) = v.get(key).and_then(|x| x.as_str()) {
            if !p.is_empty() {
                return Some(p.to_string());
            }
        }
    }
    None
}

fn extract_bash_output_paths(json: &str) -> Vec<String> {
    let Ok(v) = serde_json::from_str::<Value>(json) else {
        return Vec::new();
    };
    let cmd = v
        .get("command")
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .trim();
    if cmd.is_empty() {
        return Vec::new();
    }
    let mut paths = Vec::new();
    if let Some(idx) = cmd.rfind(">> ") {
        push_shell_path(&mut paths, cmd[idx + 3..].trim());
    } else if let Some(idx) = cmd.rfind("> ") {
        push_shell_path(&mut paths, cmd[idx + 2..].trim());
    }
    for prefix in ["touch ", "tee ", "cp ", "mv "] {
        if let Some(rest) = cmd.strip_prefix(prefix) {
            if let Some(last) = rest.split_whitespace().last() {
                push_shell_path(&mut paths, last);
            }
        }
    }
    paths
}

fn push_shell_path(out: &mut Vec<String>, raw: &str) {
    let path = raw
        .trim_matches('"')
        .trim_matches('\'')
        .trim_end_matches(';')
        .trim();
    if path.is_empty() || path.starts_with('-') || path.contains('$') {
        return;
    }
    if !out.iter().any(|p| p == path) {
        out.push(path.to_string());
    }
}

fn session_metadata_json(kind: RunSessionKind, task: &Task) -> Option<String> {
    if kind == RunSessionKind::Cron {
        let mut meta = serde_json::json!({
            "correlation_id": task.context.session_id.to_string(),
            "source": "cron",
        });
        if let Some(job_id) = task.prompt.strip_prefix("Cron ") {
            let job_id = job_id.split_whitespace().next().unwrap_or(job_id);
            meta["cron_job_id"] = serde_json::Value::String(job_id.to_string());
        }
        return serde_json::to_string(&meta).ok();
    }
    None
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    s.chars().take(max).collect::<String>() + "…"
}

#[cfg(test)]
mod tests {
    use super::extract_artifact_path;

    #[test]
    fn extracts_file_path_from_tool_json() {
        let json = r#"{"file_path":"lib/main.dart"}"#;
        assert_eq!(
            extract_artifact_path(json).as_deref(),
            Some("lib/main.dart")
        );
    }

    #[test]
    fn extracts_bash_redirect_path() {
        let paths = super::extract_bash_output_paths(r#"{"command":"echo hi >> out/report.md"}"#);
        assert!(paths.iter().any(|p| p.contains("report.md")));
    }
}
