//! Workflow execution helpers extracted from `tasks.rs`.

use super::tasks_run::{run_goal_task_with_tail, run_single_task_with_tail, RunTaskOptions};
use super::tasks_sink::ReplSink;
use crate::dashboard_record::DashboardRecorderHandle;
use crate::task_builders::build_headless_task;
use anycode_agent::AgentRuntime;
use anycode_core::prelude::*;
use anycode_core::{WorkflowDefinition, WorkflowStep};
use anycode_dashboard::{DashboardRecorder, RunSessionKind};
use anycode_tools::workflows;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

pub(super) async fn run_workflow_path(
    runtime: &AgentRuntime,
    disk: &DiskTaskOutput,
    working_dir: &Path,
    workflow_path: &Path,
    user_prompt: Option<String>,
) -> anyhow::Result<()> {
    let workflow = workflows::load_workflow_from_file(workflow_path)?;
    run_workflow_definition(
        runtime,
        disk,
        working_dir,
        &workflow,
        workflow_path,
        user_prompt,
    )
    .await
}

pub(super) async fn run_workflow_definition(
    runtime: &AgentRuntime,
    disk: &DiskTaskOutput,
    working_dir: &Path,
    workflow: &WorkflowDefinition,
    workflow_path: &Path,
    user_prompt: Option<String>,
) -> anyhow::Result<()> {
    let validation = crate::commands::workflow::validate_workflow_definition(workflow);
    if !validation.ok {
        let msg = validation
            .issues
            .iter()
            .map(|i| format!("{}: {}", i.severity, i.message))
            .collect::<Vec<_>>()
            .join("; ");
        anyhow::bail!("workflow validation failed: {msg}");
    }
    println!("workflow: {} ({})", workflow.name, workflow_path.display());
    let working_dir =
        std::fs::canonicalize(working_dir).unwrap_or_else(|_| working_dir.to_path_buf());
    let retry_max = workflow
        .retry
        .as_ref()
        .map(|r| r.max_attempts.max(1))
        .unwrap_or(1);
    let retry_backoff_ms = workflow.retry.as_ref().map(|r| r.backoff_ms).unwrap_or(0);
    let mut current_mode = workflow
        .mode
        .as_deref()
        .and_then(RuntimeMode::parse)
        .unwrap_or(RuntimeMode::Code);
    let mut context_text = user_prompt.clone().unwrap_or_default();
    let mut last_result = TaskResult::Failure {
        error: "workflow produced no steps".to_string(),
        details: None,
    };

    let wf_agent = current_mode.default_agent().as_str().to_string();
    let wf_prompt = format!(
        "workflow: {} — {}",
        workflow.name,
        user_prompt.as_deref().unwrap_or("(no user prompt)")
    );
    let wf_task = build_headless_task(
        wf_agent.clone(),
        wf_prompt,
        working_dir.clone(),
        &RunTaskOptions::default(),
        None,
    );
    let _ = disk.ensure_initialized(wf_task.id)?;

    let workflow_recorder: Option<DashboardRecorderHandle> =
        if let Some(db) = DashboardRecorder::open().await {
            DashboardRecorder::begin(db, RunSessionKind::Workflow, &wf_task, &workflow.name)
                .await
                .ok()
                .map(|r| {
                    std::env::set_var(anycode_dashboard::approval_ipc::SESSION_ENV, r.session_id());
                    Arc::new(tokio::sync::Mutex::new(r))
                })
        } else {
            None
        };

    let step_dashboard = |parent: &DashboardRecorderHandle| RunTaskOptions {
        dashboard_parent: Some(parent.clone()),
        ..RunTaskOptions::default()
    };

    let mut last_step_task_id = wf_task.id;

    for step in &workflow.steps {
        if !should_run_workflow_step(step, &context_text, &last_result) {
            println!("workflow step {} skipped by `when`", step.id);
            continue;
        }
        let mode = step
            .mode
            .as_deref()
            .and_then(RuntimeMode::parse)
            .unwrap_or(current_mode);
        let agent = step
            .agent
            .clone()
            .unwrap_or_else(|| mode.default_agent().as_str().to_string());
        let prompt = render_workflow_prompt(
            user_prompt.clone().unwrap_or_default(),
            workflow.name.as_str(),
            step,
            step.done_when.as_deref().or(workflow.done_when.as_deref()),
        );
        let mut dash_opts = workflow_recorder
            .as_ref()
            .map(step_dashboard)
            .unwrap_or_default();
        dash_opts.budget = step.budget;
        if !step.allowed_tools.is_empty() {
            dash_opts.tool_profile = Some("allowlist".into());
            dash_opts.tool_allowlist = Some(step.allowed_tools.clone());
        }
        if let Some(rec) = workflow_recorder.as_ref() {
            let g = rec.lock().await;
            g.log_workflow_step(&step.id, &format!("Step {} started", step.id), "running")
                .await;
        }
        let mut attempt = 0;
        loop {
            attempt += 1;
            let result = if mode == RuntimeMode::Goal {
                run_goal_task_with_tail(
                    runtime,
                    disk,
                    agent.clone(),
                    prompt.clone(),
                    working_dir.clone(),
                    step.done_when
                        .clone()
                        .or_else(|| workflow.done_when.clone())
                        .unwrap_or_else(|| step.prompt.clone()),
                    step.done_when
                        .clone()
                        .or_else(|| workflow.done_when.clone()),
                    None,
                    dash_opts.clone(),
                )
                .await
            } else {
                let mut sink = ReplSink::Stdio;
                run_single_task_with_tail(
                    runtime,
                    disk,
                    agent.clone(),
                    prompt.clone(),
                    working_dir.clone(),
                    &mut sink,
                    None,
                    dash_opts.clone(),
                    None,
                )
                .await
            };
            match result {
                Ok(tid) => {
                    if tid != Uuid::nil() {
                        last_step_task_id = tid;
                    }
                    last_result = TaskResult::Success {
                        output: format!("workflow step {} completed", step.id),
                        artifacts: vec![],
                    };
                    context_text = format!("step {} completed", step.id);
                    if let Some(rec) = workflow_recorder.as_ref() {
                        let g = rec.lock().await;
                        g.log_workflow_step(
                            &step.id,
                            &format!("Step {} completed", step.id),
                            "passed",
                        )
                        .await;
                    }
                    break;
                }
                Err(e) if attempt < retry_max => {
                    eprintln!(
                        "workflow step {} failed (attempt {}/{}): {}",
                        step.id, attempt, retry_max, e
                    );
                    if retry_backoff_ms > 0 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(retry_backoff_ms))
                            .await;
                    }
                }
                Err(e) => {
                    if let Some(rec) = workflow_recorder.as_ref() {
                        let g = rec.lock().await;
                        g.log_workflow_step(
                            &step.id,
                            &format!("Step {} failed: {e}", step.id),
                            "failed",
                        )
                        .await;
                        drop(g);
                        let mut g = rec.lock().await;
                        g.ingest_full_log(disk, last_step_task_id).await;
                        g.finish_with_status("failed", Some(&e.to_string())).await;
                    }
                    std::env::remove_var(anycode_dashboard::approval_ipc::SESSION_ENV);
                    return Err(e);
                }
            }
        }
    }
    if let Some(handoff) = &workflow.handoff {
        if let Some(next_mode) = handoff.next_mode.as_deref().and_then(RuntimeMode::parse) {
            current_mode = next_mode;
            println!("workflow handoff next_mode: {}", current_mode.as_str());
        }
        if let Some(message) = &handoff.message {
            println!("workflow handoff: {}", message);
            let agent = current_mode.default_agent().as_str().to_string();
            let prompt = format!(
                "{}\n\n## Workflow Handoff\nnext_mode={}\nmessage={}",
                user_prompt.clone().unwrap_or_default(),
                current_mode.as_str(),
                message
            );
            let dash_opts = workflow_recorder
                .as_ref()
                .map(step_dashboard)
                .unwrap_or_default();
            if current_mode == RuntimeMode::Goal {
                run_goal_task_with_tail(
                    runtime,
                    disk,
                    agent,
                    prompt,
                    working_dir.clone(),
                    workflow
                        .done_when
                        .clone()
                        .unwrap_or_else(|| message.clone()),
                    workflow.done_when.clone(),
                    None,
                    dash_opts,
                )
                .await?;
            } else {
                let mut sink = ReplSink::Stdio;
                last_step_task_id = run_single_task_with_tail(
                    runtime,
                    disk,
                    agent,
                    prompt,
                    working_dir.clone(),
                    &mut sink,
                    None,
                    dash_opts,
                    None,
                )
                .await?;
            }
        }
    }

    if let Some(rec) = workflow_recorder.as_ref() {
        let status = if matches!(last_result, TaskResult::Success { .. }) {
            "completed"
        } else {
            "failed"
        };
        let mut g = rec.lock().await;
        g.ingest_full_log(disk, last_step_task_id).await;
        g.finish_with_status(status, None).await;
    }
    std::env::remove_var(anycode_dashboard::approval_ipc::SESSION_ENV);

    Ok(())
}

