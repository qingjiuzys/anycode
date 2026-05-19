//! CDN 下载 + AES-128-ECB 解密 + 消息项解析。
//! 入站字段布局与 openclaw-weixin `MessageItem` / `media-download` 对齐（含 `image_item.media`、`image_item.aeskey` hex）。

use crate::wx::fields::item_type;
use crate::wx::ilink::cdn_download_url;
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde_json::Value;
use std::time::Duration;

fn decrypt_aes_128_ecb(key16: &[u8; 16], ciphertext: &[u8]) -> Result<Vec<u8>> {
    use aes::cipher::generic_array::GenericArray;
    use aes::cipher::{BlockDecrypt, KeyInit};
    use aes::Aes128;
    if !ciphertext.len().is_multiple_of(16) {
        anyhow::bail!("密文长度非 16 倍数");
    }
    let cipher = Aes128::new(GenericArray::from_slice(key16));
    let mut out = Vec::with_capacity(ciphertext.len());
    for chunk in ciphertext.chunks_exact(16) {
        let mut block = GenericArray::clone_from_slice(chunk);
        cipher.decrypt_block(&mut block);
        out.extend_from_slice(&block);
    }
    pkcs7_unpad(&out)
}

fn pkcs7_unpad(data: &[u8]) -> Result<Vec<u8>> {
    if data.is_empty() {
        anyhow::bail!("空数据");
    }
    let n = *data.last().unwrap() as usize;
    if n == 0 || n > 16 || n > data.len() {
        anyhow::bail!("无效 PKCS#7");
    }
    if !data[data.len() - n..].iter().all(|&b| b as usize == n) {
        anyhow::bail!("无效 PKCS#7 填充");
    }
    Ok(data[..data.len() - n].to_vec())
}

fn parse_aes_key(aes_key_b64: &str) -> Result<[u8; 16]> {
    let raw = B64
        .decode(aes_key_b64.as_bytes())
        .context("AES key base64")?;
    if raw.len() == 16 {
        let mut k = [0u8; 16];
        k.copy_from_slice(&raw);
        return Ok(k);
    }
    let hex_str = String::from_utf8_lossy(&raw);
    let v = hex::decode(hex_str.trim())?;
    if v.len() != 16 {
        anyhow::bail!("AES key 长度应为 16 字节");
    }
    let mut k = [0u8; 16];
    k.copy_from_slice(&v);
    Ok(k)
}

/// openclaw-weixin: `image_item.aeskey` 常为 32 位 hex，等价于对 16 字节 key 做 base64 再作为 `parse_aes_key` 输入。
fn normalize_media_aes_key(s: &str) -> String {
    let t = s.trim();
    if t.len() == 32 && t.chars().all(|c| c.is_ascii_hexdigit()) {
        if let Ok(raw) = hex::decode(t) {
            if raw.len() == 16 {
                return B64.encode(&raw);
            }
        }
    }
    t.to_string()
}

/// 仅允许微信 CDN 与 ilink 相关主机，防 SSRF（与 OpenClaw 只拉协议 URL 的假设一致）。
pub(crate) fn cdn_get_url_trusted(u: &str) -> bool {
    let Ok(parsed) = url::Url::parse(u) else {
        return false;
    };
    if parsed.scheme() != "https" {
        return false;
    }
    let Some(h) = parsed.host_str() else {
        return false;
    };
    h.ends_with(".weixin.qq.com")
        || h == "weixin.qq.com"
        || h.ends_with(".wechat.com")
        || h == "wechat.com"
        || h == "dev.weixin.qq.com"
}

/// 从 `voice_item` / `voiceItem` 取微信语音转写文本。
fn extract_voice_text_from_item(item: &Value) -> String {
    for ptr in ["/voice_item/text", "/voiceItem/text"] {
        if let Some(s) = item.pointer(ptr).and_then(|x| x.as_str()) {
            if !s.is_empty() {
                return s.to_string();
            }
        }
    }
    String::new()
}

fn image_sub(item: &Value) -> Option<&Value> {
    item.get("image_item").or_else(|| item.get("imageItem"))
}

