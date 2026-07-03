//! Append-only outbound delivery ledger for WeChat sends.

use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct OutboundRecord {
    pub ts: String,
    pub channel: String,
    pub to_user_id: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker: Option<String>,
    pub retry_count: u32,
    pub last_error: String,
    pub chars: usize,
}

pub(crate) fn wechat_outbound_log_path(data_root: &Path) -> PathBuf {
    data_root.join("outbound.jsonl")
}

pub(crate) fn append_outbound_record(path: &Path, record: &OutboundRecord) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let Ok(line) = serde_json::to_string(record) else {
        return;
    };
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{line}");
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct OutboundQueueStats {
    pub pending: u32,
    pub sent: u32,
    pub failed: u32,
}

pub(crate) fn summarize_outbound_log(path: &Path) -> OutboundQueueStats {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return OutboundQueueStats::default();
    };
    let mut stats = OutboundQueueStats::default();
    for line in raw.lines() {
        let Ok(row) = serde_json::from_str::<OutboundRecord>(line) else {
            continue;
        };
        match row.status.as_str() {
            "pending" => stats.pending += 1,
            "sent" => stats.sent += 1,
            "failed" => stats.failed += 1,
            _ => {}
        }
    }
    stats
}
