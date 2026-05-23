use crate::app_config::Config;
use anycode_core::MemoryType;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct RetentionRow {
    id: String,
    mem_type: String,
    title: String,
    updated_at: String,
    action: String,
    reason: String,
}

pub(crate) async fn run_prune(
    config: &Config,
    dry_run: bool,
    apply: bool,
    older_than_days: i64,
    json: bool,
) -> anyhow::Result<()> {
    if dry_run == apply {
        anyhow::bail!("choose exactly one of --dry-run or --apply");
    }
    let (store, _) = crate::bootstrap::build_memory_layer(
        config,
        crate::bootstrap::MemoryAttachMode::Exclusive,
    )?;
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
                mem_type: format!("{:?}", mem_type),
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
    if json {
        println!("{}", serde_json::to_string_pretty(&rows)?);
    } else {
        for row in rows {
            println!(
                "{} {} ({}) — {}: {}",
                row.action, row.id, row.mem_type, row.reason, row.title
            );
        }
    }
    Ok(())
}
