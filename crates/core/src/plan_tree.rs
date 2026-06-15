//! Hierarchical plan tree for structured agent planning.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Default maximum nesting depth for plan trees.
pub const PLAN_TREE_MAX_DEPTH: usize = 4;
/// Default maximum node count for plan trees.
pub const PLAN_TREE_MAX_NODES: usize = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    #[default]
    Pending,
    InProgress,
    Completed,
    Blocked,
    Failed,
    Cancelled,
}

impl PlanStatus {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "pending" => Some(Self::Pending),
            "in_progress" | "inprogress" | "running" => Some(Self::InProgress),
            "completed" | "done" => Some(Self::Completed),
            "blocked" => Some(Self::Blocked),
            "failed" => Some(Self::Failed),
            "cancelled" | "canceled" => Some(Self::Cancelled),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Blocked => "blocked",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn glyph(self) -> &'static str {
        match self {
            Self::Pending => "[ ]",
            Self::InProgress => "[~]",
            Self::Completed => "[x]",
            Self::Blocked => "[!]",
            Self::Failed => "[X]",
            Self::Cancelled => "[-]",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanNodeKind {
    Phase,
    Task,
    Verify,
    Checkpoint,
}

impl PlanNodeKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "phase" => Some(Self::Phase),
            "task" => Some(Self::Task),
            "verify" => Some(Self::Verify),
            "checkpoint" => Some(Self::Checkpoint),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlanNode {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub status: PlanStatus,
    #[serde(default)]
    pub children: Vec<PlanNode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<PlanNodeKind>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlanTree {
    #[serde(default)]
    pub roots: Vec<PlanNode>,
}

#[derive(Debug, Clone)]
pub struct PlanValidationError {
    pub message: String,
}

impl PlanValidationError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlanLimits {
    pub max_depth: usize,
    pub max_nodes: usize,
}

impl PlanLimits {
    pub fn default_mvp() -> Self {
        Self {
            max_depth: PLAN_TREE_MAX_DEPTH,
            max_nodes: PLAN_TREE_MAX_NODES,
        }
    }
}

/// Validate tree shape: unique ids, non-empty titles, depth and node limits.
pub fn validate_plan_tree(tree: &PlanTree, limits: &PlanLimits) -> Result<(), PlanValidationError> {
    let mut ids = HashSet::new();
    let mut count = 0usize;
    for root in &tree.roots {
        validate_node(root, 1, limits, &mut ids, &mut count)?;
    }
    Ok(())
}

fn validate_node(
    node: &PlanNode,
    depth: usize,
    limits: &PlanLimits,
    ids: &mut HashSet<String>,
    count: &mut usize,
) -> Result<(), PlanValidationError> {
    if depth > limits.max_depth {
        return Err(PlanValidationError::new(format!(
            "plan tree exceeds max depth {} at node '{}'",
            limits.max_depth, node.id
        )));
    }
    *count += 1;
    if *count > limits.max_nodes {
        return Err(PlanValidationError::new(format!(
            "plan tree exceeds max node count {}",
            limits.max_nodes
        )));
    }
    let id = node.id.trim();
    if id.is_empty() {
        return Err(PlanValidationError::new("plan node id must not be empty"));
    }
    if !ids.insert(id.to_string()) {
        return Err(PlanValidationError::new(format!(
            "duplicate plan node id '{}'",
            id
        )));
    }
    if node.title.trim().is_empty() {
        return Err(PlanValidationError::new(format!(
            "plan node '{}' title must not be empty",
            id
        )));
    }
    for child in &node.children {
        validate_node(child, depth + 1, limits, ids, count)?;
    }
    Ok(())
}

/// Roll up parent status from children when parent is not terminal.
pub fn rollup_plan_statuses(tree: &mut PlanTree) {
    for root in &mut tree.roots {
        rollup_node(root);
    }
}

fn rollup_node(node: &mut PlanNode) {
    for child in &mut node.children {
        rollup_node(child);
    }
    if node.children.is_empty() {
        return;
    }
    if matches!(
        node.status,
        PlanStatus::Completed | PlanStatus::Failed | PlanStatus::Cancelled
    ) {
        return;
    }
    let child_statuses: Vec<PlanStatus> = node.children.iter().map(|c| c.status).collect();
    node.status = rollup_from_children(&child_statuses);
}

fn rollup_from_children(children: &[PlanStatus]) -> PlanStatus {
    if children.iter().any(|s| matches!(s, PlanStatus::Failed)) {
        return PlanStatus::Failed;
    }
    if children.iter().any(|s| matches!(s, PlanStatus::Blocked)) {
        return PlanStatus::Blocked;
    }
    if children.iter().any(|s| matches!(s, PlanStatus::InProgress)) {
        return PlanStatus::InProgress;
    }
    if children.iter().all(|s| matches!(s, PlanStatus::Completed)) {
        return PlanStatus::Completed;
    }
    if children
        .iter()
        .all(|s| matches!(s, PlanStatus::Completed | PlanStatus::Cancelled))
    {
        return PlanStatus::Completed;
    }
    PlanStatus::Pending
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlanPatch {
    pub id: Option<String>,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub node: Option<PlanNode>,
}

/// Apply incremental patches to an existing tree.
pub fn apply_plan_patches(
    tree: &mut PlanTree,
    patches: &[PlanPatch],
) -> Result<(), PlanValidationError> {
    for patch in patches {
        if let Some(node) = &patch.node {
            if let Some(parent_id) = patch.parent_id.as_deref() {
                if !insert_child(tree, parent_id, node.clone())? {
                    return Err(PlanValidationError::new(format!(
                        "parent node '{}' not found for patch",
                        parent_id
                    )));
                }
            } else if patch.id.is_none() {
                tree.roots.push(node.clone());
            } else {
                return Err(PlanValidationError::new(
                    "patch with `node` requires `parent_id` or omit `id` to append root",
                ));
            }
            continue;
        }
        let Some(id) = patch.id.as_deref() else {
            return Err(PlanValidationError::new(
                "patch requires `id` when updating status/title/detail",
            ));
        };
        let Some(node) = find_node_mut(tree, id) else {
            return Err(PlanValidationError::new(format!(
                "plan node '{}' not found",
                id
            )));
        };
        if let Some(status) = &patch.status {
            node.status = PlanStatus::parse(status).ok_or_else(|| {
                PlanValidationError::new(format!("invalid plan status '{}'", status))
            })?;
        }
        if let Some(title) = &patch.title {
            if title.trim().is_empty() {
                return Err(PlanValidationError::new(format!(
                    "plan node '{}' title must not be empty",
                    id
                )));
            }
            node.title = title.trim().to_string();
        }
        if let Some(detail) = patch.detail.as_ref() {
            let trimmed = detail.trim();
            node.detail = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            };
        }
    }
    Ok(())
}

fn find_node_mut<'a>(tree: &'a mut PlanTree, id: &str) -> Option<&'a mut PlanNode> {
    for root in &mut tree.roots {
        if let Some(found) = find_in_node_mut(root, id) {
            return Some(found);
        }
    }
    None
}

fn find_in_node_mut<'a>(node: &'a mut PlanNode, id: &str) -> Option<&'a mut PlanNode> {
    if node.id == id {
        return Some(node);
    }
    for child in &mut node.children {
        if let Some(found) = find_in_node_mut(child, id) {
            return Some(found);
        }
    }
    None
}

