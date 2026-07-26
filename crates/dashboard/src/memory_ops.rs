//! Memory retention preview/apply via in-process memory store.

use anycode_bootstrap::{build_memory_layer, MemoryAttachMode};
use anycode_config::load_config_for_session;
use anycode_core::MemoryType;
use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;

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

fn memory_base_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".anycode/memory")
}

/// Memory center snapshot: preferences, facts, pending episodes, dream history, conflicts.
pub async fn memory_center_snapshot() -> Result<Value> {
    let config = load_config_for_session(None, false).await?;
    let (store, _) = build_memory_layer(&config, MemoryAttachMode::Exclusive)?;
    let base = memory_base_dir();
    let episodes = anycode_memory::load_pending_episodes(&base).unwrap_or_default();
    let mut preferences = Vec::new();
    let mut facts = Vec::new();
    let mut feedback = Vec::new();
    let mut existing = Vec::new();
    for mem in store.recall("", MemoryType::User).await? {
        preferences.push(serde_json::json!({
            "id": mem.id,
            "title": mem.title,
            "content": mem.content,
            "kind": mem.meta.as_ref().map(|m| m.kind.as_str()).unwrap_or("preference"),
            "evidence_hash": mem.meta.as_ref().map(|m| m.evidence_hash.clone()).unwrap_or_default(),
            "pinned": mem.meta.as_ref().map(|m| m.pinned).unwrap_or(false),
        }));
        existing.push(mem);
    }
    for mem in store.recall("", MemoryType::Project).await? {
        facts.push(serde_json::json!({
            "id": mem.id,
            "title": mem.title,
            "content": mem.content,
            "kind": mem.meta.as_ref().map(|m| m.kind.as_str()).unwrap_or("fact"),
            "evidence_hash": mem.meta.as_ref().map(|m| m.evidence_hash.clone()).unwrap_or_default(),
            "tags": mem.tags,
        }));
        existing.push(mem);
    }
    for mem in store.recall("", MemoryType::Feedback).await? {
        feedback.push(serde_json::json!({
            "id": mem.id,
            "title": mem.title,
            "content": mem.content,
        }));
        existing.push(mem);
    }
    let preview = anycode_memory::consolidate_episodes(
        &episodes,
        &existing,
        &anycode_memory::DreamEngineSettings::default(),
    );
    let dream_log = std::fs::read_to_string(anycode_memory::dream_log_path(&base))
        .unwrap_or_default()
        .lines()
        .rev()
        .take(20)
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .collect::<Vec<_>>();
    let sync = anycode_memory::load_sync_state(&base);
    Ok(serde_json::json!({
        "sync_mode": if sync.enabled { "encrypted_sync" } else { "local_only" },
        "preferences": preferences,
        "project_facts": facts,
        "feedback": feedback,
        "pending_episodes": episodes.len(),
        "episode_previews": episodes.iter().rev().take(20).map(|e| serde_json::json!({
            "id": e.id,
            "task_id": e.task_id,
            "evidence_hash": e.evidence_hash,
            "events": e.events.len(),
            "created_at": e.created_at,
        })).collect::<Vec<_>>(),
        "dream_preview": {
            "promotions": preview.promoted.len(),
            "conflicts": preview.conflicts,
            "skipped_secrets": preview.skipped_secrets,
            "duplicates_merged": preview.duplicates_merged,
        },
        "dream_history": dream_log,
        "e2ee": {
            "sync_enabled": sync.enabled,
            "device_id": sync.device_id,
            "last_sync_at": sync.last_sync_at,
            "keychain_service": anycode_memory::KEYCHAIN_SERVICE,
            "mode_label": if sync.enabled { "encrypted_sync" } else { "local_only" },
        },
    }))
}

/// Run local dream consolidation and optionally persist promotions.
pub async fn memory_dream_run(apply: bool) -> Result<Value> {
    let config = load_config_for_session(None, false).await?;
    let (store, _) = build_memory_layer(&config, MemoryAttachMode::Exclusive)?;
    let base = memory_base_dir();
    let episodes = anycode_memory::load_pending_episodes(&base).unwrap_or_default();
    let mut existing = Vec::new();
    for mt in MemoryType::ALL {
        existing.extend(store.recall("", mt).await?);
    }
    let report = anycode_memory::consolidate_episodes(
        &episodes,
        &existing,
        &anycode_memory::DreamEngineSettings::default(),
    );
    if apply {
        for mem in &report.promoted {
            store.save(mem.clone()).await?;
        }
        // Apply decay-forgetting named by the report.
        for id in &report.forgotten_ids {
            store.delete(id).await?;
        }
        let _ = anycode_memory::append_dream_report(&base, &report);
        // Archive processed episodes.
        let arch = base.join("episodes_done");
        let _ = std::fs::create_dir_all(&arch);
        for ep in &episodes {
            let src = anycode_memory::episodes_dir(&base).join(format!("{}.json", ep.id));
            let dst = arch.join(format!("{}.json", ep.id));
            let _ = std::fs::rename(src, dst);
        }
    }
    Ok(serde_json::json!({
        "applied": apply,
        "run_id": report.run_id,
        "promoted": report.promoted.len(),
        "skipped_secrets": report.skipped_secrets,
        "duplicates_merged": report.duplicates_merged,
        "conflicts": report.conflicts,
        "forgotten_ids": report.forgotten_ids,
        "why": report.promoted.iter().map(|m| serde_json::json!({
            "id": m.id,
            "title": m.title,
            "evidence_hash": m.meta.as_ref().map(|x| x.evidence_hash.clone()).unwrap_or_default(),
            "source": m.meta.as_ref().map(|x| x.source.clone()).unwrap_or_default(),
        })).collect::<Vec<_>>(),
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
