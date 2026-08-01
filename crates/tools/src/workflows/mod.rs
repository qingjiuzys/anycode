//! Workflow 正式工具 + YAML 加载 helper。
//!
//! - `WorkflowGet`：发现/读取工作区 `workflow.yml`（或指定 `script` / `scriptPath`），
//!   解析为 [`WorkflowDefinition`]，校验 DAG（`workflow_topo_layers`）并返回结构化摘要。
//!   执行仍由 scheduler / channel-bridge 负责（ADR 014 §6），本工具只读。

use crate::paths::resolve_path_fields;
use anycode_core::{
    workflow_topo_layers, PlanValidationIssue, PlanValidationResult, WorkflowCheckpoint,
    WorkflowDefinition, WorkflowStepStatus,
};
use anycode_core::{CoreError, PermissionMode, SecurityPolicy, Tool, ToolInput, ToolOutput};
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub fn default_workflow_candidates(base_dir: &Path) -> Vec<PathBuf> {
    vec![
        base_dir.join("workflow.yml"),
        base_dir.join("workflow.yaml"),
        base_dir.join(".anycode/workflow.yml"),
        base_dir.join(".anycode/workflow.yaml"),
    ]
}

pub fn load_workflow_from_file(path: &Path) -> anyhow::Result<WorkflowDefinition> {
    let text = std::fs::read_to_string(path)?;
    let workflow = serde_yaml::from_str::<WorkflowDefinition>(&text)?;
    Ok(workflow)
}

pub fn discover_workflow(base_dir: &Path) -> anyhow::Result<Option<(PathBuf, WorkflowDefinition)>> {
    for candidate in default_workflow_candidates(base_dir) {
        if candidate.is_file() {
            let workflow = load_workflow_from_file(&candidate)?;
            return Ok(Some((candidate, workflow)));
        }
    }
    Ok(None)
}

/// 轻量校验：name/step id 必填与唯一、depends_on 已知、DAG 无环（拓扑层）。
pub fn validate_workflow(workflow: &WorkflowDefinition) -> PlanValidationResult {
    let mut issues = Vec::new();
    if workflow.name.trim().is_empty() {
        issues.push(issue("error", None, "workflow name is required"));
    }
    let mut ids: HashSet<&str> = HashSet::new();
    for step in &workflow.steps {
        if step.id.trim().is_empty() {
            issues.push(issue("error", None, "step id is required"));
        } else if !ids.insert(step.id.as_str()) {
            issues.push(issue("error", Some(step.id.as_str()), "duplicate step id"));
        }
        if step.prompt.trim().is_empty()
            && step
                .intent
                .as_deref()
                .map(str::trim)
                .unwrap_or("")
                .is_empty()
        {
            issues.push(issue(
                "warn",
                Some(step.id.as_str()),
                "step has neither prompt nor intent",
            ));
        }
        for dep in &step.depends_on {
            if !ids.contains(dep.as_str()) {
                issues.push(issue(
                    "error",
                    Some(step.id.as_str()),
                    format!("unknown dependency {dep}"),
                ));
            }
        }
    }
    match workflow_topo_layers(workflow) {
        Ok(_) => {}
        Err(v) => issues.extend(v.issues),
    }
    PlanValidationResult {
        ok: !issues.iter().any(|i| i.severity == "error"),
        issues,
    }
}

fn issue(severity: &str, step_id: Option<&str>, message: impl Into<String>) -> PlanValidationIssue {
    PlanValidationIssue {
        severity: severity.into(),
        step_id: step_id.map(str::to_string),
        message: message.into(),
    }
}

/// 校验结果的 JSON 视图（与 `PlanValidationResult` 序列化保持一致，供工具输出）。
fn validation_json(result: &PlanValidationResult) -> serde_json::Value {
    serde_json::json!({
        "ok": result.ok,
        "issues": result.issues.iter().map(|i| {
            serde_json::json!({
                "severity": i.severity,
                "stepId": i.step_id,
                "message": i.message,
            })
        }).collect::<Vec<_>>(),
    })
}

