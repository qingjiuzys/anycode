//! MCP diagnostics (always available; OAuth login stays behind `mcp-oauth`).

use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
struct McpStatusRow {
    name: String,
    status: String,
    detail: String,
}

fn home_path(rel: &str) -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(rel))
}

fn exists_status(path: &Path) -> &'static str {
    if path.exists() {
        "ok"
    } else {
        "missing"
    }
}

fn env_row(name: &str) -> McpStatusRow {
    let set = std::env::var_os(name).is_some();
    McpStatusRow {
        name: format!("env.{name}"),
        status: if set { "configured" } else { "unset" }.into(),
        detail: name.into(),
    }
}

pub(crate) fn print_status(json: bool) -> anyhow::Result<()> {
    let mut rows = vec![
        McpStatusRow {
            name: "policy.reconnect".into(),
            status: "manual".into(),
            detail: "ADR 007: stdio MCP reconnect is controlled and disabled by default".into(),
        },
        env_row("ANYCODE_MCP_COMMAND"),
        env_row("ANYCODE_MCP_SERVERS"),
        env_row("ANYCODE_MCP_RECONNECT"),
    ];
    if let Some(p) = home_path(".anycode/audit/tool-calls.jsonl") {
        rows.push(McpStatusRow {
            name: "audit.tool_calls".into(),
            status: exists_status(&p).into(),
            detail: p.display().to_string(),
        });
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&rows)?);
    } else {
        for row in &rows {
            println!("{}: {} — {}", row.name, row.status, row.detail);
        }
    }
    Ok(())
}
