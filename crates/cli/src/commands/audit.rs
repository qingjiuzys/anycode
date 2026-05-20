//! Tool governance audit log queries.

use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
struct AuditRow {
    line_no: usize,
    raw: serde_json::Value,
}

fn audit_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".anycode/audit/tool-calls.jsonl"))
}

pub(crate) fn print_tail(
    task: Option<String>,
    tool: Option<String>,
    limit: usize,
    json: bool,
) -> anyhow::Result<()> {
    let Some(path) = audit_path() else {
        if json {
            println!("[]");
        }
        return Ok(());
    };
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let mut rows = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let raw: serde_json::Value = serde_json::from_str(line).unwrap_or_else(|_| {
            serde_json::json!({
                "parse_error": true,
                "line": line,
            })
        });
        if let Some(ref task_id) = task {
            if raw.get("task_id").and_then(|v| v.as_str()) != Some(task_id.as_str()) {
                continue;
            }
        }
        if let Some(ref tool_name) = tool {
            if raw.get("tool_name").and_then(|v| v.as_str()) != Some(tool_name.as_str()) {
                continue;
            }
        }
        rows.push(AuditRow {
            line_no: idx + 1,
            raw,
        });
    }
    let keep = limit.max(1);
    if rows.len() > keep {
        rows = rows.split_off(rows.len() - keep);
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&rows)?);
    } else if rows.is_empty() {
        println!("No tool audit entries found at {}.", path.display());
    } else {
        for row in rows {
            println!("#{} {}", row.line_no, row.raw);
        }
    }
    Ok(())
}