/// `image_item` 下 `media` 的 query / 密钥 / `full_url`（与 openclaw-weixin 一致）。
fn cdn_fields_from_image_item(img: &Value) -> Option<(String, String, Option<String>)> {
    let media = img.get("media")?;
    let full = media
        .get("full_url")
        .or_else(|| media.get("fullUrl"))
        .and_then(|x| x.as_str())
        .map(String::from);
    let enc = media
        .get("encrypt_query_param")
        .or_else(|| media.get("encryptQueryParam"))
        .and_then(|x| x.as_str())
        .unwrap_or("");
    let key_str: String = if let Some(h) = img
        .get("aeskey")
        .or_else(|| img.get("aesKey"))
        .and_then(|x| x.as_str())
    {
        normalize_media_aes_key(h)
    } else {
        let km = media
            .get("aes_key")
            .or_else(|| media.get("aesKey"))
            .and_then(|x| x.as_str())?;
        normalize_media_aes_key(km)
    };
    if enc.is_empty() && full.is_none() {
        return None;
    }
    if key_str.is_empty() {
        return None;
    }
    Some((enc.to_string(), key_str, full))
}

/// openclaw-weixin `isMediaItem`：IMAGE / VOICE / FILE / VIDEO
fn is_media_item_type(t: i64) -> bool {
    matches!(t, 2..=5)
}

fn is_media_item(v: &Value) -> bool {
    is_media_item_type(item_type(v))
}

/// 与 openclaw-weixin `inbound.ts` 的 `bodyFromItemList` 一致：同序、首条可生成 body 即返回（TEXT 在 VOICE 前被优先消费）。
pub fn body_from_item_list(items: &[Value]) -> String {
    for item in items {
        let t = item_type(item);
        if t == 1 {
            let tnode = item.get("text_item").or_else(|| item.get("textItem"));
            if tnode.is_none() {
                continue;
            }
            if matches!(tnode.and_then(|n| n.get("text")), Some(Value::Null)) {
                continue;
            }
            let text = extract_text_from_item(item);
            let refv = item.get("ref_msg").or_else(|| item.get("refMsg"));
            if refv.is_none() {
                return text;
            }
            let refv = refv.unwrap();
            if let Some(ref_item) = refv.get("message_item").or_else(|| refv.get("messageItem")) {
                if is_media_item(ref_item) {
                    return text;
                }
                let mut parts: Vec<String> = Vec::new();
                if let Some(title) = refv.get("title").and_then(|x| x.as_str()) {
                    if !title.is_empty() {
                        parts.push(title.to_string());
                    }
                }
                let ref_body = body_from_item_list(std::slice::from_ref(ref_item));
                if !ref_body.is_empty() {
                    parts.push(ref_body);
                }
                if parts.is_empty() {
                    return text;
                }
                return format!("[引用: {}]\n{}", parts.join(" | "), text);
            } else if let Some(title) = refv.get("title").and_then(|x| x.as_str()) {
                if !title.is_empty() {
                    return format!("[引用: {}]\n{}", title, text);
                }
                return text;
            } else {
                return text;
            }
        }
        if t == 3 {
            let v = extract_voice_text_from_item(item);
            if !v.is_empty() {
                return v;
            }
        }
    }
    String::new()
}

/// 与 openclaw-weixin `process-message.ts` 的 `extractTextBody` 一致：首条 `TEXT` 的原文（不展开引用），供斜杠命令解析。
pub fn first_plain_text_from_items(items: &[Value]) -> String {
    for it in items {
        if item_type(it) == 1 {
            let tnode = it.get("text_item").or_else(|| it.get("textItem"));
            if tnode.is_none() {
                continue;
            }
            if matches!(tnode.and_then(|n| n.get("text")), Some(Value::Null)) {
                continue;
            }
            return extract_text_from_item(it);
        }
    }
    String::new()
}

/// 标准 `media`（无 `image_item` 顶栏 hex），用于 voice/file/video。
fn cdn_from_standard_media_object(media: &Value) -> Option<(String, String, Option<String>)> {
    let full = media
        .get("full_url")
        .or_else(|| media.get("fullUrl"))
        .and_then(|x| x.as_str())
        .map(String::from);
    let enc = media
        .get("encrypt_query_param")
        .or_else(|| media.get("encryptQueryParam"))
        .and_then(|x| x.as_str())
        .unwrap_or("");
    let k = media
        .get("aes_key")
        .or_else(|| media.get("aesKey"))
        .and_then(|x| x.as_str())?;
    if enc.is_empty() && full.is_none() {
        return None;
    }
    if k.is_empty() {
        return None;
    }
    Some((enc.to_string(), normalize_media_aes_key(k), full))
}

