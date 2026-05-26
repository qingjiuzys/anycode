//! Sandboxed UI-triggered `anycode run` / goal subprocess (loopback-only by default).

use crate::cancel_ipc::dashboard_state_dir;
use crate::service_governance::is_loopback_host;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerRunRequest {
    pub prompt: String,
    #[serde(default = "default_kind")]
    pub kind: String,
    pub goal: Option<String>,
    pub agent: Option<String>,
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

pub fn validate_request(req: &TriggerRunRequest) -> Result<()> {
    let prompt = req.prompt.trim();
    if prompt.is_empty() {
        bail!("prompt is required");
    }
    if prompt.len() > 8_000 {
        bail!("prompt too long (max 8000 chars)");
    }
    if prompt.contains('\0') {
        bail!("invalid prompt");
    }
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
    Ok(())
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths).find_map(|dir| {
        let candidate = dir.join(name);
        candidate.is_file().then_some(candidate)
    })
}

fn exe_looks_like_anycode_cli(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n == "anycode" || n.starts_with("anycode-"))
}

fn release_binary_sibling(exe: &Path) -> Option<PathBuf> {
    let parent = exe.parent()?;
    if parent.ends_with("debug") {
        let candidate = parent.parent()?.join("release").join("anycode");
        return candidate.is_file().then_some(candidate);
    }
    None
}

/// Default `ANYCODE_BIN` for UI triggers when unset (prefer release sibling in dev trees).
pub fn init_default_anycode_bin() {
    if std::env::var_os("ANYCODE_BIN").is_some() {
        return;
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(release) = release_binary_sibling(&exe) {
            tracing::info!(
                path = %release.display(),
                "ANYCODE_BIN defaulted to release sibling for UI triggers"
            );
            std::env::set_var("ANYCODE_BIN", release);
        }
    }
}

/// Resolve the `anycode` CLI for UI-triggered subprocesses.
///
/// Prefers `ANYCODE_BIN`, then `current_exe` when it is the real CLI, then `PATH`.
pub fn resolve_anycode_binary() -> Result<PathBuf> {
    if let Ok(raw) = std::env::var("ANYCODE_BIN") {
        let path = PathBuf::from(raw.trim());
        if path.is_file() {
            return Ok(path);
        }
        bail!("ANYCODE_BIN points to a missing file: {}", path.display());
    }

    if let Ok(exe) = std::env::current_exe() {
        if exe_looks_like_anycode_cli(&exe) {
            return Ok(exe);
        }
    }

    if let Some(found) = find_on_path("anycode") {
        return Ok(found);
    }

    if let Ok(exe) = std::env::current_exe() {
        return Ok(exe);
    }

    bail!("anycode CLI not found; set ANYCODE_BIN or start dashboard via the anycode binary")
}

pub fn build_argv(project_root: &Path, req: &TriggerRunRequest) -> Result<Vec<String>> {
    let mut req = req.clone();
    normalize_trigger_request(&mut req);
    validate_request(&req)?;
    let exe = resolve_anycode_binary().context("resolve anycode binary")?;
    let mut argv = vec![exe.display().to_string(), "run".into()];
    argv.push("-C".into());
    argv.push(project_root.display().to_string());
    if let Some(agent) = req
        .agent
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        argv.push("--agent".into());
        argv.push(agent.to_string());
    }
    if req.kind == "goal" {
        if let Some(goal) = req.goal.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            argv.push("--goal".into());
            argv.push(goal.to_string());
        }
    }
    argv.push(req.prompt.trim().to_string());
    Ok(argv)
}

fn triggers_dir() -> PathBuf {
    dashboard_state_dir().join("triggers")
}

