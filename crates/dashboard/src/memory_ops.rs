//! Memory retention preview/apply via in-process memory store.

use anycode_bootstrap::{build_memory_layer, MemoryAttachMode};
use anycode_config::load_config_for_session;
use anycode_core::MemoryType;
use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
struct RetentionRow {
    id: String,
    mem_type: String,
    title: String,
    updated_at: String,
    action: String,
    reason: String,
}

async fn run_memory_prune(dry_run: bool, apply: bool, older_than_days: i64) -> Result<Value> {
    if dry_run == apply {
        anyhow::bail!("choose exactly one of dry_run or apply");
    }
    let config = load_config_for_session(None, false).await?;
    let (store, _) = build_memory_layer(&config, MemoryAttachMode::Exclusive)?;
    let cutoff = chrono::Utc::now() - chrono::Duration::days(older_than_days.max(0));
    let mut rows = Vec::new();
    for mem_type in [
        MemoryType::Project,
        MemoryType::User,
        MemoryType::Feedback,
        MemoryType::Reference,
    ] {
        for memory in store.recall("", mem_type).await? {
            let protect = memory.tags.iter().any(|t| {
                matches!(
                    t.as_str(),
                    "pin" | "pinned" | "important" | "retain" | "provenance"
                )
            });
            let old = memory.updated_at < cutoff;
            let (action, reason) = if protect {
                ("keep", "protected tag")
            } else if old {
                ("delete", "older than retention window")
            } else {
                ("keep", "recently updated")
            };
            if apply && action == "delete" {
                store.delete(&memory.id).await?;
            }
            rows.push(RetentionRow {
                id: memory.id,
                mem_type: format!("{mem_type:?}"),
                title: memory.title,
                updated_at: memory.updated_at.to_rfc3339(),
                action: if dry_run {
                    format!("would_{action}")
                } else {
                    action.to_string()
                },
                reason: reason.to_string(),
            });
        }
    }
    let rows_value = serde_json::to_value(&rows).context("serialize retention rows")?;
    let summary = summarize_retention_rows(&rows_value);
    Ok(serde_json::json!({
        "rows": rows_value,
        "summary": summary,
        "older_than_days": older_than_days.max(0),
    }))
}

pub async fn memory_retention_preview(older_than_days: i64) -> Result<Value> {
    run_memory_prune(true, false, older_than_days).await
}

pub async fn memory_retention_apply(older_than_days: i64) -> Result<Value> {
    run_memory_prune(false, true, older_than_days).await
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
