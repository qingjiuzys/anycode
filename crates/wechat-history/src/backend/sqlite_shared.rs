use crate::backend::attachment_resolver::build_attachment_from_payload;
use crate::date_filter::direction_from_is_sender;
use crate::model::{MessageDirection, WechatChatMessage};
use crate::wc4_appmsg::extract_payload;
use chrono::{NaiveDateTime, TimeZone, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
#[allow(unused_imports)]
use sqlx::ConnectOptions as _;
use sqlx::{Row, SqlitePool};
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub async fn open_readonly(path: &Path) -> crate::Result<SqlitePool> {
    let opts = SqliteConnectOptions::from_str(&format!("sqlite:{}", path.display()))
        .map_err(|e| crate::WechatHistoryError::Sqlite(format!("parse sqlite url: {e}")))?
        .read_only(true);
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .map_err(|e| crate::WechatHistoryError::Sqlite(format!("open {}: {e}", path.display())))
}

pub fn discover_sqlite_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if root.is_file() && root.extension().is_some_and(|e| e == "db") {
        out.push(root.to_path_buf());
        return out;
    }
    if !root.is_dir() {
        return out;
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(read) = std::fs::read_dir(&dir) else {
            continue;
        };
        for ent in read.flatten() {
            let p = ent.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|e| e == "db") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

pub async fn query_message_tables(
    pool: &SqlitePool,
    talker: Option<&str>,
) -> crate::Result<Vec<WechatChatMessage>> {
    let rows = if let Some(t) = talker {
        let pattern = format!("%{t}%");
        sqlx::query(
            "SELECT MsgSvrID, Type, CreateTime, StrTalker, StrContent, IsSender, Des FROM Message \
             WHERE StrTalker LIKE ?1 OR StrContent LIKE ?1",
        )
        .bind(pattern)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query(
            "SELECT MsgSvrID, Type, CreateTime, StrTalker, StrContent, IsSender, Des FROM Message",
        )
        .fetch_all(pool)
        .await
    }
    .map_err(|e| crate::WechatHistoryError::Sqlite(format!("query Message failed: {e}")))?;

    rows.into_iter()
        .map(map_message_row)
        .collect::<crate::Result<Vec<_>>>()
}

fn map_message_row(row: sqlx::sqlite::SqliteRow) -> crate::Result<WechatChatMessage> {
    let msg_id: Option<String> = row.try_get(0).ok();
    let msg_type_num: i64 = row.try_get(1).unwrap_or(1);
    let create_time: i64 = row.try_get(2).unwrap_or(0);
    let talker: String = row.try_get(3).unwrap_or_default();
    let content: String = row.try_get(4).unwrap_or_default();
    let is_sender: i64 = row.try_get(5).unwrap_or(0);
    let des: Option<String> = row.try_get(6).ok();

    let timestamp_ms = normalize_timestamp_ms(create_time);
    let direction = direction_from_is_sender(is_sender != 0);
    let (content, sender) = if let Some(d) = des.filter(|s| !s.trim().is_empty()) {
        (content, Some(d))
    } else {
        (content, None)
    };

    Ok(WechatChatMessage {
        id: msg_id,
        timestamp_ms,
        conversation_id: talker.clone(),
        conversation_name: Some(talker),
        sender,
        sender_id: None,
        direction,
        msg_type: map_msg_type(msg_type_num),
        content,
        media_path: None,
        attachments: Vec::new(),
    })
}

pub fn normalize_timestamp_ms(raw: i64) -> i64 {
    if raw > 1_000_000_000_000 {
        raw
    } else {
        raw.saturating_mul(1000)
    }
}

pub fn map_msg_type(raw: i64) -> String {
    match raw {
        1 => "text".into(),
        3 => "image".into(),
        34 => "voice".into(),
        43 => "video".into(),
        47 => "emoji".into(),
        49 => "app".into(),
        10000 => "system".into(),
        other => format!("type_{other}"),
    }
}

pub fn parse_chatlog_timestamp(raw: &str) -> i64 {
    let trimmed = raw.trim();
    if let Ok(v) = trimmed.parse::<i64>() {
        return normalize_timestamp_ms(v);
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S") {
        return Utc.from_utc_datetime(&dt).timestamp_millis();
    }
    0
}

pub fn chatlog_message_from_value(v: &serde_json::Value) -> Option<WechatChatMessage> {
    let obj = v.as_object()?;
    let conversation_id = obj
        .get("talker")
        .or_else(|| obj.get("Talker"))
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    if conversation_id.is_empty() {
        return None;
    }
    let content = obj
        .get("content")
        .or_else(|| obj.get("Content"))
        .or_else(|| obj.get("StrContent"))
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let time_raw = obj
        .get("time")
        .or_else(|| obj.get("Time"))
        .or_else(|| obj.get("CreateTime"))
        .map(|x| {
            if let Some(n) = x.as_i64() {
                n.to_string()
            } else {
                x.as_str().unwrap_or("").to_string()
            }
        })
        .unwrap_or_default();
    let timestamp_ms = parse_chatlog_timestamp(&time_raw);
    let direction = obj
        .get("isSend")
        .or_else(|| obj.get("isSender"))
        .or_else(|| obj.get("IsSender"))
        .and_then(|x| x.as_bool().or_else(|| x.as_i64().map(|n| n != 0)))
        .map(direction_from_is_sender)
        .unwrap_or(MessageDirection::Unknown);
    let sender = obj
        .get("sender")
        .or_else(|| obj.get("Sender"))
        .or_else(|| obj.get("Des"))
        .and_then(|x| x.as_str())
        .map(String::from);
    let msg_type = obj
        .get("type")
        .or_else(|| obj.get("Type"))
        .and_then(|x| x.as_i64())
        .map(map_msg_type)
        .unwrap_or_else(|| "text".to_string());
    let id = obj.get("id").or_else(|| obj.get("MsgSvrID")).and_then(|x| {
        x.as_str()
            .map(String::from)
            .or_else(|| x.as_i64().map(|n| n.to_string()))
    });

    Some(WechatChatMessage {
        id,
        timestamp_ms,
        conversation_id: conversation_id.clone(),
        conversation_name: obj
            .get("talkerName")
            .or_else(|| obj.get("TalkerName"))
            .and_then(|x| x.as_str())
            .map(String::from)
            .or(Some(conversation_id)),
        sender: sender.clone(),
        sender_id: sender,
        direction,
        msg_type,
        content,
        media_path: obj
            .get("path")
            .or_else(|| obj.get("filePath"))
            .and_then(|x| x.as_str())
            .map(String::from),
        attachments: Vec::new(),
    })
}

/// Map a JSON object row from `sqlcipher -json` output into a message.
pub fn message_from_sqlcipher_json(v: &serde_json::Value) -> Option<WechatChatMessage> {
    let obj = v.as_object()?;
    let talker = obj
        .get("StrTalker")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    if talker.is_empty() {
        return None;
    }
    let msg_type_num = obj.get("Type").and_then(|x| x.as_i64()).unwrap_or(1);
    let create_time = obj
        .get("CreateTime")
        .and_then(|x| {
            x.as_i64()
                .or_else(|| x.as_str().and_then(|s| s.parse().ok()))
        })
        .unwrap_or(0);
    let content = obj
        .get("StrContent")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let is_sender = obj
        .get("IsSender")
        .and_then(|x| x.as_i64().map(|b| b as i64))
        .unwrap_or(0);
    let des = obj.get("Des").and_then(|x| x.as_str()).map(String::from);
    let id = obj.get("MsgSvrID").and_then(|x| {
        x.as_str()
            .map(String::from)
            .or_else(|| x.as_i64().map(|n| n.to_string()))
    });
    let timestamp_ms = normalize_timestamp_ms(create_time);
    let direction = direction_from_is_sender(is_sender != 0);
    let (content, sender) = if let Some(d) = des.filter(|s| !s.trim().is_empty()) {
        (content, Some(d))
    } else {
        (content, None)
    };
    Some(WechatChatMessage {
        id,
        timestamp_ms,
        conversation_id: talker.clone(),
        conversation_name: Some(talker),
        sender,
        sender_id: None,
        direction,
        msg_type: map_msg_type(msg_type_num),
        content,
        media_path: None,
        attachments: Vec::new(),
    })
}

pub fn username_to_msg_table(username: &str) -> String {
    format!("Msg_{:x}", md5::compute(username.as_bytes()))
}

/// WeChat 4.x `Msg_<md5>` table row from sqlcipher JSON output.
pub fn message_from_wc4_sqlcipher_json(
    v: &serde_json::Value,
    username: &str,
) -> Option<WechatChatMessage> {
    let obj = v.as_object()?;
    let local_type = obj.get("local_type").and_then(|x| x.as_i64()).unwrap_or(1);
    let create_time = obj
        .get("create_time")
        .and_then(|x| {
            x.as_i64()
                .or_else(|| x.as_str().and_then(|s| s.parse().ok()))
        })
        .unwrap_or(0);
    let message_content = obj
        .get("message_content")
        .and_then(|x| x.as_str())
        .unwrap_or("");
    let compress_content = obj.get("compress_content").and_then(|x| x.as_str());
    let source = obj.get("source").and_then(|x| x.as_str()).unwrap_or("");
    let real_sender_id = obj.get("real_sender_id").and_then(|x| {
        x.as_i64()
            .map(|n| n.to_string())
            .or_else(|| x.as_str().map(String::from))
    });
    let id = obj.get("local_id").and_then(|x| {
        x.as_str()
            .map(String::from)
            .or_else(|| x.as_i64().map(|n| n.to_string()))
    });
    let payload = extract_payload(message_content, compress_content);
    let is_group = username.contains("@chatroom");
    let mut content = payload.clone();
    let mut sender_id = real_sender_id.clone();
    let mut sender = None;
    if is_group && content.contains(":\n") {
        let (raw_sender, body) = content.split_once(":\n").unwrap_or((&content, ""));
        if sender_id.is_none() {
            sender_id = Some(raw_sender.to_string());
        }
        sender = Some(raw_sender.to_string());
        content = body.to_string();
    } else if !source.is_empty() {
        sender = Some(source.to_string());
    }
    if local_type != 1 && content.starts_with('<') {
        if let Some(title) = crate::wc4_appmsg::parse_appmsg_xml(&content).and_then(|m| m.title) {
            content = title;
        }
    }
    let mut attachments = Vec::new();
    if let Some(att) =
        build_attachment_from_payload(&payload, local_type, username, sender_id.clone())
    {
        attachments.push(att);
    }
    let timestamp_ms = normalize_timestamp_ms(create_time);
    Some(WechatChatMessage {
        id,
        timestamp_ms,
        conversation_id: username.to_string(),
        conversation_name: None,
        sender,
        sender_id,
        direction: MessageDirection::Unknown,
        msg_type: map_msg_type(local_type),
        content,
        media_path: None,
        attachments,
    })
}