/// 步骤的摘要视图（避免把整个 prompt / vars 全量返回给模型）。
fn step_json(workflow: &WorkflowDefinition) -> Vec<serde_json::Value> {
    workflow
        .steps
        .iter()
        .map(|s| {
            serde_json::json!({
                "id": s.id,
                "prompt": s.prompt,
                "intent": s.intent,
                "agent": s.agent,
                "mode": s.mode,
                "model": s.model,
                "dependsOn": s.depends_on,
                "parallelGroup": s.parallel_group,
                "budget": s.budget,
                "requiredGates": s.required_gates,
                "allowedTools": s.allowed_tools,
                "doneWhen": s.done_when,
                "vars": s.vars,
            })
        })
        .collect()
}

/// 读取 `.anycode/workflow-checkpoints/<name>.json`（channel-bridge 持久化的可恢复进度）。
fn checkpoint_summary(base_dir: &Path, workflow_name: &str) -> Option<serde_json::Value> {
    let path = base_dir
        .join(".anycode")
        .join("workflow-checkpoints")
        .join(format!("{}.json", workflow_name.replace('/', "_")));
    if !path.is_file() {
        return None;
    }
    let text = std::fs::read_to_string(&path).ok()?;
    let cp: WorkflowCheckpoint = serde_json::from_str(&text).ok()?;
    let steps: Vec<serde_json::Value> = cp
        .steps
        .iter()
        .map(|(_, st)| {
            serde_json::json!({
                "stepId": st.step_id,
                "status": status_str(st.status),
                "attempts": st.attempts,
                "artifactSummary": st.artifact_summary,
                "lastError": st.last_error,
            })
        })
        .collect::<Vec<_>>();
    Some(serde_json::json!({
        "runId": cp.run_id,
        "name": cp.workflow_name,
        "steps": steps,
        "completedOrder": cp.completed_order,
        "version": cp.version,
    }))
}

fn status_str(status: WorkflowStepStatus) -> &'static str {
    match status {
        WorkflowStepStatus::Pending => "pending",
        WorkflowStepStatus::Ready => "ready",
        WorkflowStepStatus::Running => "running",
        WorkflowStepStatus::Passed => "passed",
        WorkflowStepStatus::Failed => "failed",
        WorkflowStepStatus::Skipped => "skipped",
    }
}

// --- WorkflowGet ---

#[derive(Deserialize)]
struct WfGetIn {
    #[serde(default, alias = "scriptPath")]
    script_path: Option<String>,
    #[serde(default)]
    script: Option<String>,
}

pub struct WorkflowGetTool {
    pub sandbox_mode: bool,
}

impl WorkflowGetTool {
    pub fn new(sandbox_mode: bool) -> Self {
        Self { sandbox_mode }
    }
}

#[async_trait]
impl Tool for WorkflowGetTool {
    fn name(&self) -> &str {
        "WorkflowGet"
    }

