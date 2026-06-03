//! `anycode run` / 单次任务执行与 goal 循环（与 REPL 解耦）。

use crate::app_config::Config;
use crate::dashboard_record::DashboardRecorderHandle;
use crate::i18n::{tr, tr_args};
use crate::task_builders::build_headless_task;
use anycode_agent::AgentRuntime;
use anycode_core::prelude::*;
use anycode_dashboard::{DashboardRecorder, RunSessionKind};
use fluent_bundle::FluentArgs;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

use super::tasks_sink::ReplSink;
use super::workflow_exec::run_workflow_path;

/// Optional knobs for headless single-task runs (cron, workflows).
#[derive(Clone, Default)]
pub(crate) struct RunTaskOptions {
    pub session_id: Option<Uuid>,
    pub tool_profile: Option<String>,
    pub tool_allowlist: Option<Vec<String>>,
    pub budget: TaskBudget,
    /// When set, tail `output.log` into this recorder and do not open a nested session.
    pub dashboard_parent: Option<DashboardRecorderHandle>,
    /// Session kind when creating a new dashboard session (default `Run`; cron uses `Cron`).
    pub dashboard_kind: Option<RunSessionKind>,
    /// Optional session title in the workbench (e.g. `Cron job_id`).
    pub dashboard_title: Option<String>,
}

/// `execute_task` 已将总结逐行写入磁盘 tail；若再原样 `sink.line(output)` 会与流式 stdout 叠一份。
fn streamed_log_already_contains_output(streamed: &str, output: &str) -> bool {
    let o = output.trim();
    if o.is_empty() {
        return false;
    }
    let s: String = streamed.chars().filter(|c| *c != '\r').collect();
    let o_norm: String = o.chars().filter(|c| *c != '\r').collect();
    s.contains(o_norm.trim_end())
}

pub(crate) async fn run_task(
    config: crate::app_config::Config,
    agent_type: Option<String>,
    mode: Option<String>,
    workflow: Option<PathBuf>,
    goal: Option<String>,
    done_when: Option<String>,
    max_goal_attempts: Option<usize>,
    token_budget: Option<u32>,
    cost_budget_usd: Option<f64>,
    max_duration_secs: Option<u64>,
    prompt: String,
    working_dir: PathBuf,
) -> anyhow::Result<()> {
    let working_dir = std::fs::canonicalize(&working_dir).unwrap_or(working_dir);
    let project_enabled =
        crate::workbench::project_skills::load_project_enabled_skills(&working_dir).await;
    let runtime = crate::bootstrap::initialize_runtime(
        &config,
        None,
        None,
        crate::bootstrap::MemoryAttachMode::Shared,
        project_enabled,
    )
    .await?;
    let disk = DiskTaskOutput::new_default()?;
    if let Some(workflow_path) = workflow {
        return run_workflow_path(&runtime, &disk, &working_dir, &workflow_path, Some(prompt))
            .await;
    }
    let resolved_mode = mode
        .as_deref()
        .and_then(RuntimeMode::parse)
        .unwrap_or(config.runtime.default_mode);
    let resolved_agent =
        agent_type.unwrap_or_else(|| resolved_mode.default_agent().as_str().to_string());
    let run_options = RunTaskOptions {
        budget: TaskBudget {
            token_budget_total: token_budget,
            cost_budget_usd,
            max_duration_secs,
            ..TaskBudget::default()
        },
        ..RunTaskOptions::default()
    };
    if let Some(goal) = goal {
        run_goal_task_with_tail(
            &runtime,
            &disk,
            resolved_agent,
            prompt,
            working_dir,
            goal,
            done_when,
            max_goal_attempts,
            run_options,
        )
        .await?;
        return Ok(());
    }
    let mut sink = ReplSink::Stdio;
    let _task_id = run_single_task_with_tail(
        &runtime,
        &disk,
        resolved_agent,
        prompt,
        working_dir,
        &mut sink,
        None,
        run_options,
        Some(&config),
    )
    .await?;
    Ok(())
}

