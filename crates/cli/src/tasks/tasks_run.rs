//! `anycode run` / 单次任务执行与 goal 循环（与 REPL 解耦）。

use crate::i18n::{tr, tr_args};
use crate::workspace;
use anycode_agent::AgentRuntime;
use anycode_core::prelude::*;
use fluent_bundle::FluentArgs;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::info;
use uuid::Uuid;

use super::tasks_sink::ReplSink;
use super::workflow_exec::run_workflow_path;

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
    mut config: crate::app_config::Config,
    agent_type: Option<String>,
    mode: Option<String>,
    workflow: Option<PathBuf>,
    goal: Option<String>,
    prompt: String,
    working_dir: PathBuf,
) -> anyhow::Result<()> {
    let working_dir = std::fs::canonicalize(&working_dir).unwrap_or(working_dir);
    workspace::apply_project_overlays(&mut config, &working_dir);
    let runtime = crate::bootstrap::initialize_runtime(&config, None, None).await?;
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
    if let Some(goal) = goal {
        return run_goal_task_with_tail(&runtime, &disk, resolved_agent, prompt, working_dir, goal)
            .await;
    }
    let mut sink = ReplSink::Stdio;
    run_single_task_with_tail(
        &runtime,
        &disk,
        resolved_agent,
        prompt,
        working_dir,
        &mut sink,
    )
    .await
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
) -> anyhow::Result<()> {
    info!("Running task with agent: {}", agent_type);
    info!("Working directory: {:?}", working_dir);
    info!("Prompt: {}", prompt);

    let working_dir = std::fs::canonicalize(&working_dir).unwrap_or(working_dir);

    let task = Task {
        id: Uuid::new_v4(),
        agent_type: AgentType::new(agent_type),
        prompt,
        context: TaskContext {
            session_id: Uuid::new_v4(),
            working_directory: working_dir.to_string_lossy().to_string(),
            environment: HashMap::new(),
            user_id: None,
            system_prompt_append: None,
            context_injections: vec![],
            nested_model_override: None,
            nested_worktree_path: None,
            nested_worktree_repo_root: None,
            nested_cancel: None,
            channel_progress_tx: None,
        },
        created_at: chrono::Utc::now(),
    };

    let output_path = disk.ensure_initialized(task.id)?;
    let mut po = FluentArgs::new();
    po.set("path", output_path.display().to_string());
    sink.eprint_line(tr_args("repl-task-out", &po));

    sink.eprint_line(tr("repl-task-run"));
    let exec = runtime.execute_task(task.clone());

    let mut offset: u64 = 0;
    let mut streamed_from_disk = String::new();
    tokio::pin!(exec);
    let result = loop {
        tokio::select! {
            res = &mut exec => break res?,
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(300)) => {
                let (delta, new_offset) = disk.read_delta(task.id, offset, 16 * 1024).unwrap_or_default();
                if !delta.is_empty() {
                    streamed_from_disk.push_str(&delta);
                    sink.push_stdout_str(&delta);
                    offset = new_offset;
                }
            }
        }
    };

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

    match result {
        TaskResult::Success { output, artifacts } => {
            sink.eprint_line(tr("repl-task-ok"));
            let skip_duplicate_block =
                streamed_log_already_contains_output(&streamed_from_disk, &output);
            if !skip_duplicate_block && !output.trim().is_empty() {
                sink.line("");
                sink.line(tr("repl-output-header"));
                sink.line(output);
            }
            let written = crate::artifact_summary::claude_turn_written_lines(&artifacts);
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
            sink.eprint_line(tr("repl-task-partial"));
            let mut ps = FluentArgs::new();
            ps.set("done", success.to_string());
            sink.eprint_line(tr_args("repl-task-partial-done", &ps));
            let mut pr = FluentArgs::new();
            pr.set("rem", remaining.to_string());
            sink.eprint_line(tr_args("repl-task-partial-rem", &pr));
        }
    }

    Ok(())
}

pub(crate) async fn run_goal_task_with_tail(
    runtime: &AgentRuntime,
    disk: &DiskTaskOutput,
    agent_type: String,
    prompt: String,
    working_dir: PathBuf,
    goal: String,
) -> anyhow::Result<()> {
    let working_dir = std::fs::canonicalize(&working_dir).unwrap_or(working_dir);
    let task = build_task(agent_type, prompt, working_dir, None);
    let output_path = disk.ensure_initialized(task.id)?;
    eprintln!("goal output log: {}", output_path.display());
    let (result, progress) = runtime
        .execute_goal_task(
            task,
            GoalSpec {
                objective: goal,
                done_when: None,
                allow_infinite_retries: true,
                max_attempts_cap: None,
            },
        )
        .await?;
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
    Ok(())
}

fn build_task(
    agent_type: String,
    prompt: String,
    working_dir: PathBuf,
    system_prompt_append: Option<String>,
) -> Task {
    Task {
        id: Uuid::new_v4(),
        agent_type: AgentType::new(agent_type),
        prompt,
        context: TaskContext {
            session_id: Uuid::new_v4(),
            working_directory: working_dir.to_string_lossy().to_string(),
            environment: HashMap::new(),
            user_id: None,
            system_prompt_append,
            context_injections: vec![],
            nested_model_override: None,
            nested_worktree_path: None,
            nested_worktree_repo_root: None,
            nested_cancel: None,
            channel_progress_tx: None,
        },
        created_at: chrono::Utc::now(),
    }
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
