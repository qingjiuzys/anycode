//! Shared Digital Workbench recording for REPL turns.

use anycode_core::{DiskTaskOutput, Task, TaskId};
use anycode_dashboard::{cancel_ipc, DashboardRecorder, RunSessionKind};
use std::future::Future;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;
use uuid::Uuid;

pub type DashboardRecorderHandle = Arc<AsyncMutex<DashboardRecorder>>;

/// Poll dashboard cancel IPC and set the cooperative cancel flag when requested.
pub fn poll_dashboard_cancel_ipc(
    recorder: &DashboardRecorder,
    coop: &std::sync::atomic::AtomicBool,
) {
    let sid = recorder.session_id();
    if cancel_ipc::poll_cancel_requested(sid) {
        coop.store(true, Ordering::Release);
        cancel_ipc::consume_cancel(sid);
    }
}

/// Start a REPL turn session in `projects.db`.
pub async fn begin_repl_turn(
    agent: &str,
    prompt: &str,
    working_dir: &str,
    task_id: Uuid,
    kind: RunSessionKind,
) -> Option<DashboardRecorderHandle> {
    let db = DashboardRecorder::open().await?;
    let task = repl_task(agent, prompt, working_dir, task_id);
    let title = truncate(prompt, 80);
    let rec = DashboardRecorder::begin(db, kind, &task, &title)
        .await
        .ok()?;
    std::env::set_var(
        anycode_dashboard::approval_ipc::SESSION_ENV,
        rec.session_id(),
    );
    Some(Arc::new(AsyncMutex::new(rec)))
}

/// Incremental `output.log` ingest (stream REPL worker tick).
pub async fn tail_tick(recorder: &DashboardRecorderHandle, disk: &DiskTaskOutput, task_id: TaskId) {
    if let Ok(mut r) = recorder.try_lock() {
        r.ingest_delta(disk, task_id).await;
    }
}

/// Poll cancel IPC during dashboard tail (when `coop_cancel` is set).
pub async fn tail_tick_with_cancel(
    recorder: &DashboardRecorderHandle,
    disk: &DiskTaskOutput,
    task_id: TaskId,
    coop_cancel: &Arc<std::sync::atomic::AtomicBool>,
) {
    if let Ok(mut guard) = recorder.try_lock() {
        poll_dashboard_cancel_ipc(&guard, coop_cancel);
        guard.ingest_delta(disk, task_id).await;
    }
}

/// Run `exec` while tailing `output.log` into the dashboard recorder.
pub async fn run_with_dashboard_tail<F, T>(
    recorder: &mut DashboardRecorder,
    disk: &DiskTaskOutput,
    task_id: TaskId,
    coop_cancel: Option<Arc<std::sync::atomic::AtomicBool>>,
    exec: F,
) -> T
where
    F: Future<Output = T>,
{
    let _ = disk.ensure_initialized(task_id);
    tokio::pin!(exec);
    loop {
        tokio::select! {
            res = &mut exec => return res,
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(300)) => {
                if let Some(ref coop) = coop_cancel {
                    poll_dashboard_cancel_ipc(recorder, coop);
                }
                recorder.ingest_delta(disk, task_id).await;
            }
        }
    }
}

pub async fn run_with_dashboard_tail_arc<F, T>(
    recorder: &DashboardRecorderHandle,
    disk: &DiskTaskOutput,
    task_id: TaskId,
    coop_cancel: Arc<std::sync::atomic::AtomicBool>,
    exec: F,
) -> T
where
    F: Future<Output = T>,
{
    let _ = disk.ensure_initialized(task_id);
    tokio::pin!(exec);
    loop {
        tokio::select! {
            res = &mut exec => return res,
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(300)) => {
                tail_tick_with_cancel(recorder, disk, task_id, &coop_cancel).await;
            }
        }
    }
}

pub async fn finish_repl_turn(
    recorder: &mut DashboardRecorder,
    disk: &DiskTaskOutput,
    task_id: TaskId,
    summary: Option<&str>,
) {
    recorder.ingest_full_log(disk, task_id).await;
    recorder.finish_run(disk, task_id, summary).await;
    clear_dashboard_session_env_if_not_sticky();
}

/// Finish and clear dashboard fields on a REPL session.
pub async fn finish_repl_session(
    session: &mut crate::tasks::ReplLineSession,
    disk: &DiskTaskOutput,
    summary: Option<&str>,
) {
    let Some(rec) = session.dashboard_recorder.take() else {
        session.dashboard_task_id = None;
        clear_dashboard_session_env_if_not_sticky();
        return;
    };
    let Some(task_id) = session.dashboard_task_id.take() else {
        clear_dashboard_session_env_if_not_sticky();
        return;
    };
    let mut r = rec.lock().await;
    finish_repl_turn(&mut r, disk, task_id, summary).await;
}

fn repl_task(agent: &str, prompt: &str, working_dir: &str, task_id: Uuid) -> Task {
    use anycode_core::{AgentType, TaskBudget, TaskContext};
    Task {
        id: task_id,
        agent_type: AgentType::new(agent.to_string()),
        prompt: prompt.to_string(),
        context: TaskContext {
            session_id: Uuid::new_v4(),
            working_directory: working_dir.to_string(),
            environment: Default::default(),
            user_id: None,
            system_prompt_append: None,
            context_injections: vec![],
            nested_model_override: None,
            nested_worktree_path: None,
            nested_worktree_repo_root: None,
            nested_cancel: None,
            channel_progress_tx: None,
            tool_deny_names: vec![],
            tool_deny_prefixes: vec![],
            budget: TaskBudget::default(),
        },
        created_at: chrono::Utc::now(),
    }
}

fn clear_dashboard_session_env_if_not_sticky() {
    if std::env::var("ANYCODE_DASHBOARD_SESSION_STICKY")
        .ok()
        .as_deref()
        == Some("1")
    {
        return;
    }
    std::env::remove_var(anycode_dashboard::approval_ipc::SESSION_ENV);
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    s.chars().take(max).collect::<String>() + "…"
}
