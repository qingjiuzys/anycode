//! Compaction checkpoint metadata for long-horizon session recovery.

use anycode_core::prelude::Message;
use chrono::Utc;
use serde::Serialize;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
struct CompactionCheckpointRow {
    ts: String,
    message_count_before: usize,
    message_count_after: usize,
    included_message_ids: Vec<String>,
    trigger: &'static str,
}

fn checkpoint_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".anycode/sessions/checkpoints.jsonl"))
}

/// Best-effort append of compaction checkpoint metadata (does not block compaction on failure).
pub fn append_compaction_checkpoint(
    session_before: &[Message],
    compacted: &[Message],
    trigger: &'static str,
) {
    let Some(path) = checkpoint_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let row = CompactionCheckpointRow {
        ts: Utc::now().to_rfc3339(),
        message_count_before: session_before.len(),
        message_count_after: compacted.len(),
        included_message_ids: session_before.iter().map(|m| m.id.to_string()).collect(),
        trigger,
    };
    let Ok(line) = serde_json::to_string(&row) else {
        return;
    };
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{line}");
    }
}