pub(super) fn render_workflow_prompt(
    user_prompt: String,
    workflow_name: &str,
    step: &WorkflowStep,
    workflow_done_when: Option<&str>,
) -> String {
    let mut step_prompt = step.prompt.clone();
    for (key, value) in &step.vars {
        step_prompt = step_prompt.replace(&format!("{{{{{}}}}}", key), value);
    }
    let done_when = workflow_done_when.unwrap_or("step objective is complete");
    format!(
        "{}\n\n## Workflow\nname: {}\nstep_id: {}\ndone_when: {}\nstep_prompt: {}",
        user_prompt, workflow_name, step.id, done_when, step_prompt
    )
}

pub(super) fn should_run_workflow_step(
    step: &WorkflowStep,
    context_text: &str,
    last_result: &TaskResult,
) -> bool {
    let Some(raw_when) = step.when.as_deref() else {
        return true;
    };
    let cond = raw_when.trim();
    if cond.is_empty() || cond.eq_ignore_ascii_case("always") {
        return true;
    }
    if let Some(needle) = cond.strip_prefix("contains:") {
        return context_text.contains(needle.trim());
    }
    if let Some(needle) = cond.strip_prefix("not_contains:") {
        return !context_text.contains(needle.trim());
    }
    if cond.eq_ignore_ascii_case("result_success") {
        return matches!(last_result, TaskResult::Success { .. });
    }
    if cond.eq_ignore_ascii_case("result_failure") {
        return matches!(last_result, TaskResult::Failure { .. });
    }
    true
}