/// `voice_item.media`（仅 `media.aes_key`，与插件一致）
pub fn extract_cdn_keys_for_voice_item(item: &Value) -> Option<(String, String, Option<String>)> {
    let v = item.get("voice_item").or_else(|| item.get("voiceItem"))?;
    cdn_from_standard_media_object(v.get("media")?)
}

fn extract_cdn_keys_for_file_item(item: &Value) -> Option<(String, String, Option<String>)> {
    let f = item.get("file_item").or_else(|| item.get("fileItem"))?;
    cdn_from_standard_media_object(f.get("media")?)
}

fn extract_cdn_keys_for_video_item(item: &Value) -> Option<(String, String, Option<String>)> {
    let v = item.get("video_item").or_else(|| item.get("videoItem"))?;
    cdn_from_standard_media_object(v.get("media")?)
}

/// 可尝试 CDN 拉取：用于主列表 / 引用内 `message_item`（2/3/4/5）。
pub fn extract_cdn_keys_for_any_item(item: &Value) -> Option<(String, String, Option<String>)> {
    let t = item_type(item);
    match t {
        2 => extract_cdn_keys_for_image_item(item),
        3 => extract_cdn_keys_for_voice_item(item),
        4 => extract_cdn_keys_for_file_item(item),
        5 => extract_cdn_keys_for_video_item(item),
        _ => None,
    }
}

fn can_download_cdn_item(item: &Value) -> bool {
    if image_plain_url_only(item).is_some() {
        return true;
    }
    let t = item_type(item);
    if t == 2 {
        return extract_cdn_keys_for_image_item(item).is_some();
    }
    if t == 3 {
        if !extract_voice_text_from_item(item).is_empty() {
            return false;
        }
        return extract_cdn_keys_for_voice_item(item).is_some();
    }
    if t == 4 {
        return extract_cdn_keys_for_file_item(item).is_some();
    }
    if t == 5 {
        return extract_cdn_keys_for_video_item(item).is_some();
    }
    false
}

/// 与 `processOneMessage` 一致：主 `item_list` 中 IMAGE>VIDEO>FILE>VOICE(无转写)；否则从带引用的 `TEXT` 的 `ref_msg.message_item` 回退。
pub fn select_inbound_media_item(items: &[Value]) -> Option<&Value> {
    for it in items {
        if item_type(it) == 2 && can_download_cdn_item(it) {
            return Some(it);
        }
    }
    for it in items {
        if item_type(it) == 5 && can_download_cdn_item(it) {
            return Some(it);
        }
    }
    for it in items {
        if item_type(it) == 4 && can_download_cdn_item(it) {
            return Some(it);
        }
    }
    for it in items {
        if item_type(it) == 3 && can_download_cdn_item(it) {
            return Some(it);
        }
    }
    for it in items {
        if item_type(it) != 1 {
            continue;
        }
        let refv = it.get("ref_msg").or_else(|| it.get("refMsg"))?;
        let ref_m = refv
            .get("message_item")
            .or_else(|| refv.get("messageItem"))?;
        if is_media_item(ref_m) && can_download_cdn_item(ref_m) {
            return Some(ref_m);
        }
    }
    None
}

/// 与 openclaw-weixin 后处理 pipeline 的「用户主文案 + 可下载主媒体项」对拍。
pub fn extract_user_text_and_image_item(items: &[Value]) -> (String, Option<&Value>) {
    (body_from_item_list(items), select_inbound_media_item(items))
}

/// 是否存在「语音项但尚无转写」的入站，用于向用户说明而非静默忽略。
pub fn has_voice_item_without_stt(items: &[Value]) -> bool {
    items
        .iter()
        .any(|it| item_type(it) == 3 && extract_voice_text_from_item(it).is_empty())
}