fn insert_child(
    tree: &mut PlanTree,
    parent_id: &str,
    child: PlanNode,
) -> Result<bool, PlanValidationError> {
    if let Some(parent) = find_node_mut(tree, parent_id) {
        parent.children.push(child);
        return Ok(true);
    }
    Ok(false)
}

/// Prefix for hidden runtime context messages that carry the active plan tree.
pub const PLAN_TREE_CONTEXT_PREFIX: &str = "## Active Plan Tree";

/// Compact text summary for LLM context reinjection.
pub fn format_plan_tree_summary(tree: &PlanTree) -> String {
    if tree.roots.is_empty() {
        return String::new();
    }
    let mut out = String::from(PLAN_TREE_CONTEXT_PREFIX);
    out.push_str("\n\n");
    for root in &tree.roots {
        format_node_summary(root, 0, &mut out);
    }
    out.trim_end().to_string()
}

fn format_node_summary(node: &PlanNode, depth: usize, out: &mut String) {
    let indent = "  ".repeat(depth);
    out.push_str(&indent);
    out.push_str(node.status.glyph());
    out.push(' ');
    out.push_str(&node.title);
    out.push_str(" (");
    out.push_str(node.id.as_str());
    out.push_str(")\n");
    for child in &node.children {
        format_node_summary(child, depth + 1, out);
    }
}

