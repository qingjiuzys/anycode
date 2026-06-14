//! Direct local SQLCipher reads via the `sqlcipher` CLI (no HTTP port, no rusqlite link conflict).

use super::sqlite_shared::{
    message_from_sqlcipher_json, message_from_wc4_sqlcipher_json, username_to_msg_table,
};
use crate::model::WechatChatMessage;
use crate::{Result, WechatHistoryError};
use std::path::{Path, PathBuf};
use std::process::Command;

const SQLCIPHER_CANDIDATES: &[&str] = &[
    "sqlcipher",
    "/opt/homebrew/bin/sqlcipher",
    "/usr/local/bin/sqlcipher",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DbSchema {
    LegacyMessage,
    WeChat4MsgTables,
}

pub fn find_sqlcipher_bin() -> Option<PathBuf> {
    for name in SQLCIPHER_CANDIDATES {
        let path = if name.contains('/') {
            PathBuf::from(name)
        } else {
            find_in_path(name)?
        };
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

fn find_in_path(name: &str) -> Option<PathBuf> {
    let out = Command::new("sh")
        .args(["-lc", &format!("command -v {name}")])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if p.is_empty() {
        None
    } else {
        Some(PathBuf::from(p))
    }
}

fn key_pragma(key: &str) -> String {
    let trimmed = key.trim();
    if trimmed.starts_with("x'") || trimmed.starts_with("X'") {
        format!("PRAGMA key = \"{trimmed}\";")
    } else {
        format!("PRAGMA key = \"x'{trimmed}'\";")
    }
}

fn cipher_pragmas(key: &str) -> String {
    cipher_pragmas_for_key(key)
}

pub fn cipher_pragmas_for_key(key: &str) -> String {
    format!(
        "{}\nPRAGMA cipher_page_size = 4096;\nPRAGMA kdf_iter = 256000;",
        key_pragma(key)
    )
}

pub fn run_sqlcipher_json(db_path: &Path, sql: &str) -> Result<Vec<serde_json::Value>> {
    let bin = find_sqlcipher_bin().ok_or_else(|| {
        WechatHistoryError::SqlCipher(
            "sqlcipher CLI not found; install with `brew install sqlcipher` for direct local reads"
                .into(),
        )
    })?;
    let mut child = Command::new(&bin)
        .arg(db_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| WechatHistoryError::SqlCipher(format!("spawn {}: {e}", bin.display())))?;
    use std::io::Write;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(sql.as_bytes())
            .map_err(|e| WechatHistoryError::SqlCipher(format!("write sqlcipher stdin: {e}")))?;
    }
    let finished = child
        .wait_with_output()
        .map_err(|e| WechatHistoryError::SqlCipher(format!("wait sqlcipher: {e}")))?;
    if !finished.status.success() {
        let stderr = String::from_utf8_lossy(&finished.stderr);
        return Err(WechatHistoryError::SqlCipher(format!(
            "sqlcipher query {} failed: {stderr}",
            db_path.display()
        )));
    }
    parse_json_rows_raw(&finished.stdout)
}

fn parse_json_rows_raw(stdout: &[u8]) -> Result<Vec<serde_json::Value>> {
    let text = String::from_utf8_lossy(stdout);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    // sqlcipher CLI may prefix decrypted status with a line `ok`.
    let json_start = match trimmed.find(['[', '{']) {
        Some(i) => i,
        None if trimmed == "ok"
            || trimmed
                .lines()
                .all(|l| l.trim() == "ok" || l.trim().is_empty()) =>
        {
            return Ok(Vec::new());
        }
        None => {
            return Err(WechatHistoryError::SqlCipher(
                "parse sqlcipher json output: no json".into(),
            ));
        }
    };
    let json_text = &trimmed[json_start..];
    let value: serde_json::Value = serde_json::from_str(json_text)
        .map_err(|e| WechatHistoryError::SqlCipher(format!("parse sqlcipher json output: {e}")))?;
    if let Some(arr) = value.as_array() {
        Ok(arr.clone())
    } else {
        Ok(vec![value])
    }
}

fn detect_schema(db_path: &Path, key: &str) -> Result<DbSchema> {
    let sql = format!(
        "{}\n.mode json\nSELECT name FROM sqlite_master WHERE type='table' AND (name='Message' OR name LIKE 'Msg_%');",
        cipher_pragmas(key)
    );
    let rows = run_sqlcipher_json(db_path, &sql)?;
    let mut has_message = false;
    let mut has_msg_md5 = false;
    for row in &rows {
        let Some(name) = row.get("name").and_then(|v| v.as_str()) else {
            continue;
        };
        if name == "Message" {
            has_message = true;
        } else if name.starts_with("Msg_") {
            has_msg_md5 = true;
        }
    }
    if has_message {
        Ok(DbSchema::LegacyMessage)
    } else if has_msg_md5 {
        Ok(DbSchema::WeChat4MsgTables)
    } else {
        Ok(DbSchema::WeChat4MsgTables)
    }
}

fn build_legacy_query_sql(key: &str, talker: Option<&str>) -> String {
    let select = if let Some(t) = talker {
        let escaped = t.replace('\'', "''");
        format!(
            "SELECT MsgSvrID, Type, CreateTime, StrTalker, StrContent, IsSender, Des FROM Message \
             WHERE StrTalker LIKE '%{escaped}%' OR StrContent LIKE '%{escaped}%';"
        )
    } else {
        "SELECT MsgSvrID, Type, CreateTime, StrTalker, StrContent, IsSender, Des FROM Message;"
            .to_string()
    };
    format!("{}\n.mode json\n{select}", cipher_pragmas(key))
}

fn load_name2id(db_path: &Path, key: &str) -> Result<Vec<String>> {
    let sql = format!(
        "{}\n.mode json\nSELECT user_name FROM Name2Id WHERE user_name != '';",
        cipher_pragmas(key)
    );
    let rows = run_sqlcipher_json(db_path, &sql)?;
    Ok(rows
        .iter()
        .filter_map(|row| {
            row.get("user_name")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
        .collect())
}

fn username_matches_filter(username: &str, talker: &str) -> bool {
    let u = username.to_ascii_lowercase();
    let t = talker.to_ascii_lowercase();
    u.contains(&t) || t.contains(&u)
}

fn query_wc4_tables(
    db_path: &Path,
    key: &str,
    talker: Option<&str>,
    start_sec: i64,
    end_sec: i64,
) -> Result<Vec<WechatChatMessage>> {
    let mut usernames = load_name2id(db_path, key)?;
    if let Some(t) = talker.filter(|s| !s.trim().is_empty()) {
        usernames.retain(|u| username_matches_filter(u, t));
    }
    let mut out = Vec::new();
    for username in usernames {
        let table = username_to_msg_table(&username);
        let escaped_table = table.replace(']', "]]");
        let sql = format!(
            "{}\n.mode json\nSELECT local_id, local_type, create_time, message_content, compress_content, source, real_sender_id \
             FROM [{escaped_table}] \
             WHERE create_time >= {start_sec} AND create_time < {end_sec};",
            cipher_pragmas(key)
        );
        let rows = match run_sqlcipher_json(db_path, &sql) {
            Ok(rows) => rows,
            Err(_) => continue,
        };
        for row in rows {
            if let Some(msg) = message_from_wc4_sqlcipher_json(&row, &username) {
                out.push(msg);
            }
        }
    }
    Ok(out)
}

fn query_legacy(db_path: &Path, key: &str, talker: Option<&str>) -> Result<Vec<WechatChatMessage>> {
    let sql = build_legacy_query_sql(key, talker);
    let rows = run_sqlcipher_json(db_path, &sql)?;
    Ok(rows
        .iter()
        .filter_map(message_from_sqlcipher_json)
        .collect())
}

pub fn query_encrypted_db(
    db_path: &Path,
    key: &str,
    talker: Option<&str>,
    start_ms: i64,
    end_ms: i64,
) -> Result<Vec<WechatChatMessage>> {
    let start_sec = start_ms / 1000;
    let end_sec = (end_ms + 999) / 1000;
    match detect_schema(db_path, key)? {
        DbSchema::LegacyMessage => query_legacy(db_path, key, talker),
        DbSchema::WeChat4MsgTables => {
            let rows = run_sqlcipher_json(
                db_path,
                &format!(
                    "{}\n.mode json\nSELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'Msg_%' LIMIT 1;",
                    cipher_pragmas(key)
                ),
            )?;
            if rows.is_empty() {
                return Ok(Vec::new());
            }
            query_wc4_tables(db_path, key, talker, start_sec, end_sec)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_pragma_wraps_raw_hex() {
        let sql = build_legacy_query_sql("aabb", None);
        assert!(sql.contains("PRAGMA key = \"x'aabb'\""));
        assert!(sql.contains("cipher_page_size = 4096"));
    }

    #[test]
    fn parse_json_rows_empty() {
        assert!(parse_json_rows_raw(b"").unwrap().is_empty());
    }

    #[test]
    fn username_to_msg_table_matches_python_md5() {
        let table = username_to_msg_table("wxid_test");
        assert_eq!(table, format!("Msg_{:x}", md5::compute(b"wxid_test")));
    }

    #[test]
    fn discover_helper_available() {
        let dir = tempfile::tempdir().unwrap();
        assert!(super::super::sqlite_shared::discover_sqlite_files(dir.path()).is_empty());
    }
}