/// 尝试从入站 `MessageItem` 解析可解密下载所需的密钥与 query / URL（含嵌套 `image_item`、legacy 顶层 `cdn_media`）。
pub fn extract_cdn_keys_for_image_item(item: &Value) -> Option<(String, String, Option<String>)> {
    if let Some(cdn) = item.get("cdn_media") {
        let k = cdn.get("aes_key").and_then(|x| x.as_str())?;
        let e = cdn.get("encrypt_query_param").and_then(|x| x.as_str())?;
        if !k.is_empty() && !e.is_empty() {
            return Some((
                e.to_string(),
                normalize_media_aes_key(k),
                cdn.get("full_url")
                    .or_else(|| cdn.get("fullUrl"))
                    .and_then(|x| x.as_str())
                    .map(str::to_string),
            ));
        }
    }
    if let Some(img) = image_sub(item) {
        if let Some(triple) = cdn_fields_from_image_item(img) {
            return Some(triple);
        }
    }
    if let Some(media) = item.get("media") {
        let e = media
            .get("encrypt_query_param")
            .or_else(|| media.get("encryptQueryParam"))
            .and_then(|x| x.as_str())?;
        let k = media
            .get("aes_key")
            .or_else(|| media.get("aesKey"))
            .and_then(|x| x.as_str())
            .or_else(|| {
                item.get("aeskey")
                    .or_else(|| item.get("aesKey"))
                    .and_then(|x| x.as_str())
            })?;
        return Some((
            e.to_string(),
            normalize_media_aes_key(k),
            media
                .get("full_url")
                .or_else(|| media.get("fullUrl"))
                .and_then(|x| x.as_str())
                .map(str::to_string),
        ));
    }
    None
}

/// 有 `full_url`、无 `aes_key` 时拉取**明文**字节（`downloadPlainCdnBuffer` 语义）。
fn image_plain_url_only(item: &Value) -> Option<String> {
    if let Some(img) = image_sub(item) {
        if let Some(media) = img.get("media") {
            let has_key = media
                .get("aes_key")
                .or_else(|| media.get("aesKey"))
                .is_some()
                || img.get("aeskey").or_else(|| img.get("aesKey")).is_some();
            if has_key {
                return None;
            }
            if let Some(u) = media
                .get("full_url")
                .or_else(|| media.get("fullUrl"))
                .and_then(|x| x.as_str())
            {
                if !u.is_empty() && cdn_get_url_trusted(u) {
                    return Some(u.to_string());
                }
            }
        }
    }
    None
}

pub async fn download_and_decrypt(
    client: &reqwest::Client,
    enc_q: &str,
    aes_key_b64: &str,
    full_url: Option<&str>,
) -> Result<Vec<u8>> {
    let url = if let Some(u) = full_url {
        if !cdn_get_url_trusted(u) {
            anyhow::bail!("不可信的 CDN full_url");
        }
        u.to_string()
    } else if !enc_q.is_empty() {
        cdn_download_url(enc_q)?
    } else {
        anyhow::bail!("缺少 encrypt_query_param 与 full_url");
    };
    let enc = client
        .get(&url)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .context("CDN GET")?
        .bytes()
        .await
        .context("CDN body")?;
    let key = parse_aes_key(aes_key_b64)?;
    decrypt_aes_128_ecb(&key, &enc)
}

async fn download_url_plain_trusted(client: &reqwest::Client, url: &str) -> Result<Vec<u8>> {
    if !cdn_get_url_trusted(url) {
        anyhow::bail!("不可信的 CDN full_url");
    }
    Ok(client
        .get(url)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .context("CDN GET plain")?
        .bytes()
        .await
        .context("plain body")?
        .to_vec())
}

fn detect_mime(head: &[u8]) -> &'static str {
    if head.len() >= 2 && head[0] == 0x89 && head[1] == 0x50 {
        return "image/png";
    }
    if head.len() >= 2 && head[0] == 0xff && head[1] == 0xd8 {
        return "image/jpeg";
    }
    if head.len() >= 2 && head[0] == 0x47 && head[1] == 0x49 {
        return "image/gif";
    }
    if head.len() >= 2 && head[0] == 0x52 && head[1] == 0x49 {
        return "image/webp";
    }
    "image/jpeg"
}

