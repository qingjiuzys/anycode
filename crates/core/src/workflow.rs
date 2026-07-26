//! Minimal YAML workflow schema + DAG scheduling / checkpoint helpers.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

use crate::task::TaskBudget;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowDefinition {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default)]
    pub steps: Vec<WorkflowStep>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry: Option<WorkflowRetry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub done_when: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handoff: Option<WorkflowHandoff>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowStep {
    pub id: String,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parallel_group: Option<String>,
    #[serde(default, skip_serializing_if = "TaskBudget::is_empty")]
    pub budget: TaskBudget,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_gates: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub done_when: Option<String>,
    #[serde(default)]
    pub vars: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlanValidationIssue {
    pub severity: String,
    pub step_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlanValidationResult {
    pub ok: bool,
    #[serde(default)]
    pub issues: Vec<PlanValidationIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowRetry {
    #[serde(default = "default_attempts")]
    pub max_attempts: u32,
    #[serde(default)]
    pub backoff_ms: u64,
}

fn default_attempts() -> u32 {
    3
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowHandoff {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Per-step runtime status for recoverable DAG execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStepStatus {
    #[default]
    Pending,
    Ready,
    Running,
    Passed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowStepState {
    pub step_id: String,
    pub status: WorkflowStepStatus,
    #[serde(default)]
    pub attempts: u32,
    #[serde(default)]
    pub artifact_summary: String,
    #[serde(default)]
    pub gate_results: HashMap<String, bool>,
    #[serde(default)]
    pub last_error: String,
}

/// Durable workflow checkpoint (Desktop/daemon restart resume).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowCheckpoint {
    pub workflow_name: String,
    pub run_id: String,
    #[serde(default)]
    pub context_text: String,
    #[serde(default)]
    pub steps: HashMap<String, WorkflowStepState>,
    #[serde(default)]
    pub completed_order: Vec<String>,
    #[serde(default)]
    pub version: u32,
}

impl WorkflowCheckpoint {
    pub fn new(workflow: &WorkflowDefinition, run_id: impl Into<String>) -> Self {
        let mut steps = HashMap::new();
        for step in &workflow.steps {
            steps.insert(
                step.id.clone(),
                WorkflowStepState {
                    step_id: step.id.clone(),
                    status: WorkflowStepStatus::Pending,
                    ..Default::default()
                },
            );
        }
        Self {
            workflow_name: workflow.name.clone(),
            run_id: run_id.into(),
            context_text: String::new(),
            steps,
            completed_order: Vec::new(),
            version: 2,
        }
    }

    pub fn mark_passed(&mut self, step_id: &str, artifact_summary: impl Into<String>) {
        if let Some(st) = self.steps.get_mut(step_id) {
            st.status = WorkflowStepStatus::Passed;
            st.artifact_summary = artifact_summary.into();
            st.last_error.clear();
        }
        if !self.completed_order.iter().any(|s| s == step_id) {
            self.completed_order.push(step_id.to_string());
        }
    }

    pub fn mark_failed(&mut self, step_id: &str, error: impl Into<String>) {
        if let Some(st) = self.steps.get_mut(step_id) {
            st.status = WorkflowStepStatus::Failed;
            st.last_error = error.into();
            st.attempts = st.attempts.saturating_add(1);
        }
    }

    pub fn mark_skipped(&mut self, step_id: &str) {
        if let Some(st) = self.steps.get_mut(step_id) {
            st.status = WorkflowStepStatus::Skipped;
        }
    }

    pub fn is_complete(&self, workflow: &WorkflowDefinition) -> bool {
        workflow.steps.iter().all(|s| {
            matches!(
                self.steps
                    .get(&s.id)
                    .map(|st| st.status)
                    .unwrap_or(WorkflowStepStatus::Pending),
                WorkflowStepStatus::Passed | WorkflowStepStatus::Skipped
            )
        })
    }
}

/// Validate dependency graph and return topological layers (each layer may run in parallel).
pub fn workflow_topo_layers(
    workflow: &WorkflowDefinition,
) -> Result<Vec<Vec<String>>, PlanValidationResult> {
    let mut issues = Vec::new();
    let ids: HashSet<&str> = workflow.steps.iter().map(|s| s.id.as_str()).collect();
    let mut indegree: HashMap<&str, usize> = HashMap::new();
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for step in &workflow.steps {
        indegree.entry(step.id.as_str()).or_insert(0);
        for dep in &step.depends_on {
            if !ids.contains(dep.as_str()) {
                issues.push(PlanValidationIssue {
                    severity: "error".into(),
                    step_id: Some(step.id.clone()),
                    message: format!("depends_on unknown step {dep}"),
                });
                continue;
            }
            adj.entry(dep.as_str()).or_default().push(step.id.as_str());
            *indegree.entry(step.id.as_str()).or_insert(0) += 1;
        }
    }
    if !issues.is_empty() {
        return Err(PlanValidationResult { ok: false, issues });
    }

    let mut queue: VecDeque<&str> = indegree
        .iter()
        .filter(|(_, d)| **d == 0)
        .map(|(id, _)| *id)
        .collect();
    // Stable order by declaration for determinism.
    let order_index: HashMap<&str, usize> = workflow
        .steps
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id.as_str(), i))
        .collect();
    let mut layers = Vec::new();
    let mut seen = 0usize;
    while !queue.is_empty() {
        let mut layer: Vec<&str> = queue.drain(..).collect();
        layer.sort_by_key(|id| order_index.get(id).copied().unwrap_or(usize::MAX));
        let mut next = Vec::new();
        for id in &layer {
            seen += 1;
            if let Some(children) = adj.get(id) {
                for child in children {
                    if let Some(d) = indegree.get_mut(child) {
                        *d = d.saturating_sub(1);
                        if *d == 0 {
                            next.push(*child);
                        }
                    }
                }
            }
        }
        layers.push(layer.into_iter().map(str::to_string).collect());
        queue.extend(next);
    }
    if seen != workflow.steps.len() {
        issues.push(PlanValidationIssue {
            severity: "error".into(),
            step_id: None,
            message: "workflow depends_on graph has a cycle".into(),
        });
        return Err(PlanValidationResult { ok: false, issues });
    }
    Ok(layers)
}

/// Steps whose dependencies are all passed/skipped and themselves still pending.
pub fn workflow_ready_steps<'a>(
    workflow: &'a WorkflowDefinition,
    checkpoint: &WorkflowCheckpoint,
) -> Vec<&'a WorkflowStep> {
    workflow
        .steps
        .iter()
        .filter(|step| {
            let status = checkpoint
                .steps
                .get(&step.id)
                .map(|s| s.status)
                .unwrap_or(WorkflowStepStatus::Pending);
            if !matches!(
                status,
                WorkflowStepStatus::Pending
                    | WorkflowStepStatus::Ready
                    | WorkflowStepStatus::Failed
            ) {
                return false;
            }
            step.depends_on.iter().all(|dep| {
                matches!(
                    checkpoint
                        .steps
                        .get(dep)
                        .map(|s| s.status)
                        .unwrap_or(WorkflowStepStatus::Pending),
                    WorkflowStepStatus::Passed | WorkflowStepStatus::Skipped
                )
            })
        })
        .collect()
}

/// Default webpage recipe: prefs → design → implement → visual verify → test → accept.
pub fn webpage_default_workflow() -> WorkflowDefinition {
    WorkflowDefinition {
        name: "webpage-visual-loop".into(),
        mode: Some("code".into()),
        steps: vec![
            WorkflowStep {
                id: "prefs".into(),
                prompt: "Collect or reuse visual preferences (colors, density, references).".into(),
                agent: Some("general-purpose".into()),
                depends_on: vec![],
                ..Default::default()
            },
            WorkflowStep {
                id: "design".into(),
                prompt: "Produce design tokens / visual spec from preferences.".into(),
                agent: Some("general-purpose".into()),
                depends_on: vec!["prefs".into()],
                ..Default::default()
            },
            WorkflowStep {
                id: "implement".into(),
                prompt: "Implement the page from the design spec.".into(),
                agent: Some("general-purpose".into()),
                depends_on: vec!["design".into()],
                ..Default::default()
            },
            WorkflowStep {
                id: "visual_verify".into(),
                prompt: "Browser screenshot and contrast/layout checks.".into(),
                agent: Some("general-purpose".into()),
                depends_on: vec!["implement".into()],
                required_gates: vec!["screenshot".into()],
                allowed_tools: vec![
                    "BrowserNavigate".into(),
                    "BrowserScreenshot".into(),
                    "BrowserSnapshot".into(),
                ],
                ..Default::default()
            },
            WorkflowStep {
                id: "build_test".into(),
                prompt: "Run build/test relevant to the page.".into(),
                agent: Some("general-purpose".into()),
                depends_on: vec!["implement".into()],
                parallel_group: Some("verify".into()),
                required_gates: vec!["build".into()],
                ..Default::default()
            },
            WorkflowStep {
                id: "accept".into(),
                prompt: "Final acceptance against deliverables and preferences.".into(),
                agent: Some("general-purpose".into()),
                depends_on: vec!["visual_verify".into(), "build_test".into()],
                required_gates: vec!["acceptance".into()],
                ..Default::default()
            },
        ],
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webpage_recipe_topo_and_ready() {
        let wf = webpage_default_workflow();
        let layers = workflow_topo_layers(&wf).expect("acyclic");
        assert_eq!(layers[0], vec!["prefs".to_string()]);
        let mut cp = WorkflowCheckpoint::new(&wf, "run1");
        let ready = workflow_ready_steps(&wf, &cp);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "prefs");
        cp.mark_passed("prefs", "prefs ok");
        cp.mark_passed("design", "design ok");
        cp.mark_passed("implement", "impl ok");
        let ready2: Vec<_> = workflow_ready_steps(&wf, &cp)
            .into_iter()
            .map(|s| s.id.as_str())
            .collect();
        assert!(ready2.contains(&"visual_verify"));
        assert!(ready2.contains(&"build_test"));
    }
}
