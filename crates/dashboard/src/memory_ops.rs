//! Memory retention preview/apply via CLI subprocess (avoids sled lock coupling in dashboard).

use anyhow::{Context, Result};
use serde_json::Value;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;

fn resolve_anycode_bin() -> PathBuf {
    if let Ok(p) = std::env::var("ANYCODE_BIN") {
        let path = PathBuf::from(p);
        if path.is_file() {
            return path;
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if exe.is_file() {
            return exe;
        }
    }
    PathBuf::from("anycode")
}

pub async fn memory_retention_preview(older_than_days: i64) -> Result<Value> {
    run_memory_prune(true, false, older_than_days).await
}

pub async fn memory_retention_apply(older_than_days: i64) -> Result<Value> {
    run_memory_prune(false, true, older_than_days).await
}

async fn run_memory_prune(dry_run: bool, apply: bool, older_than_days: i64) -> Result<Value> {
    let bin = resolve_anycode_bin();
    let mut cmd = Command::new(&bin);
    cmd.arg("memory").arg("prune");
    if dry_run {
        cmd.arg("--dry-run");
    }
    if apply {
        cmd.arg("--apply");
    }
    cmd.args([
        "--json",
        "--older-than-days",
        &older_than_days.max(0).to_string(),
    ]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let out = cmd
        .output()
        .await
        .with_context(|| format!("spawn {}", bin.display()))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!("memory prune failed ({}): {}", out.status, stderr.trim());
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let rows: Value = serde_json::from_str(stdout.trim())
        .or_else(|_| {
            // CLI may print non-JSON warnings before JSON array.
            let start = stdout.find('[').or_else(|| stdout.find('{'));
            start
                .map(|i| serde_json::from_str(stdout[i..].trim()))
                .transpose()
        })
        .context("parse memory prune JSON")?
        .unwrap_or_else(|| Value::Array(vec![]));
    let summary = summarize_retention_rows(&rows);
    Ok(serde_json::json!({
        "rows": rows,
        "summary": summary,
        "older_than_days": older_than_days.max(0),
    }))
}

fn summarize_retention_rows(rows: &Value) -> Value {
    let mut would_delete = 0i64;
    let mut keep = 0i64;
    let mut protected = 0i64;
    let Some(arr) = rows.as_array() else {
        return serde_json::json!({ "would_delete": 0, "keep": 0, "protected": 0 });
    };
    for row in arr {
        let action = row.get("action").and_then(|x| x.as_str()).unwrap_or("");
        let reason = row.get("reason").and_then(|x| x.as_str()).unwrap_or("");
        if action.contains("delete") {
            would_delete += 1;
        } else if reason.contains("protected") {
            protected += 1;
        } else {
            keep += 1;
        }
    }
    serde_json::json!({
        "would_delete": would_delete,
        "keep": keep,
        "protected": protected,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarizes_rows() {
        let rows = serde_json::json!([
            {"action": "would_delete", "reason": "older than retention window"},
            {"action": "keep", "reason": "protected tag"},
            {"action": "keep", "reason": "recently updated"}
        ]);
        let s = summarize_retention_rows(&rows);
        assert_eq!(s["would_delete"], 1);
        assert_eq!(s["protected"], 1);
    }
}
