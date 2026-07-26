//! Workflow execution helpers extracted from `tasks.rs`.
//!
//! Live entry: cron jobs with a `workflow` file run this DAG executor from
//! `crate::scheduler` (ADR 014 §6).

use super::tasks_run::{run_goal_task_with_tail, run_single_task_with_tail, RunTaskOptions};
use super::tasks_sink::ReplSink;
use crate::task_builders::build_headless_task;
use crate::workbench::dashboard_record::DashboardRecorderHandle;
use anycode_agent::AgentRuntime;
use anycode_core::prelude::*;
use anycode_core::{
    workflow_ready_steps, workflow_topo_layers, WorkflowCheckpoint, WorkflowDefinition,
    WorkflowStep, WorkflowStepStatus,
};
use anycode_dashboard::{DashboardRecorder, RunSessionKind};
use anycode_tools::workflows;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

pub(crate) async fn run_workflow_path(
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
    let validation = crate::workflow_validate::validate_workflow_definition(workflow);
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
    let layers = workflow_topo_layers(workflow).map_err(|v| {
        let msg = v
            .issues
            .iter()
            .map(|i| format!("{}: {}", i.severity, i.message))
            .collect::<Vec<_>>()
            .join("; ");
        anyhow::anyhow!("workflow DAG invalid: {msg}")
    })?;
    let checkpoint_path = working_dir
        .join(".anycode")
        .join("workflow-checkpoints")
        .join(format!("{}.json", workflow.name.replace('/', "_")));
    let mut checkpoint = if checkpoint_path.exists() {
        serde_json::from_str::<WorkflowCheckpoint>(&std::fs::read_to_string(&checkpoint_path)?)
            .unwrap_or_else(|_| WorkflowCheckpoint::new(workflow, Uuid::new_v4().to_string()))
    } else {
        WorkflowCheckpoint::new(workflow, Uuid::new_v4().to_string())
    };
    let _ = std::fs::create_dir_all(checkpoint_path.parent().unwrap_or(working_dir.as_path()));

    let step_by_id: std::collections::HashMap<&str, &WorkflowStep> =
        workflow.steps.iter().map(|s| (s.id.as_str(), s)).collect();

    for layer in &layers {
        // Restricted parallelism: run ready steps in layer sequentially for safety,
        // but honor depends_on so unrelated branches unlock together across layers.
        for step_id in layer {
            let Some(step) = step_by_id.get(step_id.as_str()).copied() else {
                continue;
            };
            let status = checkpoint
                .steps
                .get(&step.id)
                .map(|s| s.status)
                .unwrap_or(WorkflowStepStatus::Pending);
            if matches!(
                status,
                WorkflowStepStatus::Passed | WorkflowStepStatus::Skipped
            ) {
                continue;
            }
            // Ensure deps satisfied (resume safety).
            let ready_ids: std::collections::HashSet<_> =
                workflow_ready_steps(workflow, &checkpoint)
                    .into_iter()
                    .map(|s| s.id.as_str())
                    .collect();
            if !ready_ids.contains(step.id.as_str()) {
                continue;
            }
            if !should_run_workflow_step(step, &context_text, &last_result) {
                println!("workflow step {} skipped by `when`", step.id);
                checkpoint.mark_skipped(&step.id);
                continue;
            }
            if let Some(st) = checkpoint.steps.get_mut(&step.id) {
                st.status = WorkflowStepStatus::Running;
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
            let mut prompt = render_workflow_prompt(
                user_prompt.clone().unwrap_or_default(),
                workflow.name.as_str(),
                step,
                step.done_when.as_deref().or(workflow.done_when.as_deref()),
            );
            if !step.required_gates.is_empty() {
                prompt.push_str(&format!(
                    "\nrequired_gates: {}",
                    step.required_gates.join(", ")
                ));
            }
            // Artifact handoff from completed deps.
            let mut handoff = String::new();
            for dep in &step.depends_on {
                if let Some(dep_st) = checkpoint.steps.get(dep) {
                    if !dep_st.artifact_summary.is_empty() {
                        handoff.push_str(&format!(
                            "\n## Artifact from {dep}\n{}\n",
                            dep_st.artifact_summary
                        ));
                    }
                }
            }
            if !handoff.is_empty() {
                prompt.push_str(&handoff);
            }
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
                        checkpoint.context_text = context_text.clone();
                        for gate in &step.required_gates {
                            if let Some(st) = checkpoint.steps.get_mut(&step.id) {
                                st.gate_results.insert(gate.clone(), true);
                            }
                        }
                        checkpoint.mark_passed(
                            &step.id,
                            format!(
                                "step {} completed; gates={:?}",
                                step.id, step.required_gates
                            ),
                        );
                        let _ = std::fs::write(
                            &checkpoint_path,
                            serde_json::to_string_pretty(&checkpoint).unwrap_or_default(),
                        );
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
                            tokio::time::sleep(tokio::time::Duration::from_millis(
                                retry_backoff_ms,
                            ))
                            .await;
                        }
                    }
                    Err(e) => {
                        checkpoint.mark_failed(&step.id, e.to_string());
                        let _ = std::fs::write(
                            &checkpoint_path,
                            serde_json::to_string_pretty(&checkpoint).unwrap_or_default(),
                        );
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