pub async fn download_image_from_item(
    client: &reqwest::Client,
    item: &Value,
) -> Option<(String, String)> {
    if let Some(u) = image_plain_url_only(item) {
        let bytes = download_url_plain_trusted(client, &u).await.ok()?;
        let mime = detect_mime(&bytes);
        return Some((mime.to_string(), B64.encode(&bytes)));
    }
    let (enc_q, key, full) = extract_cdn_keys_for_image_item(item)?;
    if enc_q.is_empty() {
        if let Some(ref u) = full {
            if cdn_get_url_trusted(u) {
                let bytes = download_and_decrypt(client, "", &key, Some(u)).await.ok()?;
                let mime = detect_mime(&bytes);
                return Some((mime.to_string(), B64.encode(&bytes)));
            }
        }
        return None;
    }
    let full_ref = full.as_deref();
    let bytes = download_and_decrypt(client, &enc_q, &key, full_ref)
        .await
        .ok()?;
    let mime = detect_mime(&bytes);
    Some((mime.to_string(), B64.encode(&bytes)))
}

fn guess_mime_file_item(item: &Value) -> &'static str {
    let name = item
        .get("file_item")
        .or_else(|| item.get("fileItem"))
        .and_then(|f| f.get("file_name").or_else(|| f.get("fileName")))
        .and_then(|x| x.as_str())
        .unwrap_or("");
    let lower = name.to_lowercase();
    if lower.ends_with(".zip") {
        "application/zip"
    } else if lower.ends_with(".pdf") {
        "application/pdf"
    } else if lower.ends_with(".txt") {
        "text/plain"
    } else if lower.ends_with(".json") {
        "application/json"
    } else {
        "application/octet-stream"
    }
}

/// 对 IMAGE 走既有逻辑；对 VIDEO/FILE/VOICE 走标准 `media` 解密，返回 (mime, base64 字符串体)。
pub async fn download_cdn_item_to_b64(
    client: &reqwest::Client,
    item: &Value,
) -> Option<(String, String)> {
    let t = item_type(item);
    if t == 2 {
        return download_image_from_item(client, item).await;
    }
    if !matches!(t, 3..=5) {
        return None;
    }
    let (enc, key, full) = extract_cdn_keys_for_any_item(item)?;
    if enc.is_empty() && full.is_none() {
        return None;
    }
    let bytes = download_and_decrypt(client, &enc, &key, full.as_deref())
        .await
        .ok()?;
    let mime = match t {
        3 => "application/octet-stream",
        4 => guess_mime_file_item(item),
        5 => "video/mp4",
        _ => "application/octet-stream",
    };
    Some((mime.to_string(), B64.encode(&bytes)))
}