    fn description(&self) -> &str {
        "Discover and read the workspace workflow definition (workflow.yml / workflow.yaml / .anycode/*), or parse an inline `script` / explicit `scriptPath`. Returns parsed steps, DAG layers, validation issues, and checkpoint progress. Read-only; execution is handled by the scheduler."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "scriptPath": { "type": "string", "description": "Optional explicit workflow YAML file path (absolute or relative to the working directory). When omitted, the standard candidates (workflow.yml, workflow.yaml, .anycode/workflow.yml, .anycode/workflow.yaml) are scanned under the working directory." },
                "script": { "type": "string", "description": "Optional inline YAML workflow definition to parse instead of reading a file." }
            }
        })
    }

    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Auto
    }

    fn security_policy(&self) -> Option<&SecurityPolicy> {
        None
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let start = Instant::now();
        let wd = input.working_directory.as_deref();
        let sandbox_in = input.sandbox_mode;
        let r: WfGetIn =
            serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;

        // 1) 定位 workflow：script（内联）→ scriptPath（显式文件）→ 工作目录发现。
        let (path, workflow) = if let Some(script) = r.script.as_deref() {
            if script.trim().is_empty() {
                return Ok(tool_fail(
                    start,
                    serde_json::json!({ "error": "script is empty" }),
                    "script is empty",
                ));
            }
            let wf: WorkflowDefinition = serde_yaml::from_str(script)
                .map_err(|e| CoreError::Other(anyhow::anyhow!("invalid workflow YAML: {e}")))?;
            (None, wf)
        } else if let Some(script_path) = r.script_path.as_deref() {
            let p = resolve_path_fields(self.sandbox_mode, sandbox_in, wd, script_path)?;
            let wf = load_workflow_from_file(&p)
                .map_err(|e| CoreError::Other(anyhow::anyhow!("load workflow: {e}")))?;
            (Some(p), wf)
        } else {
            let base = wd
                .map(PathBuf::from)
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
            match discover_workflow(&base) {
                Ok(Some((found, wf))) => (Some(found), wf),
                Ok(None) => {
                    return Ok(ToolOutput {
                        result: serde_json::json!({
                            "found": false,
                            "error": "no workflow definition found (looked for workflow.yml, workflow.yaml, .anycode/workflow.yml, .anycode/workflow.yaml)"
                        }),
                        error: Some("no workflow definition found".into()),
                        duration_ms: start.elapsed().as_millis() as u64,
                    });
                }
                Err(e) => {
                    return Ok(tool_fail(
                        start,
                        serde_json::json!({ "error": format!("discover workflow: {e}") }),
                        "discover workflow failed",
                    ));
                }
            }
        };

        let validation = validate_workflow(&workflow);
        let layers = workflow_topo_layers(&workflow).ok();
        let base = wd
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let checkpoint = checkpoint_summary(&base, &workflow.name);

        let result = serde_json::json!({
            "found": true,
            "path": path.as_ref().map(|p| p.to_string_lossy().to_string()),
            "name": workflow.name,
            "trigger": workflow.trigger,
            "mode": workflow.mode,
            "model": workflow.model,
            "retry": workflow.retry,
            "doneWhen": workflow.done_when,
            "handoff": workflow.handoff,
            "steps": step_json(&workflow),
            "dagLayers": layers.unwrap_or_default(),
            "validation": validation_json(&validation),
            "checkpoint": checkpoint,
        });

        Ok(ToolOutput {
            result,
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

fn tool_fail(start: Instant, result: serde_json::Value, error: &str) -> ToolOutput {
    ToolOutput {
        result,
        error: Some(error.to_string()),
        duration_ms: start.elapsed().as_millis() as u64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_default_candidates() {
        let base = Path::new("/tmp/demo");
        let names: Vec<String> = default_workflow_candidates(base)
            .into_iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(
            names,
            vec![
                "workflow.yml",
                "workflow.yaml",
                "workflow.yml",
                "workflow.yaml"
            ]
        );
    }

    #[test]
    fn parses_workflow_yaml() {
        let yaml = r#"
name: demo
mode: code
steps:
  - id: a
    prompt: do a
  - id: b
    prompt: do b
    depends_on: [a]
"#;
        let wf: WorkflowDefinition = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(wf.name, "demo");
        assert_eq!(wf.steps.len(), 2);
        assert_eq!(wf.steps[1].depends_on, vec!["a".to_string()]);
    }

    #[test]
    fn validates_dag_and_cycles() {
        let good = WorkflowDefinition {
            name: "good".into(),
            steps: vec![
                anycode_core::WorkflowStep {
                    id: "a".into(),
                    prompt: "a".into(),
                    depends_on: vec![],
                    ..Default::default()
                },
                anycode_core::WorkflowStep {
                    id: "b".into(),
                    prompt: "b".into(),
                    depends_on: vec!["a".into()],
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        assert!(validate_workflow(&good).ok);

        let cyclic = WorkflowDefinition {
            name: "cyc".into(),
            steps: vec![
                anycode_core::WorkflowStep {
                    id: "a".into(),
                    prompt: "a".into(),
                    depends_on: vec!["b".into()],
                    ..Default::default()
                },
                anycode_core::WorkflowStep {
                    id: "b".into(),
                    prompt: "b".into(),
                    depends_on: vec!["a".into()],
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let res = validate_workflow(&cyclic);
        assert!(!res.ok);
        assert!(res.issues.iter().any(|i| i.message.contains("cycle")));
    }

    #[tokio::test]
    async fn get_with_inline_script() {
        let tool = WorkflowGetTool::new(false);
        let out = tool
            .execute(ToolInput {
                name: "WorkflowGet".into(),
                input: serde_json::json!({
                    "script": "name: s\nsteps:\n  - id: x\n    prompt: run x\n"
                }),
                working_directory: None,
                sandbox_mode: false,
            })
            .await
            .unwrap();
        assert!(out.error.is_none());
        assert_eq!(out.result["found"], true);
        assert_eq!(out.result["name"], "s");
        assert_eq!(out.result["steps"][0]["id"], "x");
        assert_eq!(out.result["validation"]["ok"], true);
    }
}