/// Terminal-friendly indented tree with status glyphs.
pub fn format_plan_tree_terminal(tree: &PlanTree, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let inner = width.saturating_sub(4).max(16);
    for (i, root) in tree.roots.iter().enumerate() {
        let is_last_root = i + 1 == tree.roots.len();
        format_node_terminal(root, "", is_last_root, inner, &mut lines);
    }
    lines
}

fn format_node_terminal(
    node: &PlanNode,
    prefix: &str,
    is_last: bool,
    width: usize,
    lines: &mut Vec<String>,
) {
    let branch = if is_last { "└─ " } else { "├─ " };
    let cont = if is_last { "   " } else { "│  " };
    let title = truncate_chars(
        &node.title,
        width.saturating_sub(prefix.len() + branch.len() + 6),
    );
    let line = format!(
        "{}{}{} {} {}",
        prefix,
        branch,
        node.status.glyph(),
        title,
        node.id
    );
    lines.push(line);
    let child_prefix = format!("{}{}", prefix, cont);
    for (i, child) in node.children.iter().enumerate() {
        let last = i + 1 == node.children.len();
        format_node_terminal(child, &child_prefix, last, width, lines);
    }
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    format!(
        "{}…",
        s.chars().take(max.saturating_sub(1)).collect::<String>()
    )
}

pub fn plan_tree_is_empty(tree: &PlanTree) -> bool {
    tree.roots.is_empty()
}

pub fn plan_tree_all_completed(tree: &PlanTree) -> bool {
    !tree.roots.is_empty() && tree.roots.iter().all(|n| subtree_all_completed(n))
}

fn subtree_all_completed(node: &PlanNode) -> bool {
    matches!(node.status, PlanStatus::Completed) && node.children.iter().all(subtree_all_completed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tree() -> PlanTree {
        PlanTree {
            roots: vec![PlanNode {
                id: "phase-1".into(),
                title: "Research".into(),
                status: PlanStatus::Pending,
                children: vec![PlanNode {
                    id: "task-1".into(),
                    title: "Read code".into(),
                    status: PlanStatus::Completed,
                    children: vec![],
                    detail: None,
                    kind: Some(PlanNodeKind::Task),
                }],
                detail: None,
                kind: Some(PlanNodeKind::Phase),
            }],
        }
    }

    #[test]
    fn validate_rejects_duplicate_ids() {
        let tree = PlanTree {
            roots: vec![
                PlanNode {
                    id: "a".into(),
                    title: "A".into(),
                    status: PlanStatus::Pending,
                    children: vec![],
                    detail: None,
                    kind: None,
                },
                PlanNode {
                    id: "a".into(),
                    title: "B".into(),
                    status: PlanStatus::Pending,
                    children: vec![],
                    detail: None,
                    kind: None,
                },
            ],
        };
        let err = validate_plan_tree(&tree, &PlanLimits::default_mvp()).unwrap_err();
        assert!(err.message.contains("duplicate"));
    }

    #[test]
    fn rollup_marks_parent_completed() {
        let mut tree = sample_tree();
        tree.roots[0].children[0].status = PlanStatus::Completed;
        rollup_plan_statuses(&mut tree);
        assert_eq!(tree.roots[0].status, PlanStatus::Completed);
    }

    #[test]
    fn apply_patch_updates_status() {
        let mut tree = sample_tree();
        apply_plan_patches(
            &mut tree,
            &[PlanPatch {
                id: Some("task-1".into()),
                parent_id: None,
                status: Some("in_progress".into()),
                title: None,
                detail: None,
                node: None,
            }],
        )
        .unwrap();
        assert_eq!(tree.roots[0].children[0].status, PlanStatus::InProgress);
    }

    #[test]
    fn summary_and_terminal_formats() {
        let tree = sample_tree();
        let summary = format_plan_tree_summary(&tree);
        assert!(summary.contains("## Active Plan Tree"));
        assert!(summary.contains("Research"));
        let lines = format_plan_tree_terminal(&tree, 80);
        assert!(!lines.is_empty());
        assert!(lines[0].contains("Research"));
    }
}