pub fn extract_text_from_item(item: &Value) -> String {
    for ptr in ["/text_item/text", "/textItem/text"] {
        if let Some(s) = item.pointer(ptr).and_then(|x| x.as_str()) {
            if !s.is_empty() {
                return s.to_string();
            }
        }
    }
    item.get("content")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn image_item_nested_aeskey_hex_yields_key_material() {
        let item = json!({
            "type": 2,
            "image_item": {
                "aeskey": "00112233445566778899aabbccddeeff",
                "media": {
                    "encrypt_query_param": "encq",
                    "aes_key": "should-not-win-over-hex"
                }
            }
        });
        let got = extract_cdn_keys_for_image_item(&item).expect("keys");
        assert_eq!(got.0, "encq");
        assert_eq!(
            got.1,
            normalize_media_aes_key("00112233445566778899aabbccddeeff")
        );
    }

    #[test]
    fn body_from_empty_item_list_is_empty() {
        assert_eq!(body_from_item_list(&[]), "");
        assert_eq!(first_plain_text_from_items(&[]), "");
    }

    #[test]
    fn first_plain_text_ignores_ref_msg_for_slash_commands() {
        let items = vec![json!({
            "type": 1,
            "text_item": { "text": "/help" },
            "ref_msg": { "title": "noise" }
        })];
        assert_eq!(first_plain_text_from_items(&items), "/help");
    }

    #[test]
    fn voice_only_body_stt() {
        let items = vec![json!({
            "type": 3,
            "voice_item": { "text": "你好这是语音" }
        })];
        let (t, img) = extract_user_text_and_image_item(&items);
        assert_eq!(t, "你好这是语音");
        assert!(img.is_none());
    }

    /// openclaw：先遇到 TEXT 则直接返回，不拼后续 VOICE。
    #[test]
    fn text_then_voice_returns_text_only() {
        let items = vec![
            json!({ "type": 1, "text_item": { "text": "前缀" } }),
            json!({ "type": 3, "voice_item": { "text": "转写" } }),
        ];
        assert_eq!(body_from_item_list(&items), "前缀");
        assert_eq!(first_plain_text_from_items(&items), "前缀");
    }

    #[test]
    fn voice_then_text_returns_voice_stt() {
        let items = vec![
            json!({ "type": 3, "voice_item": { "text": "转写" } }),
            json!({ "type": 1, "text_item": { "text": "后文" } }),
        ];
        assert_eq!(body_from_item_list(&items), "转写");
        assert_eq!(first_plain_text_from_items(&items), "后文");
    }

    #[test]
    fn body_text_ref_title_only() {
        let items = vec![json!({
            "type": 1,
            "text_item": { "text": "回复" },
            "ref_msg": { "title": "仅标题" }
        })];
        assert_eq!(body_from_item_list(&items), "[引用: 仅标题]\n回复");
    }

    #[test]
    fn body_text_ref_camel_case_refmsg_title() {
        let items = vec![json!({
            "type": 1,
            "text_item": { "text": "ok" },
            "refMsg": { "title": "camel" }
        })];
        assert_eq!(body_from_item_list(&items), "[引用: camel]\nok");
    }

    #[test]
    fn body_text_ref_empty_title_skips_quote_line() {
        let items = vec![json!({
            "type": 1,
            "text_item": { "text": "回复" },
            "ref_msg": { "title": "" }
        })];
        assert_eq!(body_from_item_list(&items), "回复");
    }

    #[test]
    fn body_text_with_ref_preamble() {
        let items = vec![json!({
            "type": 1,
            "text_item": { "text": "回复" },
            "ref_msg": {
                "title": "标题",
                "message_item": { "type": 1, "text_item": { "text": "被引" } }
            }
        })];
        assert_eq!(body_from_item_list(&items), "[引用: 标题 | 被引]\n回复");
    }

    /// 引用为纯媒体时：不拼 `引用` 行，只返回当前正文（与插件一致）。
    #[test]
    fn body_text_ref_to_image_returns_caption_only() {
        let items = vec![json!({
            "type": 1,
            "text_item": { "text": "看图" },
            "ref_msg": {
                "title": "旧图",
                "message_item": {
                    "type": 2,
                    "image_item": {
                        "aeskey": "00112233445566778899aabbccddeeff",
                        "media": { "encrypt_query_param": "q", "aes_key": "k" }
                    }
                }
            }
        })];
        assert_eq!(body_from_item_list(&items), "看图");
    }

    /// 主列表无图、仅 `TEXT+ref` 内嵌可下 CDN 的图时择引用内 `message_item`。
    #[test]
    fn select_media_falls_back_to_ref_image() {
        let items = vec![json!({
            "type": 1,
            "text_item": { "text": "caption" },
            "ref_msg": {
                "message_item": {
                    "type": 2,
                    "image_item": {
                        "aeskey": "00112233445566778899aabbccddeeff",
                        "media": { "encrypt_query_param": "encq", "aes_key": "k" }
                    }
                }
            }
        })];
        let got = select_inbound_media_item(&items);
        assert!(got.is_some());
        assert_eq!(item_type(got.unwrap()), 2);
    }

    #[test]
    fn main_image_wins_over_ref_image() {
        let main = json!({
            "type": 2,
            "image_item": {
                "aeskey": "00112233445566778899aabbccddeeff",
                "media": { "encrypt_query_param": "a", "aes_key": "k" }
            }
        });
        let text_ref = json!({
            "type": 1,
            "text_item": { "text": "t" },
            "ref_msg": { "message_item": {
                "type": 2,
                "image_item": {
                    "aeskey": "ffeeddccbbaa99887766554433221100",
                    "media": { "encrypt_query_param": "b", "aes_key": "k" }
                }
            }}
        });
        let items = vec![text_ref, main];
        let got = select_inbound_media_item(&items);
        assert_eq!(item_type(got.unwrap()), 2);
        assert_eq!(
            extract_cdn_keys_for_image_item(got.unwrap())
                .map(|(e, ..)| e)
                .as_deref(),
            Some("a")
        );
    }

    /// VIDEO > FILE：主列表同时有视频与文件时选视频。
    #[test]
    fn select_video_before_file() {
        let items = vec![
            json!({
                "type": 4,
                "file_item": { "file_name": "a.pdf", "media": { "encrypt_query_param": "f", "aes_key": "k" } }
            }),
            json!({
                "type": 5,
                "video_item": { "media": { "encrypt_query_param": "v", "aes_key": "k", "full_url": "https://v.wechat.com/x" } }
            }),
        ];
        let got = select_inbound_media_item(&items);
        assert_eq!(item_type(got.unwrap()), 5);
    }

    /// IMAGE > VIDEO：主列表同时有图与视频时选图。
    #[test]
    fn select_image_before_video() {
        let items = vec![
            json!({
                "type": 5,
                "video_item": { "media": { "encrypt_query_param": "v", "aes_key": "k", "full_url": "https://v.wechat.com/x" } }
            }),
            json!({
                "type": 2,
                "image_item": {
                    "aeskey": "00112233445566778899aabbccddeeff",
                    "media": { "encrypt_query_param": "i", "aes_key": "k" }
                }
            }),
        ];
        let got = select_inbound_media_item(&items);
        assert_eq!(item_type(got.unwrap()), 2);
    }

    #[test]
    fn video_file_extract_cdn_keys() {
        let v = json!({
            "type": 5,
            "video_item": { "media": { "encrypt_query_param": "vq", "aes_key": "abKey", "full_url": "https://v.wechat.com/1" } }
        });
        let triple = extract_cdn_keys_for_any_item(&v).expect("video keys");
        assert_eq!(triple.0, "vq");
        let f = json!({
            "type": 4,
            "file_item": { "file_name": "a.pdf", "media": { "encrypt_query_param": "fq", "aes_key": "k2" } }
        });
        let ft = extract_cdn_keys_for_any_item(&f).expect("file keys");
        assert_eq!(ft.0, "fq");
    }

    #[test]
    fn select_skips_voice_with_stt_text() {
        let items = vec![
            json!({ "type": 3, "voice_item": { "text": "转写", "media": { "encrypt_query_param": "v", "aes_key": "k" } } }),
            json!({
                "type": 4,
                "file_item": { "file_name": "a.pdf", "media": { "encrypt_query_param": "f", "aes_key": "k" } }
            }),
        ];
        let got = select_inbound_media_item(&items);
        assert_eq!(item_type(got.unwrap()), 4);
    }

    #[test]
    fn voice_no_stt_flag() {
        let no = vec![json!({ "type": 3, "voice_item": { "playtime": 1000 } })];
        assert!(has_voice_item_without_stt(&no));
        let ok = vec![json!({ "type": 3, "voice_item": { "text": "x" } })];
        assert!(!has_voice_item_without_stt(&ok));
    }

    #[test]
    fn cdn_get_url_trusted_accepts_wechat_com() {
        assert!(cdn_get_url_trusted("https://file.wechat.com/attach"));
    }

    #[test]
    fn cdn_get_url_trusted_accepts_weixin_qq_com() {
        assert!(cdn_get_url_trusted(
            "https://szfile.weixin.qq.com/cgi-bin/download"
        ));
        assert!(cdn_get_url_trusted("https://dev.weixin.qq.com/ilink"));
    }

    #[test]
    fn normalize_media_aes_key_from_32_hex() {
        let b64 = normalize_media_aes_key("00112233445566778899aabbccddeeff");
        assert!(!b64.is_empty());
        assert_eq!(parse_aes_key(&b64).unwrap().len(), 16);
    }

    #[test]
    fn body_text_camel_case_text_item() {
        let items = vec![json!({
            "type": 1,
            "textItem": { "text": "hello" }
        })];
        assert_eq!(body_from_item_list(&items), "hello");
    }

    #[test]
    fn cdn_get_url_trusted_rejects_untrusted_host() {
        assert!(!cdn_get_url_trusted("https://evil.example.com/steal"));
        assert!(!cdn_get_url_trusted("http://127.0.0.1/"));
    }
}
