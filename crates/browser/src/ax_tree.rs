//! Accessibility tree snapshot via CDP (supplements DOM ref walk).

use chromiumoxide::cdp::browser_protocol::accessibility::{AxNode, AxValue};
use chromiumoxide::Page;

const MAX_AX_NODES: usize = 400;

fn ax_value_str(value: &Option<AxValue>) -> String {
    value
        .as_ref()
        .and_then(|v| v.value.as_ref())
        .map(|j| match j {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        })
        .unwrap_or_default()
        .trim()
        .chars()
        .take(120)
        .collect()
}

pub async fn fetch_ax_tree_yaml(page: &Page) -> Option<String> {
    use chromiumoxide::cdp::browser_protocol::accessibility::GetFullAxTreeParams;

    let tree = page.execute(GetFullAxTreeParams::default()).await.ok()?;
    let nodes = tree.nodes.clone();
    if nodes.is_empty() {
        return None;
    }

    let mut children: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    let mut node_by_id: std::collections::HashMap<String, AxNode> =
        std::collections::HashMap::new();
    let mut roots = Vec::new();

    for node in nodes {
        let id = node.node_id.as_ref().to_string();
        if node.ignored {
            continue;
        }
        if let Some(parent) = &node.parent_id {
            children
                .entry(parent.as_ref().to_string())
                .or_default()
                .push(id.clone());
        } else {
            roots.push(id.clone());
        }
        node_by_id.insert(id, node);
    }

    let mut lines = Vec::new();
    let mut count = 0usize;

    fn walk(
        id: &str,
        depth: usize,
        node_by_id: &std::collections::HashMap<String, AxNode>,
        children: &std::collections::HashMap<String, Vec<String>>,
        lines: &mut Vec<String>,
        count: &mut usize,
    ) {
        if *count >= MAX_AX_NODES || depth > 8 {
            return;
        }
        let Some(node) = node_by_id.get(id) else {
            return;
        };
        *count += 1;
        let role = ax_value_str(&node.role);
        let name = ax_value_str(&node.name);
        let indent = "  ".repeat(depth);
        lines.push(format!("{indent}- axref={id} role={role} name=\"{name}\""));
        if let Some(kids) = children.get(id) {
            for kid in kids {
                walk(kid, depth + 1, node_by_id, children, lines, count);
            }
        }
    }

    for root in roots {
        walk(&root, 0, &node_by_id, &children, &mut lines, &mut count);
    }

    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}
