//! Sandboxed UI-triggered project runs (in-process embedded runtime).

use crate::cancel_ipc::dashboard_state_dir;
use crate::service_governance::is_loopback_host;
use anycode_tools::{SkillCatalog, SkillsGovernance};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[must_use]
pub fn inprocess_triggers_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerRunRequest {
    pub prompt: String,
    #[serde(default = "default_kind")]
    pub kind: String,
    pub goal: Option<String>,
    pub agent: Option<String>,
    #[serde(default)]
    pub skills: Option<Vec<String>>,
}

fn default_kind() -> String {
    "run".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerRunResult {
    pub trigger_id: String,
    pub project_id: String,
    pub kind: String,
    pub pid: u32,
    pub command_preview: String,
    pub log_path: String,
    pub started_at: String,
    pub sandbox_note: String,
}

#[must_use]
pub fn triggers_enabled() -> bool {
    !matches!(
        std::env::var("ANYCODE_DASHBOARD_TRIGGER_RUN").as_deref(),
        Ok("0") | Ok("false") | Ok("off")
    )
}

#[must_use]
pub fn triggers_allowed(host: &str) -> bool {
    if !triggers_enabled() {
        return false;
    }
    if is_loopback_host(host) {
        return true;
    }
    std::env::var("ANYCODE_DASHBOARD_TRIGGER_RUN_REMOTE")
        .ok()
        .is_some_and(|v| v == "1")
}

/// When kind=goal and goal is empty, reuse the prompt as the objective.
pub fn normalize_trigger_request(req: &mut TriggerRunRequest) {
    if req.kind.trim() != "goal" {
        return;
    }
    let has_goal = req
        .goal
        .as_deref()
        .map(str::trim)
        .is_some_and(|s| !s.is_empty());
    if !has_goal {
        req.goal = Some(req.prompt.trim().to_string());
    }
}

fn validate_prompt_text(prompt: &str, vision_count: usize, text_file_count: usize) -> Result<()> {
    let prompt = prompt.trim();
    if prompt.is_empty() && vision_count == 0 && text_file_count == 0 {
        bail!("prompt is required");
    }
    if prompt.len() > 8_000 {
        bail!("prompt too long (max 8000 chars)");
    }
    if prompt.contains('\0') {
        bail!("invalid prompt");
    }
    Ok(())
}

/// Conversation follow-up: allow image/text-only messages when attachments are present.
pub fn validate_conversation_message(
    prompt: &str,
    vision_count: usize,
    text_file_count: usize,
) -> Result<()> {
    validate_prompt_text(prompt, vision_count, text_file_count)
}

pub fn validate_request(req: &TriggerRunRequest) -> Result<()> {
    validate_prompt_text(&req.prompt, 0, 0)?;
    let kind = req.kind.trim();
    if kind != "run" && kind != "goal" {
        bail!("kind must be run or goal");
    }
    if kind == "goal" {
        let goal = req
            .goal
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .context("goal objective is required when kind=goal")?;
        if goal.len() > 2_000 {
            bail!("goal too long (max 2000 chars)");
        }
    }
    if let Some(agent) = req
        .agent
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if agent.len() > 64
            || !agent
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            bail!("invalid agent id");
        }
    }
    validate_skill_ids(req.skills.as_deref())?;
    Ok(())
}

pub fn validate_skill_ids(skills: Option<&[String]>) -> Result<()> {
    let Some(list) = skills else {
        return Ok(());
    };
    if list.len() > 8 {
        bail!("too many skills (max 8)");
    }
    for skill in list {
        let id = skill.trim();
        if id.is_empty() {
            continue;
        }
        if id.len() > 64
            || !id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            bail!("invalid skill id: {id}");
        }
    }
    Ok(())
}

/// Scan skill directories for a project root (same roots as runtime bootstrap).
#[must_use]
pub fn scan_skill_catalog_for_root(project_root: &Path) -> SkillCatalog {
    let mut roots = vec![
        project_root.join("skills"),
        project_root.join(".anycode/skills"),
    ];
    if let Some(h) = dirs::home_dir() {
        roots.push(h.join(".anycode/skills"));
    }
    SkillCatalog::scan(&roots, None, 120_000, false)
}

pub fn skills_governance_from_config(
    config: &anycode_config::Config,
    project_enabled: Option<HashSet<String>>,
) -> SkillsGovernance {
    SkillsGovernance {
        global_allowlist: config.skills.allowlist.clone(),
        agent_allowlists: config.skills.agent_allowlists.clone(),
        project_enabled,
    }
}

pub fn validate_skills_governed(
    skills: Option<&[String]>,
    catalog: &SkillCatalog,
    governance: &SkillsGovernance,
    agent: &str,
) -> Result<()> {
    validate_skill_ids(skills)?;
    let Some(list) = skills else {
        return Ok(());
    };
    for skill in list {
        let id = skill.trim();
        if id.is_empty() {
            continue;
        }
        if !catalog.metas().iter().any(|s| s.id == id) {
            bail!("unknown skill: {id}");
        }
        if !governance.is_allowed(agent, id) {
            bail!("skill not allowed for this project or agent: {id}");
        }
    }
    Ok(())
}

