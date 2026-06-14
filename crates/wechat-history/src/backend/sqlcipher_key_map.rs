use super::attachment_resolver::AttachmentContext;
use super::sqlite_cipher::query_encrypted_db;
use super::sqlite_shared::discover_sqlite_files;
use super::sqlite_snapshot::SnapshotDb;
use crate::backend::WechatHistoryBackend;
use crate::config::WechatHistoryConfig;
use crate::date_filter::{filter_messages, validate_query};
use crate::file_parser::DEFAULT_MAX_PARSE_BYTES;
use crate::format::render_markdown_table;
use crate::model::{WechatHistoryQuery, WechatHistoryResult};
use crate::{Result, WechatHistoryError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct SqlcipherKeyMapBackend {
    data_dir: PathBuf,
    key_map_path: PathBuf,
}

impl SqlcipherKeyMapBackend {
    pub fn new(data_dir: PathBuf, key_map_path: PathBuf) -> Self {
        Self {
            data_dir,
            key_map_path,
        }
    }

    fn load_key_map(&self) -> Result<HashMap<String, String>> {
        let raw = std::fs::read_to_string(&self.key_map_path).map_err(|e| {
            WechatHistoryError::SqlCipher(format!(
                "read key map {}: {e}; run: anycode wechat history setup",
                self.key_map_path.display()
            ))
        })?;
        let parsed: HashMap<String, String> = serde_json::from_str(&raw).map_err(|e| {
            WechatHistoryError::SqlCipher(format!(
                "parse key map {}: {e}",
                self.key_map_path.display()
            ))
        })?;
        Ok(parsed)
    }

    fn resolve_key<'a>(key_map: &'a HashMap<String, String>, db_path: &Path) -> Option<&'a str> {
        let file_name = db_path.file_name()?.to_str()?;
        if let Some(k) = key_map.get(file_name) {
            return Some(k.as_str());
        }
        let display = db_path.display().to_string();
        if let Some(k) = key_map.get(&display) {
            return Some(k.as_str());
        }
        let suffix = format!("/{file_name}");
        let mut best: Option<&'a str> = None;
        let mut best_len = 0usize;
        for (k, v) in key_map {
            if k == file_name || k.ends_with(&suffix) {
                if k.len() > best_len {
                    best = Some(v.as_str());
                    best_len = k.len();
                }
            } else if display.ends_with(k.trim_start_matches('/')) && k.len() > best_len {
                best = Some(v.as_str());
                best_len = k.len();
            }
        }
        best
    }
}

impl WechatHistoryBackend for SqlcipherKeyMapBackend {
    fn name(&self) -> &'static str {
        "sqlcipher_key_map"
    }

    fn query(
        &self,
        query: &WechatHistoryQuery,
        config: &WechatHistoryConfig,
    ) -> Result<WechatHistoryResult> {
        let key_map = self.load_key_map()?;
        if key_map.is_empty() {
            return Err(WechatHistoryError::SqlCipher("key map is empty".into()));
        }
        let (date, tz, start_ms, end_ms) = validate_query(query)?;
        let limit = config.effective_limit(query.limit);
        let talker = query.conversation.as_deref();
        let mut messages = Vec::new();
        for db in discover_sqlite_files(&self.data_dir) {
            let Some(key) = Self::resolve_key(&key_map, &db) else {
                continue;
            };
            // Only primary chat DB shards (message_0.db, message_1.db, …).
            let is_message_shard = db
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|name| {
                    name.strip_prefix("message_")
                        .and_then(|rest| rest.strip_suffix(".db"))
                        .filter(|mid| !mid.is_empty() && mid.chars().all(|c| c.is_ascii_digit()))
                })
                .is_some();
            if !is_message_shard {
                continue;
            }
            let snapshot = SnapshotDb::create(&db)?;
            messages.extend(query_encrypted_db(
                snapshot.copy_path(),
                key,
                talker,
                start_ms,
                end_ms,
            )?);
        }
        let (mut messages, truncated) = filter_messages(messages, query, start_ms, end_ms, limit);

        let max_bytes = query
            .max_file_parse_bytes
            .unwrap_or(DEFAULT_MAX_PARSE_BYTES);
        let ctx = AttachmentContext::new(self.data_dir.clone(), &key_map, max_bytes);
        for msg in &mut messages {
            msg.conversation_name = ctx
                .names
                .conversation_name(&msg.conversation_id)
                .or_else(|| msg.conversation_name.clone());
            if let Some(ref sid) = msg.sender_id {
                msg.sender = ctx.names.sender_name(sid).or_else(|| msg.sender.clone());
            }
        }
        let attachment_stats =
            super::attachment_resolver::enrich_messages(&ctx, query, &mut messages);

        let markdown_table = render_markdown_table(&messages, query, tz);
        Ok(WechatHistoryResult {
            date: date.format("%Y-%m-%d").to_string(),
            timezone: tz.to_string(),
            backend: self.name().into(),
            total: messages.len(),
            truncated,
            markdown_table: Some(markdown_table),
            messages,
            attachment_stats: Some(attachment_stats),
        })
    }
}

#[cfg(test)]
mod live_tests {
    use super::*;
    use crate::WechatHistoryBackendKind;
    use std::path::PathBuf;

    #[test]
    #[ignore = "requires local wechat_keys.json and sqlcipher"]
    fn wc4_day_query_smoke() {
        let home = std::env::var("HOME").expect("HOME");
        let cfg = WechatHistoryConfig {
            enabled: true,
            backend: WechatHistoryBackendKind::SqlcipherKeyMap,
            data_dir: Some(PathBuf::from(format!(
                "{home}/Library/Containers/com.tencent.xinWeChat/Data/Documents/xwechat_files/wxid_9vfnpn3c64b922_5038/db_storage"
            ))),
            key_map_path: Some(PathBuf::from(format!(
                "{home}/.anycode/wechat-history/wechat_keys.json"
            ))),
            ..Default::default()
        };
        let backend = SqlcipherKeyMapBackend::new(
            cfg.data_dir.clone().unwrap(),
            cfg.key_map_path.clone().unwrap(),
        );
        let query = WechatHistoryQuery {
            date: "2026-04-13".into(),
            conversation: None,
            keyword: None,
            timezone: Some("Asia/Shanghai".into()),
            limit: Some(10),
            include_group_sender: false,
            ..Default::default()
        };
        let result = backend.query(&query, &cfg).expect("query");
        assert!(result.total > 0, "expected messages on 2026-04-13");
        eprintln!("sample: {:?}", result.messages.first());
    }
}
