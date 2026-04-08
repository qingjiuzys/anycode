//! Workflow execution helpers extracted from `tasks.rs`.

use super::tasks_run::{run_goal_task_with_tail, run_single_task_with_tail};
use super::tasks_sink::ReplSink;
use anycode_agent::AgentRuntime;
use anycode_core::prelude::*;
use anycode_core::{WorkflowDefinition, WorkflowStep};
use anycode_tools::workflows;
use std::path::Path;

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
    println!("workflow: {} ({})", workflow.name, workflow_path.display());
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
        let agent = mode.default_agent().as_str().to_string();
        let prompt = render_workflow_prompt(
            user_prompt.clone().unwrap_or_default(),
            workflow.name.as_str(),
            step,
            step.done_when.as_deref().or(workflow.done_when.as_deref()),
        );
        let mut attempt = 0;
        loop {
            attempt += 1;
            let result = if mode == RuntimeMode::Goal {
                run_goal_task_with_tail(
                    runtime,
                    disk,
                    agent.clone(),
                    prompt.clone(),
                    working_dir.to_path_buf(),
                    step.done_when
                        .clone()
                        .or_else(|| workflow.done_when.clone())
                        .unwrap_or_else(|| step.prompt.clone()),
                )
                .await
            } else {
                let mut sink = ReplSink::Stdio;
                run_single_task_with_tail(
                    runtime,
                    disk,
                    agent.clone(),
                    prompt.clone(),
                    working_dir.to_path_buf(),
                    &mut sink,
                )
                .await
            };
            match result {
                Ok(()) => {
                    last_result = TaskResult::Success {
                        output: format!("workflow step {} completed", step.id),
                        artifacts: vec![],
                    };
                    context_text = format!("step {} completed", step.id);
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
                Err(e) => return Err(e),
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
            if current_mode == RuntimeMode::Goal {
                run_goal_task_with_tail(
                    runtime,
                    disk,
                    agent,
                    prompt,
                    working_dir.to_path_buf(),
                    workflow
                        .done_when
                        .clone()
                        .unwrap_or_else(|| message.clone()),
                )
                .await?;
            } else {
                let mut sink = ReplSink::Stdio;
                run_single_task_with_tail(
                    runtime,
                    disk,
                    agent,
                    prompt,
                    working_dir.to_path_buf(),
                    &mut sink,
                )
                .await?;
            }
        }
    }
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
