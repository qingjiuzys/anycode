//! GraphEngine — multi-step workflow DAG coordination (Workbench + shared foundation).
//!
//! Owns topological layer scheduling, checkpoints, retries, and step handoff.
//! Each step still runs through [`crate::AgentRuntime::execute_task`] (single-step ReAct).
//! Cron/scheduler keeps its richer channel-bridge wrapper; Workbench calls this path directly.

use anycode_core::prelude::*;
use anycode_core::{
    plan_tree_is_empty, resolve_agent_loop_limits, workflow_ready_steps, workflow_topo_layers,
    PlanTree, WorkflowCheckpoint, WorkflowDefinition, WorkflowStep, WorkflowStepStatus,
};
use anycode_tools::workflows::{load_workflow_from_file, validate_workflow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use uuid::Uuid;

use crate::AgentRuntime;

/// Options for a Workbench / embedded graph run.
#[derive(Debug, Clone, Default)]
pub struct GraphRunOptions {
    pub working_directory: PathBuf,
    pub user_prompt: Option<String>,
    /// Dashboard session id for approval / chat-turn scoping.
    pub dashboard_session_id: Option<String>,
    pub nested_cancel: Option<Arc<AtomicBool>>,
    /// When set, resume / write checkpoint under this path (default: `.anycode/workflow-checkpoints/{name}.json`).
    pub checkpoint_path: Option<PathBuf>,
}

/// Per-step outcome from a graph run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStepResult {
    pub step_id: String,
    pub status: String,
    pub task_id: Option<String>,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Aggregate result of running a [`WorkflowDefinition`] via GraphEngine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphRunResult {
    pub run_id: String,
    pub workflow_name: String,
    pub status: String,
    pub layers: Vec<Vec<String>>,
    pub steps: Vec<GraphStepResult>,
    pub checkpoint: WorkflowCheckpoint,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Coordinates multi-step workflow DAGs; does **not** own the ReAct loop.
pub struct GraphEngine;

impl GraphEngine {
    /// Load a workflow from JSON or YAML text.
    pub fn parse_workflow_text(text: &str) -> Result<WorkflowDefinition, CoreError> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(CoreError::Other(anyhow::anyhow!("workflow text is empty")));
        }
        if trimmed.starts_with('{') {
            serde_json::from_str(trimmed)
                .map_err(|e| CoreError::Other(anyhow::anyhow!("workflow JSON parse error: {e}")))
        } else {
            serde_yaml::from_str(trimmed)
                .map_err(|e| CoreError::Other(anyhow::anyhow!("workflow YAML parse error: {e}")))
        }
    }

    /// Load from a filesystem path (YAML/JSON).
    pub fn load_workflow_path(path: &Path) -> Result<WorkflowDefinition, CoreError> {
        load_workflow_from_file(path)
            .map_err(|e| CoreError::Other(anyhow::anyhow!("load workflow {}: {e}", path.display())))
    }

    /// Sample 3-node linear graph for Workbench smoke / Settings demo.
    #[must_use]
    pub fn sample_three_node_workflow() -> WorkflowDefinition {
        WorkflowDefinition {
            name: "sample-three-node".into(),
            mode: Some("code".into()),
            steps: vec![
                WorkflowStep {
                    id: "scout".into(),
                    prompt: "Briefly inspect the project layout and note the top-level structure."
                        .into(),
                    agent: Some("explore".into()),
                    depends_on: vec![],
                    ..Default::default()
                },
                WorkflowStep {
                    id: "plan".into(),
                    prompt: "Propose a 3-bullet improvement plan based on the scout findings."
                        .into(),
                    agent: Some("plan".into()),
                    depends_on: vec!["scout".into()],
                    ..Default::default()
                },
                WorkflowStep {
                    id: "summarize".into(),
                    prompt: "Write a short summary of scout + plan for the user (no file writes)."
                        .into(),
                    agent: Some("general-purpose".into()),
                    depends_on: vec!["plan".into()],
                    ..Default::default()
                },
            ],
            ..Default::default()
        }
    }

    /// Convert a session plan tree into a sequential workflow (DFS order).
    #[must_use]
    pub fn plan_tree_to_workflow(tree: &PlanTree, name: impl Into<String>) -> WorkflowDefinition {
        let mut ordered: Vec<&anycode_core::PlanNode> = Vec::new();
        fn collect<'a>(
            nodes: &'a [anycode_core::PlanNode],
            out: &mut Vec<&'a anycode_core::PlanNode>,
        ) {
            for n in nodes {
                out.push(n);
                collect(&n.children, out);
            }
        }
        collect(&tree.roots, &mut ordered);

        let steps: Vec<WorkflowStep> = ordered
            .iter()
            .enumerate()
            .map(|(i, node)| {
                let prompt = match node
                    .detail
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    Some(detail) => format!("{}\n\n{}", node.title, detail),
                    None => node.title.clone(),
                };
                WorkflowStep {
                    id: node.id.clone(),
                    prompt,
                    agent: Some("general-purpose".into()),
                    depends_on: if i == 0 {
                        vec![]
                    } else {
                        vec![ordered[i - 1].id.clone()]
                    },
                    ..Default::default()
                }
            })
            .collect();

        let prose = tree.prose.trim();
        WorkflowDefinition {
            name: name.into(),
            mode: Some("code".into()),
            done_when: (!prose.is_empty()).then(|| prose.to_string()),
            steps,
            ..Default::default()
        }
    }

    /// Run a workflow: validate → topo layers → `execute_task` per ready step.
    pub async fn run(
        runtime: &AgentRuntime,
        workflow: &WorkflowDefinition,
        opts: GraphRunOptions,
    ) -> Result<GraphRunResult, CoreError> {
        let validation = validate_workflow(workflow);
        if !validation.ok {
            let msg = validation
                .issues
                .iter()
                .map(|i| format!("{}: {}", i.severity, i.message))
                .collect::<Vec<_>>()
                .join("; ");
            return Err(CoreError::Other(anyhow::anyhow!(
                "workflow validation failed: {msg}"
            )));
        }

        // Legacy required_gates were previously marked true without verification.
        // Refuse before side effects; migrate gated workflows to Harness GraphRunner.
        if workflow
            .steps
            .iter()
            .any(|step| !step.required_gates.is_empty())
        {
            return Err(CoreError::Other(anyhow::anyhow!(
                "legacy graph gates have no trusted verifier; use harness GraphRunner"
            )));
        }
        let layers = workflow_topo_layers(workflow).map_err(|v| {
            let msg = v
                .issues
                .iter()
                .map(|i| format!("{}: {}", i.severity, i.message))
                .collect::<Vec<_>>()
                .join("; ");
            CoreError::Other(anyhow::anyhow!("workflow DAG invalid: {msg}"))
        })?;

        let working_dir = std::fs::canonicalize(&opts.working_directory)
            .unwrap_or_else(|_| opts.working_directory.clone());
        // No implicit retry of arbitrary side-effecting legacy steps.
        // Harness retries require a trusted executor.safe_to_retry decision.
        let retry_max = 1u32;
        let retry_backoff_ms = workflow.retry.as_ref().map(|r| r.backoff_ms).unwrap_or(0);
        let default_mode = workflow
            .mode
            .as_deref()
            .and_then(RuntimeMode::parse)
            .unwrap_or(RuntimeMode::Code);

        let checkpoint_path = opts.checkpoint_path.clone().unwrap_or_else(|| {
            working_dir
                .join(".anycode")
                .join("workflow-checkpoints")
                .join(format!(
                    "{}-{}.json",
                    workflow.name.replace('/', "_"),
                    Uuid::new_v4()
                ))
        });
        let run_id = Uuid::new_v4().to_string();
        if checkpoint_path.exists() {
            return Err(CoreError::Other(anyhow::anyhow!(
                "legacy checkpoint resume is disabled: use explicit scope-bound harness resume"
            )));
        }
        let mut checkpoint = WorkflowCheckpoint::new(workflow, run_id.clone());
        let run_id = checkpoint.run_id.clone();
        let _ = std::fs::create_dir_all(
            checkpoint_path
                .parent()
                .unwrap_or_else(|| working_dir.as_path()),
        );

        let step_by_id: HashMap<&str, &WorkflowStep> =
            workflow.steps.iter().map(|s| (s.id.as_str(), s)).collect();
        let mut context_text = opts.user_prompt.clone().unwrap_or_default();
        let mut last_result = TaskResult::Failure {
            error: "workflow produced no steps".into(),
            details: None,
        };
        let mut step_results: Vec<GraphStepResult> = Vec::new();

        for layer in &layers {
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
                    step_results.push(GraphStepResult {
                        step_id: step.id.clone(),
                        status: "skipped_checkpoint".into(),
                        task_id: None,
                        summary: format!("step {} already completed", step.id),
                        error: None,
                    });
                    continue;
                }
                let ready_ids: std::collections::HashSet<_> =
                    workflow_ready_steps(workflow, &checkpoint)
                        .into_iter()
                        .map(|s| s.id.as_str())
                        .collect();
                if !ready_ids.contains(step.id.as_str()) {
                    continue;
                }
                if !should_run_workflow_step(step, &context_text, &last_result) {
                    checkpoint.mark_skipped(&step.id);
                    step_results.push(GraphStepResult {
                        step_id: step.id.clone(),
                        status: "skipped".into(),
                        task_id: None,
                        summary: format!("step {} skipped by `when`", step.id),
                        error: None,
                    });
                    continue;
                }
                if let Some(st) = checkpoint.steps.get_mut(&step.id) {
                    st.status = WorkflowStepStatus::Running;
                }

                let mode = step
                    .mode
                    .as_deref()
                    .and_then(RuntimeMode::parse)
                    .unwrap_or(default_mode);
                let agent = step
                    .agent
                    .clone()
                    .unwrap_or_else(|| mode.default_agent().as_str().to_string());
                let mut prompt = render_workflow_prompt(
                    opts.user_prompt.clone().unwrap_or_default(),
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

                let mut attempt = 0u32;
                let step_outcome = loop {
                    attempt += 1;
                    let task_id = Uuid::new_v4();
                    let task = Task {
                        id: task_id,
                        agent_type: AgentType::new(&agent),
                        prompt: prompt.clone(),
                        context: TaskContext {
                            session_id: Uuid::new_v4(),
                            working_directory: working_dir.to_string_lossy().to_string(),
                            environment: HashMap::new(),
                            user_id: None,
                            system_prompt_append: None,
                            context_injections: vec![],
                            nested_model_override: step.model.clone(),
                            nested_worktree_path: None,
                            nested_worktree_repo_root: None,
                            nested_cancel: opts.nested_cancel.clone(),
                            channel_progress_tx: None,
                            live_trace_tx: None,
                            tool_deny_names: vec![],
                            tool_deny_prefixes: vec![],
                            user_vision_images: vec![],
                            budget: step.budget,
                            loop_limits: resolve_agent_loop_limits(None, None),
                            chat_turn: opts.dashboard_session_id.as_ref().map(|sid| {
                                ChatTurnContext {
                                    dashboard_session_id: Some(sid.clone()),
                                    user_turn_id: None,
                                    reply_language: None,
                                    host_intent_hint: Some("graph_engine".into()),
                                }
                            }),
                        },
                        created_at: chrono::Utc::now(),
                    };

                    let exec = if mode == RuntimeMode::Goal {
                        let done = step
                            .done_when
                            .clone()
                            .or_else(|| workflow.done_when.clone())
                            .unwrap_or_else(|| step.prompt.clone());
                        let spec = GoalSpec {
                            objective: done.clone(),
                            done_when: Some(done),
                            max_attempts_cap: Some(retry_max),
                            ..GoalSpec::default()
                        };
                        runtime.execute_goal_task(task, spec).await.map(|(r, _)| r)
                    } else {
                        runtime.execute_task(task).await
                    };

                    match exec {
                        Ok(result) => {
                            let summary = match &result {
                                TaskResult::Success { output, .. } => output.clone(),
                                TaskResult::Partial { success, .. } => success.clone(),
                                TaskResult::Failure { error, details } => {
                                    details.clone().unwrap_or_else(|| error.clone())
                                }
                            };
                            let ok = matches!(result, TaskResult::Success { .. });
                            if ok {
                                last_result = result;
                                context_text = format!("step {} completed", step.id);
                                checkpoint.context_text = context_text.clone();
                                let artifact = truncate_summary(&summary, 2_000);
                                checkpoint.mark_passed(&step.id, artifact.clone());
                                persist_checkpoint(&checkpoint_path, &checkpoint);
                                break Ok(GraphStepResult {
                                    step_id: step.id.clone(),
                                    status: "passed".into(),
                                    task_id: Some(task_id.to_string()),
                                    summary: artifact,
                                    error: None,
                                });
                            }
                            if attempt < retry_max {
                                if retry_backoff_ms > 0 {
                                    tokio::time::sleep(std::time::Duration::from_millis(
                                        retry_backoff_ms,
                                    ))
                                    .await;
                                }
                                continue;
                            }
                            checkpoint.mark_failed(&step.id, summary.clone());
                            persist_checkpoint(&checkpoint_path, &checkpoint);
                            break Err((task_id, summary));
                        }
                        Err(e) if e.is_cooperative_cancel() => {
                            checkpoint.mark_failed(&step.id, e.to_string());
                            persist_checkpoint(&checkpoint_path, &checkpoint);
                            break Err((task_id, e.to_string()));
                        }
                        Err(e) if attempt < retry_max => {
                            tracing::warn!(
                                step = %step.id,
                                attempt,
                                retry_max,
                                error = %e,
                                "graph step failed; retrying"
                            );
                            if retry_backoff_ms > 0 {
                                tokio::time::sleep(std::time::Duration::from_millis(
                                    retry_backoff_ms,
                                ))
                                .await;
                            }
                        }
                        Err(e) => {
                            checkpoint.mark_failed(&step.id, e.to_string());
                            persist_checkpoint(&checkpoint_path, &checkpoint);
                            break Err((task_id, e.to_string()));
                        }
                    }
                };

                match step_outcome {
                    Ok(sr) => step_results.push(sr),
                    Err((task_id, err)) => {
                        step_results.push(GraphStepResult {
                            step_id: step.id.clone(),
                            status: "failed".into(),
                            task_id: Some(task_id.to_string()),
                            summary: format!("step {} failed", step.id),
                            error: Some(err.clone()),
                        });
                        return Ok(GraphRunResult {
                            run_id,
                            workflow_name: workflow.name.clone(),
                            status: "failed".into(),
                            layers,
                            steps: step_results,
                            checkpoint,
                            error: Some(err),
                        });
                    }
                }
            }
        }

        let status = if matches!(last_result, TaskResult::Success { .. })
            || step_results.iter().any(|s| s.status == "passed")
        {
            "completed"
        } else if workflow.steps.is_empty() {
            "failed"
        } else {
            "completed"
        };

        Ok(GraphRunResult {
            run_id,
            workflow_name: workflow.name.clone(),
            status: status.into(),
            layers,
            steps: step_results,
            checkpoint,
            error: if status == "failed" {
                Some("workflow produced no successful steps".into())
            } else {
                None
            },
        })
    }

    /// True when a plan tree can be converted into a non-empty workflow.
    #[must_use]
    pub fn plan_tree_runnable(tree: &PlanTree) -> bool {
        !plan_tree_is_empty(tree)
    }
}

