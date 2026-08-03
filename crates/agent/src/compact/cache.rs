//! 压缩缓存（与 Claude Code 2.1.218 `services/compact` 的 `.precompact.json` 对齐）。
//!
//! 二进制提取语义：
//! - 缓存文件名为 `.precompact.json`；写入使用 `.precompact.json.tmp.<pid>` 临时文件后原子改名。
//! - 缓存条目包含 `version` 与 `sessionId`；读取时校验版本与会话归属。
//! - `CLAUDE_CODE_DISABLE_PRECOMPACT_SKIP`：禁用 skip 优化（不读缓存）。
//! - `SKIP_PRECOMPACT_THRESHOLD`：低于阈值的尾部消息不写缓存（避免小会话白写）。
//! - 缓存大小上限 8MB：超限不写、读取时若超限视为损坏回退。

use anycode_core::prelude::Message;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// 与 Claude `.precompact.json` 对齐的缓存格式版本。
pub const PRECOMPACT_CACHE_VERSION: u32 = 1;
/// 缓存文件大小上限（8MB）。
pub const PRECOMPACT_CACHE_MAX_BYTES: u64 = 8 * 1024 * 1024;
/// `SKIP_PRECOMPACT_THRESHOLD`：尾部消息少于该条数不写缓存。
pub const SKIP_PRECOMPACT_THRESHOLD: usize = 2;

// 测试可注入的缓存目录（线程局部，避免并行测试互相覆盖 override 造成竞态）。
thread_local! {
    static CACHE_DIR_OVERRIDE: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

#[cfg(test)]
fn set_cache_dir_override(dir: Option<PathBuf>) {
    CACHE_DIR_OVERRIDE.with(|cell| *cell.borrow_mut() = dir);
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct PrecompactCacheEntry {
    version: u32,
    session_id: String,
    /// 摘要输入的尾部消息 id 列表（指纹，用于失效判断）。
    message_ids: Vec<String>,
    /// 模型原始摘要（尚未 formatCompactSummary）。
    summary: String,
}

/// 会话缓存键：`transcript_path` 存在时用其文件名（不含扩展），否则用工作目录的稳定哈希。
pub fn cache_session_id(transcript_path: Option<&str>, working_directory: &str) -> String {
    if let Some(p) = transcript_path {
        if !p.trim().is_empty() {
            if let Some(name) = Path::new(p)
                .file_stem()
                .and_then(|s| s.to_str())
                .filter(|s| !s.is_empty())
            {
                return sanitize(name);
            }
        }
    }
    sanitize(&simple_hash(working_directory))
}

fn sanitize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    out
}

fn simple_hash(s: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    format!("{:016x}", h.finish())
}

fn cache_dir() -> Option<PathBuf> {
    let override_dir = CACHE_DIR_OVERRIDE.with(|cell| cell.borrow().clone());
    if let Some(dir) = override_dir {
        return Some(dir);
    }
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".anycode/sessions/precompact"))
}

fn cache_path(session_id: &str) -> Option<PathBuf> {
    Some(cache_dir()?.join(format!("{session_id}.precompact.json")))
}

/// 摘要输入指纹：与 `build_compact_api_messages` 一致，取 `summarization_start_index` 起的消息 id。
pub fn fingerprint_message_ids(session: &[Message]) -> Vec<String> {
    let start = super::summarization_start_index(session);
    session[start.min(session.len())..]
        .iter()
        .map(|m| m.id.to_string())
        .collect()
}

/// 读取缓存：命中返回缓存摘要。版本不符 / sessionId 不符 / 指纹不符 / 超限 / JSON 损坏均回退 `None`（损坏时删除）。
pub fn read_precompact_cache(session_id: &str, message_ids: &[String]) -> Option<String> {
    let path = cache_path(session_id)?;
    let meta = fs::metadata(&path).ok()?;
    if meta.len() > PRECOMPACT_CACHE_MAX_BYTES {
        let _ = fs::remove_file(&path);
        return None;
    }
    let bytes = fs::read(&path).ok()?;
    let Ok(entry) = serde_json::from_slice::<PrecompactCacheEntry>(&bytes) else {
        // 损坏 JSON：删除并回退，避免每次压缩都重试解析。
        let _ = fs::remove_file(&path);
        return None;
    };
    if entry.version != PRECOMPACT_CACHE_VERSION
        || entry.session_id != session_id
        || entry.message_ids != message_ids
        || entry.summary.trim().is_empty()
    {
        return None;
    }
    Some(entry.summary)
}