fn spawn_attach_watch(
    db: crate::db::DashboardDb,
    session_id: String,
    log_path: PathBuf,
    mut child: tokio::process::Child,
) {
    tokio::spawn(async move {
        let _ = child.wait().await;
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let Ok(Some(sess)) = db.get_session(&session_id).await else {
            return;
        };
        if sess.status != "pending" {
            return;
        }
        if sess
            .task_id
            .as_deref()
            .is_some_and(|task_id| !task_id.trim().is_empty())
        {
            return;
        }
        let excerpt = crate::db::read_log_excerpt(&log_path);
        let summary = excerpt.unwrap_or_else(|| {
            format!(
                "Task subprocess exited before attaching. See trigger log: {}",
                log_path.display()
            )
        });
        let _ = db
            .finish_session(&session_id, "failed", Some(&summary))
            .await;
    });
}

pub async fn trigger_run(
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

    let argv = build_argv(&root, &req)?;
    let trigger_id = format!("trg_{}", Uuid::new_v4().simple());
    let dir = triggers_dir();
    std::fs::create_dir_all(&dir)?;
    let log_path = dir.join(format!("{trigger_id}.log"));
    let meta_path = dir.join(format!("{trigger_id}.json"));

    let exe = PathBuf::from(&argv[0]);
    let mut cmd = Command::new(&exe);
    for arg in argv.iter().skip(1) {
        cmd.arg(arg);
    }
    let log_file = std::fs::File::create(&log_path).context("create trigger log")?;
    let err_file = log_file.try_clone().context("clone trigger log fd")?;
    cmd.current_dir(&root)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(err_file))
        .env("ANYCODE_DASHBOARD_RECORD", "1")
        .env("ANYCODE_MEMORY_ATTACH", "shared");
    if let Some(session_id) = dashboard_session_id.filter(|s| !s.trim().is_empty()) {
        cmd.env(crate::ipc::approval_ipc::SESSION_ENV, session_id);
    }

    let mut child = cmd.spawn().context("spawn anycode run")?;
    let pid = child.id().unwrap_or(0);
    if let (Some(db), Some(session_id)) =
        (db, dashboard_session_id.filter(|s| !s.trim().is_empty()))
    {
        spawn_attach_watch(db.clone(), session_id.to_string(), log_path.clone(), child);
    } else {
        tokio::spawn(async move {
            let _ = child.wait().await;
        });
    }
    let command_preview = argv.join(" ");
    let started_at = chrono::Utc::now().to_rfc3339();
    let result = TriggerRunResult {
        trigger_id: trigger_id.clone(),
        project_id: project_id.to_string(),
        kind: req.kind.clone(),
        pid,
        command_preview: command_preview.clone(),
        log_path: log_path.display().to_string(),
        started_at: started_at.clone(),
        sandbox_note: "Detached subprocess in project root. Sensitive tools use the Web approval inbox when approval is required; watch Conversations for the new session.".into(),
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
        })
        .unwrap_err();
        assert!(err.to_string().contains("prompt"));

        validate_request(&TriggerRunRequest {
            prompt: "hello".into(),
            kind: "run".into(),
            goal: None,
            agent: None,
        })
        .unwrap();
    }

    #[test]
    fn builds_run_argv() {
        let dir = tempdir().unwrap();
        let argv = build_argv(
            dir.path(),
            &TriggerRunRequest {
                prompt: "fix tests".into(),
                kind: "run".into(),
                goal: None,
                agent: Some("general".into()),
            },
        )
        .unwrap();
        assert!(argv.iter().any(|a| a == "run"));
        assert!(!argv.iter().any(|a| a == "-I"));
        assert!(argv.iter().any(|a| a == "--agent"));
        assert!(argv.last().is_some_and(|a| a == "fix tests"));
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
        };
        normalize_trigger_request(&mut req);
        validate_request(&req).unwrap();
        assert_eq!(req.goal.as_deref(), Some("ship feature"));
    }

    #[test]
    fn resolve_binary_honors_anycode_bin() {
        let dir = tempdir().unwrap();
        let fake = dir.path().join("anycode");
        std::fs::write(&fake, b"").unwrap();
        std::env::set_var("ANYCODE_BIN", fake.to_str().unwrap());
        let resolved = resolve_anycode_binary().unwrap();
        assert_eq!(resolved, fake);
        std::env::remove_var("ANYCODE_BIN");
    }
}
