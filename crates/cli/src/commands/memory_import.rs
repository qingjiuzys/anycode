//! 将 legacy `FileMemoryStore` 目录下的 Markdown 记忆批量导入 pipeline 热层。

use crate::app_config::Config;
use anycode_core::prelude::*;
use anycode_memory::FileMemoryStore;

pub(crate) async fn run_import(
    config: &Config,
    dry_run: bool,
    limit: Option<usize>,
) -> anyhow::Result<()> {
    if config.memory.backend != "pipeline" {
        anyhow::bail!(
            "memory.backend must be \"pipeline\" (current: {})",
            config.memory.backend
        );
    }

    let file =
        FileMemoryStore::new(config.memory.path.clone()).map_err(|e| anyhow::anyhow!("{}", e))?;

    let (dest, _) = crate::bootstrap::build_memory_layer(config)?;

    let mut total = 0usize;
    for mt in [
        MemoryType::Project,
        MemoryType::User,
        MemoryType::Feedback,
        MemoryType::Reference,
    ] {
        let batch = file
            .recall("", mt)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        for m in batch {
            if let Some(max) = limit {
                if total >= max {
                    println!(
                        "done: {} memories {}",
                        total,
                        if dry_run {
                            "(dry-run)"
                        } else {
                            "into pipeline hot store"
                        }
                    );
                    return Ok(());
                }
            }
            if dry_run {
                println!(
                    "[dry-run] would import {} ({:?}) {}",
                    m.id, m.mem_type, m.title
                );
            } else {
                dest.save(m.clone())
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                println!("imported {} ({:?}) {}", m.id, m.mem_type, m.title);
            }
            total += 1;
        }
    }

    println!(
        "done: {} memor{} {}",
        total,
        if total == 1 { "y" } else { "ies" },
        if dry_run {
            "(dry-run)"
        } else {
            "into pipeline hot store"
        }
    );
    Ok(())
}
