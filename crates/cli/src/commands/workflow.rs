use anycode_core::{PlanValidationIssue, PlanValidationResult, RuntimeMode, WorkflowDefinition};
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub(crate) fn validate(path: &Path, json: bool) -> anyhow::Result<()> {
    let workflow = anycode_tools::workflows::load_workflow_from_file(path)?;
    let result = validate_workflow(&workflow);
    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else if result.ok {
        println!("workflow validate: ok");
    } else {
        println!("workflow validate: failed");
        for issue in &result.issues {
            let step = issue.step_id.as_deref().unwrap_or("<workflow>");
            println!("{} [{}]: {}", issue.severity, step, issue.message);
        }
    }
    if result.ok {
        Ok(())
    } else {
        anyhow::bail!("workflow validation failed")
    }
}

pub(crate) fn validate_workflow_definition(workflow: &WorkflowDefinition) -> PlanValidationResult {
    validate_workflow(workflow)
}

fn validate_workflow(workflow: &WorkflowDefinition) -> PlanValidationResult {
    let mut issues = Vec::new();
    if workflow.name.trim().is_empty() {
        issues.push(issue("error", None, "workflow name is required"));
    }
    let mut ids = HashSet::new();
    for step in &workflow.steps {
        if step.id.trim().is_empty() {
            issues.push(issue("error", None, "step id is required"));
        } else if !ids.insert(step.id.clone()) {
            issues.push(issue("error", Some(&step.id), "duplicate step id"));
        }
        if let Some(mode) = step.mode.as_deref().or(workflow.mode.as_deref()) {
            if RuntimeMode::parse(mode).is_none() {
                issues.push(issue(
                    "error",
                    Some(&step.id),
                    format!("unknown mode {mode}"),
                ));
            }
        }
        if let Some(agent) = step.agent.as_deref() {
            if !known_agent(agent) {
                issues.push(issue(
                    "error",
                    Some(&step.id),
                    format!("unknown agent {agent}"),
                ));
            }
        }
        for tool in &step.allowed_tools {
            if anycode_tools::tool_catalog_entry(tool).is_none() && !tool.starts_with("mcp__") {
                issues.push(issue(
                    "error",
                    Some(&step.id),
                    format!("unknown allowed tool {tool}"),
                ));
            }
        }
        if step.prompt.trim().is_empty() && step.intent.as_deref().unwrap_or("").trim().is_empty() {
            issues.push(issue(
                "warn",
                Some(&step.id),
                "step has neither prompt nor intent",
            ));
        }
        if step
            .parallel_group
            .as_deref()
            .is_some_and(|s| !s.trim().is_empty())
        {
            issues.push(issue(
                "error",
                Some(&step.id),
                "parallel_group is not supported by the sequential local executor",
            ));
        }
        if !step.required_gates.is_empty() {
            issues.push(issue(
                "error",
                Some(&step.id),
                "required_gates is not enforced by the local workflow executor",
            ));
        }
    }

    let id_set: HashSet<&str> = workflow.steps.iter().map(|s| s.id.as_str()).collect();
    let graph: HashMap<&str, Vec<&str>> = workflow
        .steps
        .iter()
        .map(|s| {
            for dep in &s.depends_on {
                if !id_set.contains(dep.as_str()) {
                    issues.push(issue(
                        "error",
                        Some(&s.id),
                        format!("unknown dependency {dep}"),
                    ));
                }
            }
            (
                s.id.as_str(),
                s.depends_on.iter().map(String::as_str).collect(),
            )
        })
        .collect();
    if has_cycle(&graph) {
        issues.push(issue(
            "error",
            None,
            "workflow dependencies contain a cycle",
        ));
    }

    PlanValidationResult {
        ok: !issues.iter().any(|i| i.severity == "error"),
        issues,
    }
}

fn known_agent(agent: &str) -> bool {
    matches!(
        agent,
        "general-purpose" | "workspace" | "goal" | "plan" | "explore" | "code"
    )
}

fn has_cycle(graph: &HashMap<&str, Vec<&str>>) -> bool {
    fn visit<'a>(
        node: &'a str,
        graph: &HashMap<&'a str, Vec<&'a str>>,
        visiting: &mut HashSet<&'a str>,
        visited: &mut HashSet<&'a str>,
    ) -> bool {
        if visited.contains(node) {
            return false;
        }
        if !visiting.insert(node) {
            return true;
        }
        for next in graph.get(node).into_iter().flatten() {
            if visit(next, graph, visiting, visited) {
                return true;
            }
        }
        visiting.remove(node);
        visited.insert(node);
        false
    }

    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    graph
        .keys()
        .any(|node| visit(node, graph, &mut visiting, &mut visited))
}

fn issue(
    severity: impl Into<String>,
    step_id: Option<&str>,
    message: impl Into<String>,
) -> PlanValidationIssue {
    PlanValidationIssue {
        severity: severity.into(),
        step_id: step_id.map(str::to_string),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anycode_core::WorkflowStep;

    #[test]
    fn catches_unknown_dependency_and_tool() {
        let workflow = WorkflowDefinition {
            name: "x".into(),
            steps: vec![WorkflowStep {
                id: "build".into(),
                prompt: "build".into(),
                depends_on: vec!["missing".into()],
                allowed_tools: vec!["Nope".into()],
                ..WorkflowStep::default()
            }],
            ..WorkflowDefinition::default()
        };
        let result = validate_workflow(&workflow);
        assert!(!result.ok);
        assert_eq!(
            result
                .issues
                .iter()
                .filter(|i| i.severity == "error")
                .count(),
            2
        );
    }

    #[test]
    fn catches_dependency_cycles() {
        let workflow = WorkflowDefinition {
            name: "x".into(),
            steps: vec![
                WorkflowStep {
                    id: "a".into(),
                    prompt: "a".into(),
                    depends_on: vec!["b".into()],
                    ..WorkflowStep::default()
                },
                WorkflowStep {
                    id: "b".into(),
                    prompt: "b".into(),
                    depends_on: vec!["a".into()],
                    ..WorkflowStep::default()
                },
            ],
            ..WorkflowDefinition::default()
        };
        let result = validate_workflow(&workflow);
        assert!(!result.ok);
        assert!(result.issues.iter().any(|i| i.message.contains("cycle")));
    }

    #[test]
    fn rejects_schema_only_execution_fields() {
        let workflow = WorkflowDefinition {
            name: "x".into(),
            steps: vec![WorkflowStep {
                id: "gate".into(),
                prompt: "run".into(),
                required_gates: vec!["cargo test".into()],
                parallel_group: Some("p1".into()),
                ..WorkflowStep::default()
            }],
            ..WorkflowDefinition::default()
        };
        let result = validate_workflow(&workflow);
        assert!(!result.ok);
        assert_eq!(
            result
                .issues
                .iter()
                .filter(|i| i.severity == "error")
                .count(),
            2
        );
    }
}