/// 写缓存（best-effort）：尾部消息过少跳过；序列化超 8MB 跳过；临时文件 + 原子改名。
pub fn write_precompact_cache(session_id: &str, message_ids: &[String], summary: &str) -> bool {
    if message_ids.len() < SKIP_PRECOMPACT_THRESHOLD {
        return false;
    }
    let entry = PrecompactCacheEntry {
        version: PRECOMPACT_CACHE_VERSION,
        session_id: session_id.to_string(),
        message_ids: message_ids.to_vec(),
        summary: summary.to_string(),
    };
    let Ok(bytes) = serde_json::to_vec(&entry) else {
        return false;
    };
    if bytes.len() as u64 > PRECOMPACT_CACHE_MAX_BYTES {
        return false;
    }
    let Some(dir) = cache_dir() else {
        return false;
    };
    let _ = fs::create_dir_all(&dir);
    let Some(path) = cache_path(session_id) else {
        return false;
    };
    let tmp = dir.join(format!(
        "{session_id}.precompact.json.tmp.{}",
        std::process::id()
    ));
    let Ok(mut f) = fs::File::create(&tmp) else {
        return false;
    };
    if f.write_all(&bytes).is_err() || f.sync_all().is_err() {
        let _ = fs::remove_file(&tmp);
        return false;
    }
    drop(f);
    fs::rename(&tmp, &path).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use anycode_core::prelude::*;
    use std::collections::HashMap;
    use uuid::Uuid;

    fn msg(id: Uuid, role: MessageRole, summary: bool) -> Message {
        let mut metadata = HashMap::new();
        if summary {
            metadata.insert(
                ANYCODE_COMPACT_SUMMARY_METADATA_KEY.to_string(),
                serde_json::Value::Bool(true),
            );
        }
        Message {
            id,
            role,
            content: MessageContent::Text("x".into()),
            timestamp: chrono::Utc::now(),
            metadata,
        }
    }

    fn sample_session() -> Vec<Message> {
        vec![
            msg(Uuid::new_v4(), MessageRole::System, false),
            msg(Uuid::new_v4(), MessageRole::User, false),
            msg(Uuid::new_v4(), MessageRole::Assistant, false),
            msg(Uuid::new_v4(), MessageRole::User, false),
        ]
    }

    fn test_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "anycode_precompact_test_{}_{}",
            std::process::id(),
            Uuid::new_v4()
        ));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn write_then_read_roundtrip() {
        let session = sample_session();
        let ids = fingerprint_message_ids(&session);
        let sid = "test-session-1";
        let dir = test_dir();
        set_cache_dir_override(Some(dir.clone()));
        let ok = write_precompact_cache(sid, &ids, "<summary>hi</summary>");
        let cached = read_precompact_cache(sid, &ids);
        set_cache_dir_override(None);
        assert!(ok);
        assert_eq!(cached.as_deref(), Some("<summary>hi</summary>"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn fingerprint_changes_when_tail_grows() {
        let session = sample_session();
        let ids1 = fingerprint_message_ids(&session);
        let mut grown = session.clone();
        grown.push(msg(Uuid::new_v4(), MessageRole::User, false));
        let ids2 = fingerprint_message_ids(&grown);
        assert_ne!(ids1, ids2);
    }

    #[test]
    fn miss_on_version_mismatch_and_corrupt_file_falls_back() {
        let session = sample_session();
        let ids = fingerprint_message_ids(&session);
        let dir = test_dir();
        set_cache_dir_override(Some(dir.clone()));
        write_precompact_cache("s", &ids, "ok");
        let path = cache_path("s").unwrap();
        // 版本不符。
        let entry = serde_json::json!({
            "version": 999,
            "session_id": "s",
            "message_ids": ids,
            "summary": "ok"
        });
        fs::write(&path, serde_json::to_vec(&entry).unwrap()).unwrap();
        assert_eq!(read_precompact_cache("s", &ids), None);
        // 损坏 JSON → 回退 None 并删除。
        fs::write(&path, b"{not-json").unwrap();
        assert_eq!(read_precompact_cache("s", &ids), None);
        assert!(!path.exists());
        set_cache_dir_override(None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn skip_writing_when_tail_too_short() {
        let session = vec![
            msg(Uuid::new_v4(), MessageRole::System, false),
            msg(Uuid::new_v4(), MessageRole::User, false),
        ];
        let ids = fingerprint_message_ids(&session);
        let dir = test_dir();
        set_cache_dir_override(Some(dir.clone()));
        let ok = write_precompact_cache("s", &ids, "summary");
        set_cache_dir_override(None);
        assert!(!ok);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn cache_session_id_prefers_transcript_stem() {
        assert_eq!(
            cache_session_id(Some("/tmp/abc/def-123.jsonl"), "/tmp/other"),
            "def-123"
        );
        // 无 transcript 时用工作目录稳定哈希。
        let a = cache_session_id(None, "/tmp/wd");
        let b = cache_session_id(None, "/tmp/wd");
        assert_eq!(a, b);
        let c = cache_session_id(None, "/tmp/wd-other");
        assert_ne!(a, c);
    }
}