pub async fn validate_trigger_skills_for_project(
    skills: Option<&[String]>,
    agent: Option<&str>,
    project_root: &Path,
) -> Result<()> {
    let config = anycode_config::load_config(None).await?;
    let catalog = scan_skill_catalog_for_root(project_root);
    let project_enabled = anycode_bootstrap::load_project_enabled_skills(project_root).await;
    let governance = skills_governance_from_config(&config, project_enabled);
    let agent_id = agent
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("general-purpose");
    validate_skills_governed(skills, &catalog, &governance, agent_id)
}

#[must_use]
pub fn prompt_with_skills(prompt: &str, skills: Option<&[String]>) -> String {
    let ids: Vec<&str> = skills
        .map(|v| {
            v.iter()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if ids.is_empty() {
        return prompt.trim().to_string();
    }
    format!("[Use skills: {}]\n\n{}", ids.join(", "), prompt.trim())
}

fn triggers_dir() -> PathBuf {
    dashboard_state_dir().join("triggers")
}

pub async fn trigger_run(
    project_id: &str,
    project_root: &Path,
    req: TriggerRunRequest,
    dashboard_session_id: Option<&str>,
    db: Option<&crate::db::DashboardDb>,
) -> Result<TriggerRunResult> {
    trigger_run_inprocess(project_id, project_root, req, dashboard_session_id, db).await
}

async fn trigger_run_inprocess(
    project_id: &str,
    project_root: &Path,
    mut req: TriggerRunRequest,
    dashboard_session_id: Option<&str>,
    db: Option<&crate::db::DashboardDb>,
) -> Result<TriggerRunResult> {
    normalize_trigger_request(&mut req);
    validate_request(&req)?;
    let root = crate::project_root::ensure_project_root(project_root, false)?;
    if !root.is_dir() {
        bail!("project root is not a directory");
    }

    let trigger_id = format!("trg_{}", Uuid::new_v4().simple());
    let dir = triggers_dir();
    std::fs::create_dir_all(&dir)?;
    let log_path = dir.join(format!("{trigger_id}.log"));
    let meta_path = dir.join(format!("{trigger_id}.json"));

    let agent = crate::control::agent_resolve::resolve_web_chat_agent(req.agent.as_deref());
    let prompt = prompt_with_skills(&req.prompt, req.skills.as_deref());
    let command_preview = format!(
        "in-process run -C {} --agent {} {}",
        root.display(),
        agent,
        prompt
    );
    let started_at = chrono::Utc::now().to_rfc3339();
    let pid = std::process::id();

    let result = TriggerRunResult {
        trigger_id: trigger_id.clone(),
        project_id: project_id.to_string(),
        kind: req.kind.clone(),
        pid,
        command_preview: command_preview.clone(),
        log_path: log_path.display().to_string(),
        started_at: started_at.clone(),
        sandbox_note: "In-process AgentRuntime (no CLI subprocess).".into(),
    };
    std::fs::write(
        &meta_path,
        serde_json::to_string_pretty(&serde_json::json!({
            "trigger_id": trigger_id,
            "project_id": project_id,
            "kind": req.kind,
            "pid": pid,
            "command_preview": command_preview,
            "log_path": log_path,
            "started_at": started_at,
        }))?,
    )?;

    let log_path_bg = log_path.clone();
    let session_id = dashboard_session_id.map(str::to_string);
    let db = db.cloned();
    tokio::spawn(async move {
        let run_result: anyhow::Result<()> = async {
            let runtime =
                crate::control::chat_runtime::bootstrap::build_embedded_runtime(None, &root)
                    .await
                    .context("embedded runtime for trigger")?;
            let task = anycode_core::Task {
                id: Uuid::new_v4(),
                agent_type: anycode_core::AgentType::new(agent),
                prompt,
                context: anycode_core::TaskContext {
                    session_id: Uuid::new_v4(),
                    working_directory: root.to_string_lossy().to_string(),
                    environment: Default::default(),
                    user_id: None,
                    system_prompt_append: None,
                    context_injections: vec![],
                    nested_model_override: None,
                    nested_worktree_path: None,
                    nested_worktree_repo_root: None,
                    nested_cancel: None,
                    channel_progress_tx: None,
                    live_trace_tx: None,
                    tool_deny_names: vec![],
                    tool_deny_prefixes: vec![],
                    budget: anycode_core::TaskBudget::default(),
                    user_vision_images: vec![],
                    loop_limits: anycode_core::resolve_agent_loop_limits(None, None),
                    // Structured task-local context replaces the old
                    // process-wide SESSION_ENV write (approval/question scope).
                    chat_turn: session_id
                        .as_ref()
                        .map(|sid| anycode_core::ChatTurnContext {
                            dashboard_session_id: Some(sid.clone()),
                            user_turn_id: None,
                            reply_language: None,
                            host_intent_hint: None,
                        }),
                },
                created_at: chrono::Utc::now(),
            };
            let outcome = runtime.execute_task(task).await;
            let (status, summary) = match &outcome {
                Ok(result) => (
                    crate::control::session_status::session_status_for_task_result(result),
                    crate::control::session_status::session_summary_for_task_result(result)
                        .to_string(),
                ),
                Err(e) if e.is_cooperative_cancel() => (
                    crate::control::session_status::STATUS_CANCELLED,
                    e.to_string(),
                ),
                Err(e) => (crate::control::session_status::STATUS_FAILED, e.to_string()),
            };
            let _ = std::fs::write(&log_path_bg, &summary);
            if let (Some(db), Some(sid)) = (db.as_ref(), session_id.as_deref()) {
                let _ = db.finish_session(sid, status, Some(&summary)).await;
            }
            Ok(())
        }
        .await;
        if let Err(e) = run_result {
            let msg = e.to_string();
            let _ = std::fs::write(&log_path_bg, &msg);
            if let (Some(db), Some(sid)) = (db.as_ref(), session_id.as_deref()) {
                let _ = db.finish_session(sid, "failed", Some(&msg)).await;
            }
        }
    });

    Ok(result)
}

pub fn list_recent_triggers(project_id: &str, limit: usize) -> Vec<TriggerRunResult> {
    let dir = triggers_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return vec![];
    };
    let mut rows: Vec<(std::time::SystemTime, TriggerRunResult)> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| {
            let raw = std::fs::read_to_string(e.path()).ok()?;
            let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
            if v.get("project_id")?.as_str()? != project_id {
                return None;
            }
            let mtime = e.metadata().ok()?.modified().ok()?;
            Some((
                mtime,
                TriggerRunResult {
                    trigger_id: v.get("trigger_id")?.as_str()?.to_string(),
                    project_id: project_id.to_string(),
                    kind: v.get("kind")?.as_str()?.to_string(),
                    pid: v.get("pid")?.as_u64()? as u32,
                    command_preview: v.get("command_preview")?.as_str()?.to_string(),
                    log_path: v
                        .get("log_path")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    started_at: v.get("started_at")?.as_str()?.to_string(),
                    sandbox_note: String::new(),
                },
            ))
        })
        .collect();
    rows.sort_by(|a, b| b.0.cmp(&a.0));
    rows.into_iter().take(limit).map(|(_, r)| r).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn validates_prompt_and_kind() {
        let err = validate_request(&TriggerRunRequest {
            prompt: " ".into(),
            kind: "run".into(),
            goal: None,
            agent: None,
            skills: None,
        })
        .unwrap_err();
        assert!(err.to_string().contains("prompt"));

        validate_request(&TriggerRunRequest {
            prompt: "hello".into(),
            kind: "run".into(),
            goal: None,
            agent: None,
            skills: None,
        })
        .unwrap();
    }

    #[test]
    fn validate_conversation_message_allows_attachments_without_text() {
        validate_conversation_message("", 1, 0).unwrap();
        validate_conversation_message("", 0, 1).unwrap();
        validate_conversation_message("  ", 0, 0).unwrap_err();
    }

    #[test]
    fn prompt_with_skills_prefixes_hint() {
        let out = prompt_with_skills(
            "summarize",
            Some(&["daily-brief".into(), "md-to-pdf".into()]),
        );
        assert!(out.starts_with("[Use skills: daily-brief, md-to-pdf]"));
        assert!(out.contains("summarize"));
    }

    #[test]
    fn triggers_allowed_respects_env() {
        assert!(!triggers_allowed("0.0.0.0"));
        std::env::set_var("ANYCODE_DASHBOARD_TRIGGER_RUN_REMOTE", "1");
        assert!(triggers_allowed("0.0.0.0"));
        std::env::remove_var("ANYCODE_DASHBOARD_TRIGGER_RUN_REMOTE");
    }

    #[test]
    fn goal_defaults_objective_from_prompt() {
        let mut req = TriggerRunRequest {
            prompt: "ship feature".into(),
            kind: "goal".into(),
            goal: None,
            agent: None,
            skills: None,
        };
        normalize_trigger_request(&mut req);
        validate_request(&req).unwrap();
        assert_eq!(req.goal.as_deref(), Some("ship feature"));
    }

    #[test]
    fn validate_skills_governed_rejects_unknown_id() {
        let dir = tempdir().unwrap();
        let known = dir.path().join("skills/known");
        std::fs::create_dir_all(&known).unwrap();
        std::fs::write(
            known.join("SKILL.md"),
            "---\nname: known\ndescription: ok\n---\n",
        )
        .unwrap();
        let catalog = scan_skill_catalog_for_root(dir.path());
        let gov = SkillsGovernance::default();
        let err = validate_skills_governed(
            Some(&["missing-skill".to_string()]),
            &catalog,
            &gov,
            "general-purpose",
        )
        .unwrap_err();
        assert!(err.to_string().contains("unknown skill"));
    }
}