fn persist_checkpoint(path: &Path, checkpoint: &WorkflowCheckpoint) {
    if let Ok(raw) = serde_json::to_string_pretty(checkpoint) {
        let _ = std::fs::write(path, raw);
    }
}

fn truncate_summary(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    s.chars().take(max).collect::<String>() + "…"
}

fn render_workflow_prompt(
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

fn should_run_workflow_step(
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

#[cfg(test)]
mod tests {
    use super::*;
    use anycode_core::{PlanNode, PlanStatus, PlanTree};

    #[test]
    fn sample_three_node_validates_and_layers() {
        let wf = GraphEngine::sample_three_node_workflow();
        let v = validate_workflow(&wf);
        assert!(v.ok, "{:?}", v.issues);
        let layers = workflow_topo_layers(&wf).unwrap();
        assert_eq!(layers.len(), 3);
        assert_eq!(layers[0], vec!["scout".to_string()]);
        assert_eq!(layers[1], vec!["plan".to_string()]);
        assert_eq!(layers[2], vec!["summarize".to_string()]);
    }

    #[test]
    fn plan_tree_to_workflow_sequential() {
        let tree = PlanTree {
            prose: "Ship the feature".into(),
            roots: vec![PlanNode {
                id: "a".into(),
                title: "Scout".into(),
                status: PlanStatus::Pending,
                children: vec![PlanNode {
                    id: "b".into(),
                    title: "Implement".into(),
                    status: PlanStatus::Pending,
                    children: vec![],
                    detail: Some("touch main.rs".into()),
                    kind: None,
                }],
                detail: None,
                kind: None,
            }],
        };
        let wf = GraphEngine::plan_tree_to_workflow(&tree, "from-plan");
        assert_eq!(wf.steps.len(), 2);
        assert!(wf.steps[0].depends_on.is_empty());
        assert_eq!(wf.steps[1].depends_on, vec!["a".to_string()]);
        assert!(wf.steps[1].prompt.contains("touch main.rs"));
        assert!(GraphEngine::plan_tree_runnable(&tree));
    }

    #[test]
    fn parse_workflow_json() {
        let text = r#"{
            "name": "t",
            "steps": [{"id": "x", "prompt": "do x"}]
        }"#;
        let wf = GraphEngine::parse_workflow_text(text).unwrap();
        assert_eq!(wf.name, "t");
        assert_eq!(wf.steps[0].id, "x");
    }

    #[test]
    fn when_clause_matches_context_and_last_result() {
        let step = WorkflowStep {
            id: "x".into(),
            prompt: "p".into(),
            when: Some("contains:scout".into()),
            ..Default::default()
        };
        let ok = TaskResult::Success {
            output: "done".into(),
            artifacts: vec![],
        };
        let fail = TaskResult::Failure {
            error: "nope".into(),
            details: None,
        };
        assert!(should_run_workflow_step(&step, "step scout completed", &ok));
        assert!(!should_run_workflow_step(&step, "step plan completed", &ok));

        let always = WorkflowStep {
            when: Some("always".into()),
            ..step.clone()
        };
        assert!(should_run_workflow_step(&always, "", &fail));

        let on_ok = WorkflowStep {
            when: Some("result_success".into()),
            ..step.clone()
        };
        assert!(should_run_workflow_step(&on_ok, "", &ok));
        assert!(!should_run_workflow_step(&on_ok, "", &fail));
    }
}
