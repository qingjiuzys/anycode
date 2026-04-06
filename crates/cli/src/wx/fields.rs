//! iLink JSON 字段兼容：服务端可能返回 snake_case 或 camelCase。

use serde_json::Value;

pub fn str_snake_camel<'a>(
    v: &'a Value,
    snake: &'static str,
    camel: &'static str,
) -> Option<&'a str> {
    v.get(snake)
        .and_then(|x| x.as_str())
        .or_else(|| v.get(camel).and_then(|x| x.as_str()))
}

pub fn i64_snake_camel(v: &Value, snake: &'static str, camel: &'static str) -> Option<i64> {
    if let Some(n) = v.get(snake).and_then(|x| x.as_i64()) {
        return Some(n);
    }
    if let Some(n) = v.get(camel).and_then(|x| x.as_i64()) {
        return Some(n);
    }
    let s = str_snake_camel(v, snake, camel)?;
    s.parse().ok()
}

pub fn sync_buf_from_response(resp: &Value) -> Option<&str> {
    resp.get("get_updates_buf")
        .and_then(|x| x.as_str())
        .or_else(|| resp.get("getUpdatesBuf").and_then(|x| x.as_str()))
}

pub fn msgs_array(resp: &Value) -> Vec<Value> {
    resp.get("msgs")
        .and_then(|x| x.as_array())
        .or_else(|| resp.get("Msgs").and_then(|x| x.as_array()))
        .cloned()
        .unwrap_or_default()
}

pub fn item_type(it: &Value) -> i64 {
    it.get("type")
        .and_then(|x| x.as_i64())
        .or_else(|| {
            it.get("type")
                .and_then(|x| x.as_str())
                .and_then(|s| s.parse().ok())
        })
        .unwrap_or(0)
}