/// Single task execution shared by `run` / `repl` (disk tail + result printing)。
/// `ReplSink::Stream` 写入内嵌 transcript；`Stdio` 走真实终端。
pub(crate) async fn run_single_task_with_tail(
    runtime: &AgentRuntime,
    disk: &DiskTaskOutput,
    agent_type: String,
    prompt: String,
    working_dir: PathBuf,
    sink: &mut ReplSink,
    capture_output: Option<&mut String>,
    options: RunTaskOptions,
    config: Option<&Config>,
) -> anyhow::Result<TaskId> {
    info!("Running task with agent: {}", agent_type);
    info!("Working directory: {:?}", working_dir);
    info!("Prompt: {}", prompt);

    let mut task = build_headless_task(agent_type, prompt, working_dir, &options, config);

    let output_path = disk.ensure_initialized(task.id)?;
    let mut po = FluentArgs::new();
    po.set("path", output_path.display().to_string());
    sink.eprint_line(tr_args("repl-task-out", &po));

    let dashboard_cancel = Arc::new(AtomicBool::new(false));
    let mut recorder = None;
    if let Some(parent) = options.dashboard_parent.clone() {
        recorder = Some(parent);
    } else if let Some(db) = DashboardRecorder::open().await {
        let kind = options.dashboard_kind.unwrap_or(RunSessionKind::Run);
        let title_hint = options.dashboard_title.as_deref().unwrap_or(&task.prompt);
        match DashboardRecorder::begin(db, kind, &task, title_hint).await {
            Ok(r) => {
                task.context.nested_cancel = Some(dashboard_cancel.clone());
                std::env::set_var(anycode_dashboard::approval_ipc::SESSION_ENV, r.session_id());
                recorder = Some(Arc::new(tokio::sync::Mutex::new(r)));
            }
            Err(e) => tracing::debug!(error = %e, "dashboard recorder begin skipped"),
        }
    }

    sink.eprint_line(tr("repl-task-run"));
    let exec = runtime.execute_task(task.clone());

    let mut offset: u64 = 0;
    let mut streamed_from_disk = String::new();
    tokio::pin!(exec);
    let exec_result = loop {
        tokio::select! {
            res = &mut exec => break res,
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(300)) => {
                let (delta, new_offset) = disk.read_delta(task.id, offset, 16 * 1024).unwrap_or_default();
                if !delta.is_empty() {
                    streamed_from_disk.push_str(&delta);
                    sink.push_stdout_str(&delta);
                    offset = new_offset;
                }
                if let Some(r) = recorder.as_ref() {
                    if let Ok(mut guard) = r.try_lock() {
                        crate::dashboard_record::poll_dashboard_cancel_ipc(
                            &guard,
                            &dashboard_cancel,
                        );
                        guard.ingest_delta(disk, task.id).await;
                    }
                }
            }
        }
    };
    if let Err(e) = exec_result {
        if options.dashboard_parent.is_none() {
            if let Some(r) = recorder.as_ref() {
                let guard = r.lock().await;
                guard.finish_run(disk, task.id, Some(&e.to_string())).await;
            }
            std::env::remove_var(anycode_dashboard::approval_ipc::SESSION_ENV);
        }
        return Err(e.into());
    }
    let result = exec_result?;

    // 最后一轮 `sleep` 与 `exec` 完成之间可能仍有 tail；补读避免漏段后误判「未流式输出」再整段打印一遍。
    loop {
        let (delta, new_offset) = disk
            .read_delta(task.id, offset, 512 * 1024)
            .unwrap_or_default();
        if delta.is_empty() {
            break;
        }
        streamed_from_disk.push_str(&delta);
        sink.push_stdout_str(&delta);
        offset = new_offset;
    }

    let summary_for_db = match &result {
        TaskResult::Success { output, .. } => Some(output.as_str()),
        TaskResult::Failure { error, .. } => Some(error.as_str()),
        TaskResult::Partial { success, .. } => Some(success.as_str()),
    };

    match &result {
        TaskResult::Success { output, artifacts } => {
            if let Some(cap) = capture_output {
                *cap = output.clone();
            }
            sink.eprint_line(tr("repl-task-ok"));
            let skip_duplicate_block =
                streamed_log_already_contains_output(&streamed_from_disk, output);
            if !skip_duplicate_block && !output.trim().is_empty() {
                sink.line("");
                sink.line(tr("repl-output-header"));
                sink.line(output);
            }
            let written = crate::artifact_summary::claude_turn_written_lines(artifacts);
            if !written.is_empty() {
                sink.line("");
                sink.eprint_line(tr("repl-written-header"));
                for line in written {
                    let mut wl = FluentArgs::new();
                    wl.set("line", line);
                    sink.eprint_line(tr_args("repl-written-line", &wl));
                }
            }
        }
        TaskResult::Failure { error, details } => {
            let mut fe = FluentArgs::new();
            fe.set("err", error.to_string());
            sink.eprint_line(tr_args("repl-task-fail", &fe));
            if let Some(details) = details {
                let mut fd = FluentArgs::new();
                fd.set("details", details.to_string());
                sink.eprint_line(tr_args("repl-task-details", &fd));
            }
        }
        TaskResult::Partial { success, remaining } => {
            if let Some(cap) = capture_output {
                *cap = format!("{success}\n{remaining}");
            }
            sink.eprint_line(tr("repl-task-partial"));
            let mut ps = FluentArgs::new();
            ps.set("done", success.to_string());
            sink.eprint_line(tr_args("repl-task-partial-done", &ps));
            let mut pr = FluentArgs::new();
            pr.set("rem", remaining.to_string());
            sink.eprint_line(tr_args("repl-task-partial-rem", &pr));
        }
    }

    if options.dashboard_parent.is_none() {
        if let Some(r) = recorder.as_ref() {
            let guard = r.lock().await;
            guard.finish_run(disk, task.id, summary_for_db).await;
        }
        std::env::remove_var(anycode_dashboard::approval_ipc::SESSION_ENV);
    }

    match result {
        TaskResult::Failure { error, details } => {
            if let Some(d) = details.filter(|s| !s.is_empty()) {
                anyhow::bail!("{error}: {d}");
            }
            anyhow::bail!("{error}");
        }
        _ => Ok(task.id),
    }
}

