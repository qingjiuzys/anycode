//! YAML workflow loading helpers.

use anycode_core::WorkflowDefinition;
use std::path::{Path, PathBuf};

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
