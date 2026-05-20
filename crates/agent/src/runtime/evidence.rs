//! Minimal evidence index for long-horizon context recovery.

use anycode_core::prelude::*;
use chrono::Utc;
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::fs::OpenOptions;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
struct EvidenceRow<'a> {
    ts: String,
    task_id: String,
    tool_name: &'a str,
    content_hash: String,
    preview: String,
}

fn evidence_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".anycode/memory/evidence.jsonl"))
}

fn hash_text(s: &str) -> String {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    format!("{:016x}", h.finish())
}

fn is_evidence_tool(name: &str) -> bool {
    matches!(
        name,
        "FileRead" | "Read" | "Grep" | "Glob" | "WebFetch" | "WebSearch" | "mcp"
    ) || name.starts_with("mcp__")
}

pub(crate) fn append_tool_evidence(task_id: TaskId, tool_name: &str, content: &str) {
    if !is_evidence_tool(tool_name) || content.trim().is_empty() {
        return;
    }
    let Some(path) = evidence_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let row = EvidenceRow {
        ts: Utc::now().to_rfc3339(),
        task_id: task_id.to_string(),
        tool_name,
        content_hash: hash_text(content),
        preview: content.chars().take(500).collect(),
    };
    let Ok(line) = serde_json::to_string(&row) else {
        return;
    };
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::is_evidence_tool;

    #[test]
    fn evidence_tools_include_mcp_prefixes() {
        assert!(is_evidence_tool("mcp__github__get_issue"));
        assert!(is_evidence_tool("WebFetch"));
        assert!(!is_evidence_tool("Bash"));
    }
}