pub(crate) async fn run_goal_task_with_tail(
    runtime: &AgentRuntime,
    disk: &DiskTaskOutput,
    agent_type: String,
    prompt: String,
    working_dir: PathBuf,
    goal: String,
    done_when: Option<String>,
    max_goal_attempts: Option<usize>,
    options: RunTaskOptions,
) -> anyhow::Result<TaskId> {
    let working_dir = std::fs::canonicalize(&working_dir).unwrap_or(working_dir);
    let mut task = build_task(agent_type, prompt, working_dir.clone(), None);
    task.context.budget = options.budget;
    let output_path = disk.ensure_initialized(task.id)?;
    eprintln!("goal output log: {}", output_path.display());
    let done_when = done_when.filter(|s| !s.trim().is_empty());
    let max_cap = max_goal_attempts.map(|n| n.min(u32::MAX as usize) as u32);
    if let Some(ref marker) = done_when {
        match max_cap {
            Some(cap) => eprintln!("goal done_when: {marker:?} (max_attempts={cap})"),
            None => eprintln!("goal done_when: {marker:?} (max_attempts=unlimited)"),
        }
    } else if max_cap.is_some() {
        eprintln!("goal max_attempts={}", max_cap.unwrap());
    } else {
        eprintln!("goal retries: unlimited until objective is met");
    }
    let spec = GoalSpec {
        objective: goal.clone(),
        done_when: done_when.clone(),
        allow_infinite_retries: max_cap.is_none(),
        max_attempts_cap: max_cap,
    };

    let dashboard_cancel = Arc::new(AtomicBool::new(false));
    let mut recorder = None;
    if let Some(parent) = options.dashboard_parent.clone() {
        recorder = Some(parent);
    } else if let Some(db) = DashboardRecorder::open().await {
        let title = format!("Goal: {}", truncate_goal_title(&goal));
        match DashboardRecorder::begin(db, RunSessionKind::Goal, &task, &title).await {
            Ok(r) => {
                task.context.nested_cancel = Some(dashboard_cancel.clone());
                std::env::set_var(anycode_dashboard::approval_ipc::SESSION_ENV, r.session_id());
                recorder = Some(Arc::new(tokio::sync::Mutex::new(r)));
            }
            Err(e) => tracing::debug!(error = %e, "dashboard recorder begin skipped"),
        }
    }

    let exec = runtime.execute_goal_task(task.clone(), spec);
    tokio::pin!(exec);
    let goal_outcome = loop {
        tokio::select! {
            res = &mut exec => break res,
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(300)) => {
                if let Some(r) = recorder.as_ref() {
                    if let Ok(mut guard) = r.try_lock() {
                        crate::dashboard_record::poll_dashboard_cancel_ipc(
                            &guard,
                            &dashboard_cancel,
                        );
                        guard.ingest_delta(disk, task.id).await;
                    }
                }
            }
        }
    };
    let (result, progress) = match goal_outcome {
        Ok(v) => v,
        Err(e) => {
            if options.dashboard_parent.is_none() {
                if let Some(r) = recorder.as_ref() {
                    let guard = r.lock().await;
                    guard.finish_run(disk, task.id, Some(&e.to_string())).await;
                }
                std::env::remove_var(anycode_dashboard::approval_ipc::SESSION_ENV);
            }
            return Err(e.into());
        }
    };

    if options.dashboard_parent.is_none() {
        if let Some(r) = recorder.as_ref() {
            let mut guard = r.lock().await;
            guard
                .finish_goal(disk, task.id, &progress, done_when.as_deref(), &working_dir)
                .await;
        }
        std::env::remove_var(anycode_dashboard::approval_ipc::SESSION_ENV);
    }

    match result {
        TaskResult::Success { output, .. } => {
            println!("{}", output);
        }
        TaskResult::Failure { error, details } => {
            eprintln!("goal failed: {}", error);
            if let Some(details) = details {
                eprintln!("{}", details);
            }
        }
        TaskResult::Partial { success, remaining } => {
            println!("{}\n{}", success, remaining);
        }
    }
    eprintln!(
        "goal progress: attempts={} completed={} last_error={:?}",
        progress.attempts, progress.completed, progress.last_error
    );
    Ok(task.id)
}

fn truncate_goal_title(goal: &str) -> String {
    let one_line = goal.lines().next().unwrap_or(goal).trim();
    if one_line.chars().count() > 100 {
        format!("{}…", one_line.chars().take(100).collect::<String>())
    } else {
        one_line.to_string()
    }
}

fn build_task(
    agent_type: String,
    prompt: String,
    working_dir: PathBuf,
    system_prompt_append: Option<String>,
) -> Task {
    crate::task_builders::build_minimal_task(agent_type, prompt, working_dir, system_prompt_append)
}

#[cfg(test)]
mod stream_dedup_tests {
    use super::streamed_log_already_contains_output;

    #[test]
    fn detects_summary_already_in_streamed_log() {
        let log = "…\n== summary ==\nhello\n";
        assert!(streamed_log_already_contains_output(log, "hello"));
    }

    #[test]
    fn empty_output_never_counts_as_duplicate() {
        assert!(!streamed_log_already_contains_output("hello", ""));
    }
}
