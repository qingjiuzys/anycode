//! `PlanWrite` — hierarchical session plan tree (in-memory, persisted in orchestration snapshot).

use crate::services::ToolServices;
use anycode_core::prelude::*;
use anycode_core::{
    apply_plan_patches, format_plan_tree_summary, plan_tree_all_completed, plan_tree_is_empty,
    rollup_plan_statuses, validate_plan_tree, PlanLimits, PlanNode, PlanNodeKind, PlanPatch,
    PlanStatus, PlanTree, PlanValidationError,
};
use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Instant;

pub struct PlanWriteTool {
    services: Arc<ToolServices>,
}

impl PlanWriteTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self { services }
    }
}

#[derive(Debug, Deserialize)]
struct PlanNodeIn {
    id: String,
    title: String,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    children: Vec<PlanNodeIn>,
    #[serde(default)]
    detail: Option<String>,
    #[serde(default)]
    kind: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PwInput {
    #[serde(default)]
    tree: Option<Vec<PlanNodeIn>>,
    #[serde(default)]
    updates: Option<Vec<PlanPatch>>,
}

fn map_node_in(node: PlanNodeIn) -> Result<PlanNode, CoreError> {
    let status = node
        .status
        .as_deref()
        .and_then(PlanStatus::parse)
        .unwrap_or_default();
    let kind = node.kind.as_deref().and_then(PlanNodeKind::parse);
    let children = node
        .children
        .into_iter()
        .map(map_node_in)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PlanNode {
        id: node.id,
        title: node.title,
        status,
        children,
        detail: node.detail,
        kind,
    })
}

fn map_tree_in(nodes: Vec<PlanNodeIn>) -> Result<PlanTree, CoreError> {
    let roots = nodes
        .into_iter()
        .map(map_node_in)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PlanTree { roots })
}

fn validation_err(e: PlanValidationError) -> String {
    e.message
}

#[async_trait]
impl Tool for PlanWriteTool {
    fn name(&self) -> &str {
        "PlanWrite"
    }

    fn description(&self) -> &str {
        "Update the session hierarchical plan tree. Use `tree` for full replacement or `updates` for incremental changes."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "tree": {
                    "type": "array",
                    "description": "Replace the entire plan tree with these root nodes.",
                    "items": { "$ref": "#/$defs/planNode" }
                },
                "updates": {
                    "type": "array",
                    "description": "Incremental patches: update status/title/detail by id, or add child via parent_id+node.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string" },
                            "parent_id": { "type": "string" },
                            "status": {
                                "type": "string",
                                "enum": ["pending", "in_progress", "completed", "blocked", "failed", "cancelled"]
                            },
                            "title": { "type": "string" },
                            "detail": { "type": "string" },
                            "node": { "$ref": "#/$defs/planNode" }
                        }
                    }
                }
            },
            "$defs": {
                "planNode": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" },
                        "title": { "type": "string" },
                        "status": {
                            "type": "string",
                            "enum": ["pending", "in_progress", "completed", "blocked", "failed", "cancelled"]
                        },
                        "children": {
                            "type": "array",
                            "items": { "$ref": "#/$defs/planNode" }
                        },
                        "detail": { "type": "string" },
                        "kind": {
                            "type": "string",
                            "enum": ["phase", "task", "verify", "checkpoint"]
                        }
                    },
                    "required": ["id", "title"]
                }
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
        let pw: PwInput =
            serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;
        if pw.tree.is_none() && pw.updates.is_none() {
            return Ok(ToolOutput {
                result: serde_json::json!({ "error": "PlanWrite requires `tree` or `updates`" }),
                error: Some("missing_input".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
        let limits = PlanLimits::default_mvp();
        let result = if let Some(tree_in) = pw.tree {
            let mut tree = map_tree_in(tree_in)?;
            if let Err(e) = validate_plan_tree(&tree, &limits) {
                return Ok(ToolOutput {
                    result: serde_json::json!({ "error": validation_err(e) }),
                    error: Some("validation".into()),
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
            rollup_plan_statuses(&mut tree);
            self.services.replace_plan_tree(tree)
        } else {
            let updates = pw.updates.unwrap_or_default();
            let mut tree = self.services.plan_tree();
            if let Err(e) = apply_plan_patches(&mut tree, &updates) {
                return Ok(ToolOutput {
                    result: serde_json::json!({ "error": validation_err(e) }),
                    error: Some("validation".into()),
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
            if let Err(e) = validate_plan_tree(&tree, &limits) {
                return Ok(ToolOutput {
                    result: serde_json::json!({ "error": validation_err(e) }),
                    error: Some("validation".into()),
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
            rollup_plan_statuses(&mut tree);
            self.services.replace_plan_tree(tree)
        };
        let (old, new) = result;
        let summary = format_plan_tree_summary(&new);
        Ok(ToolOutput {
            result: serde_json::json!({
                "oldTree": old,
                "newTree": new,
                "summary": summary,
                "cleared": plan_tree_is_empty(&new),
                "allCompleted": plan_tree_all_completed(&new),
            }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::ToolServices;
    use serde_json::json;

    #[tokio::test]
    async fn plan_write_replaces_tree() {
        let services = Arc::new(ToolServices::default());
        let tool = PlanWriteTool::new(services);
        let out = tool
            .execute(ToolInput {
                name: "PlanWrite".into(),
                input: json!({
                    "tree": [{
                        "id": "root",
                        "title": "Implement feature",
                        "status": "pending",
                        "children": [{
                            "id": "step-1",
                            "title": "Design",
                            "status": "pending"
                        }]
                    }]
                }),
                working_directory: Some(".".into()),
                sandbox_mode: false,
            })
            .await
            .unwrap();
        assert!(out.result["newTree"]["roots"].is_array());
        assert!(out.result["summary"]
            .as_str()
            .unwrap()
            .contains("Implement feature"));
    }
}
