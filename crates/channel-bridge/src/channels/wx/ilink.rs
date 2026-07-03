//! iLink Bot HTTP（getupdates / sendmessage）。

use super::fields::{i64_snake_camel, str_snake_camel};
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use rand::RngCore;
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;

const DEFAULT_BASE: &str = "https://ilinkai.weixin.qq.com";
const SESSION_EXPIRED: i64 = -14;
const RATE_LIMIT_RET: i64 = -2;
pub(crate) const CDN_BASE: &str = "https://novac2c.cdn.weixin.qq.com/c2c";

/// openclaw-weixin `package.json` `ilink_appid`.
const ILINK_APP_ID: &str = "bot";
const BOT_AGENT: &str = "anyCode";

/// Encode semver as `0x00MMNNPP` (openclaw-weixin `buildClientVersion`).
pub fn build_ilink_client_version(version: &str) -> u32 {
    let parts: Vec<u32> = version.split('.').map(|p| p.parse().unwrap_or(0)).collect();
    let major = parts.first().copied().unwrap_or(0) & 0xff;
    let minor = parts.get(1).copied().unwrap_or(0) & 0xff;
    let patch = parts.get(2).copied().unwrap_or(0) & 0xff;
    (major << 16) | (minor << 8) | patch
}

pub fn build_base_info_json() -> Value {
    serde_json::json!({
        "channel_version": env!("CARGO_PKG_VERSION"),
        "bot_agent": BOT_AGENT,
    })
}

pub fn with_base_info(mut body: Value) -> Value {
    if let Some(obj) = body.as_object_mut() {
        obj.insert("base_info".into(), build_base_info_json());
    }
    body
}

fn generate_uin_b64() -> String {
    let mut b = [0u8; 4];
    rand::thread_rng().fill_bytes(&mut b);
    B64.encode(b)
}

fn sanitize_base_url(base: &str) -> String {
    let base = base.trim().trim_end_matches('/');
    if base.is_empty() {
        return DEFAULT_BASE.to_string();
    }
    if let Ok(u) = url::Url::parse(base) {
        let host = u.host_str().unwrap_or("");
        if host == "127.0.0.1" || host == "localhost" {
            return base.to_string();
        }
        let ok = host == "weixin.qq.com"
            || host == "wechat.com"
            || host.ends_with(".weixin.qq.com")
            || host.ends_with(".wechat.com");
        if u.scheme() == "https" && ok {
            return base.to_string();
        }
    }
    DEFAULT_BASE.to_string()
}

pub struct WeChatApi {
    token: String,
    base_url: String,
    uin: String,
    route_tag: Option<String>,
    client: reqwest::Client,
}

impl WeChatApi {
    pub fn http_client(&self) -> &reqwest::Client {
        &self.client
    }

    pub fn new(token: String, base_url: String) -> Self {
        Self {
            token,
            base_url: sanitize_base_url(&base_url),
            uin: generate_uin_b64(),
            route_tag: None,
            client: reqwest::Client::builder()
                .user_agent(anycode_core::user_agent("anycode-wx"))
                .build()
                .expect("reqwest client"),
        }
    }

    pub fn with_route_tag(mut self, route_tag: Option<String>) -> Self {
        self.route_tag = route_tag.filter(|s| !s.trim().is_empty());
        self
    }

    fn headers(&self) -> reqwest::header::HeaderMap {
        let client_version = build_ilink_client_version(env!("CARGO_PKG_VERSION")).to_string();
        let mut h = reqwest::header::HeaderMap::new();
        h.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        h.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", self.token).parse().unwrap(),
        );
        h.insert("AuthorizationType", "ilink_bot_token".parse().unwrap());
        h.insert("X-WECHAT-UIN", self.uin.parse().unwrap());
        h.insert("iLink-App-Id", ILINK_APP_ID.parse().unwrap());
        h.insert("iLink-App-ClientVersion", client_version.parse().unwrap());
        if let Some(tag) = self.route_tag.as_deref() {
            if let Ok(v) = tag.parse() {
                h.insert("SKRouteTag", v);
            }
        }
        h
    }

    async fn post_json(&self, path: &str, body: Value, timeout_ms: u64) -> Result<Value> {
        let url = format!("{}/{}", self.base_url, path.trim_start_matches('/'));
        let body = with_base_info(body);
        let res = self
            .client
            .post(&url)
            .headers(self.headers())
            .timeout(std::time::Duration::from_millis(timeout_ms))
            .json(&body)
            .send()
            .await
            .with_context(|| format!("POST {}", url))?;
        if !res.status().is_success() {
            let status = res.status();
            let t = res.text().await.unwrap_or_default();
            anyhow::bail!(
                "HTTP {}: {}",
                status,
                t.chars().take(500).collect::<String>()
            );
        }
        res.json().await.context("json")
    }

    pub async fn get_updates(&self, buf: Option<&str>) -> Result<Value> {
        let body = match buf {
            Some(b) if !b.is_empty() => serde_json::json!({ "get_updates_buf": b }),
            _ => serde_json::json!({}),
        };
        self.post_json("ilink/bot/getupdates", body, 35_000).await
    }

    pub async fn get_upload_url(&self, body: Value) -> Result<Value> {
        let resp = self
            .post_json("ilink/bot/getuploadurl", body, 30_000)
            .await?;
        ensure_api_response_ok(&resp, "getUploadUrl")?;
        Ok(resp)
    }

    pub async fn send_message(&self, msg: Value) -> Result<()> {
        let mut delay_ms: u64 = 10_000;
        for attempt in 0..=3 {
            let v = self
                .post_json(
                    "ilink/bot/sendmessage",
                    serde_json::json!({ "msg": msg }),
                    15_000,
                )
                .await?;
            match classify_send_response(&v) {
                Ok(()) => return Ok(()),
                Err(SendMessageFailure::StaleSession { detail }) => {
                    return Err(anyhow::anyhow!("stale wechat session: {detail}"));
                }
                Err(SendMessageFailure::RateLimited { detail }) if attempt < 3 => {
                    tracing::warn!(
                        attempt = attempt + 1,
                        delay_ms,
                        detail,
                        "sendmessage rate limited, retrying"
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                    delay_ms = (delay_ms * 2).min(60_000);
                }
                Err(e) => return Err(anyhow::Error::new(e)),
            }
        }
        Err(anyhow::anyhow!(
            "sendmessage rate limited after retries (iLink ret={RATE_LIMIT_RET})"
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SendMessageFailure {
    StaleSession { detail: String },
    RateLimited { detail: String },
    Api { detail: String },
}

impl std::fmt::Display for SendMessageFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StaleSession { detail } | Self::RateLimited { detail } | Self::Api { detail } => {
                write!(f, "{detail}")
            }
        }
    }
}

impl std::error::Error for SendMessageFailure {}

fn response_errmsg(v: &Value) -> String {
    str_snake_camel(v, "errmsg", "errMsg")
        .or_else(|| str_snake_camel(v, "msg", "message"))
        .unwrap_or("unknown error")
        .to_string()
}

fn is_stale_session_ret(ret: Option<i64>, errcode: Option<i64>, errmsg: &str) -> bool {
    if ret == Some(SESSION_EXPIRED) || errcode == Some(SESSION_EXPIRED) {
        return true;
    }
    let minus_two = ret == Some(RATE_LIMIT_RET) || errcode == Some(RATE_LIMIT_RET);
    minus_two && errmsg.eq_ignore_ascii_case("unknown error")
}

fn is_rate_limited_ret(ret: Option<i64>, errcode: Option<i64>, errmsg: &str) -> bool {
    (ret == Some(RATE_LIMIT_RET) || errcode == Some(RATE_LIMIT_RET))
        && !is_stale_session_ret(ret, errcode, errmsg)
}

pub fn api_response_ok(v: &Value) -> bool {
    send_response_ok(v)
}

pub fn ensure_api_response_ok(v: &Value, label: &str) -> Result<()> {
    if send_response_ok(v) {
        return Ok(());
    }
    let ret = i64_snake_camel(v, "ret", "ret");
    let errcode = i64_snake_camel(v, "errcode", "errCode");
    let errmsg = response_errmsg(v);
    anyhow::bail!("{label} failed: ret={ret:?} errcode={errcode:?} errmsg={errmsg}");
}

fn send_response_ok(v: &Value) -> bool {
    let ret = i64_snake_camel(v, "ret", "ret");
    let errcode = i64_snake_camel(v, "errcode", "errCode");
    match (ret, errcode) {
        (Some(0), _) | (_, Some(0)) => true,
        (None, None) => true,
        (Some(r), None) => r == 0,
        (None, Some(c)) => c == 0,
        _ => false,
    }
}

fn classify_send_response(v: &Value) -> Result<(), SendMessageFailure> {
    if send_response_ok(v) {
        return Ok(());
    }
    let ret = i64_snake_camel(v, "ret", "ret");
    let errcode = i64_snake_camel(v, "errcode", "errCode");
    let errmsg = response_errmsg(v);
    let detail = format!("ret={ret:?} errcode={errcode:?} errmsg={errmsg}");
    if is_stale_session_ret(ret, errcode, &errmsg) {
        return Err(SendMessageFailure::StaleSession { detail });
    }
    if is_rate_limited_ret(ret, errcode, &errmsg) {
        return Err(SendMessageFailure::RateLimited { detail });
    }
    Err(SendMessageFailure::Api { detail })
}

pub(crate) fn is_stale_wechat_session(err: &anyhow::Error) -> bool {
    err.to_string().contains("stale wechat session")
}

pub fn load_sync_buf(data_root: &Path) -> String {
    let p = data_root.join("get_updates_buf");
    let Ok(raw) = std::fs::read_to_string(&p) else {
        return String::new();
    };
    serde_json::from_str::<String>(&raw).unwrap_or_default()
}

pub fn save_sync_buf(data_root: &Path, buf: &str) -> Result<()> {
    std::fs::create_dir_all(data_root)?;
    let p = data_root.join("get_updates_buf");
    let raw = serde_json::to_string(buf)?;
    std::fs::write(&p, raw + "\n")?;
    Ok(())
}

pub fn cdn_download_url(encrypt_query_param: &str) -> Result<String> {
    if !encrypt_query_param
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "%=&+._~-/".contains(c))
    {
        anyhow::bail!("非法 CDN 参数");
    }
    Ok(format!(
        "{}/download?encrypted_query_param={}",
        CDN_BASE,
        urlencoding::encode(encrypt_query_param)
    ))
}

pub fn cdn_upload_url(upload_param: &str, filekey: &str) -> Result<String> {
    if !upload_param
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "%=&+._~-/".contains(c))
    {
        anyhow::bail!("非法 upload_param");
    }
    if !filekey.chars().all(|c| c.is_ascii_hexdigit()) {
        anyhow::bail!("非法 filekey");
    }
    Ok(format!(
        "{}/upload?encrypted_query_param={}&filekey={}",
        CDN_BASE,
        urlencoding::encode(upload_param),
        urlencoding::encode(filekey)
    ))
}

pub fn build_outbound_text(
    bot_account_id: &str,
    to_user_id: &str,
    context_token: Option<&str>,
    text: &str,
    client_id: &str,
) -> Value {
    let mut msg = serde_json::json!({
        "from_user_id": bot_account_id,
        "to_user_id": to_user_id,
        "client_id": client_id,
        "message_type": 2,
        "message_state": 2,
        "item_list": [{
            "type": 1,
            "text_item": { "text": text }
        }]
    });
    if let Some(tok) = context_token.filter(|s| !s.is_empty()) {
        msg["context_token"] = serde_json::json!(tok);
    }
    msg
}

/// openclaw-weixin outbound `CDNMedia.aes_key`: base64(utf8(hex string of 16 bytes)) for all media kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutboundCdnKeyEncoding {
    Image,
    FileOrVideo,
}

pub fn outbound_cdn_aes_key_b64(raw_key: &[u8; 16], encoding: OutboundCdnKeyEncoding) -> String {
    use base64::{engine::general_purpose::STANDARD as B64, Engine};
    let _ = encoding;
    B64.encode(hex::encode(raw_key).as_bytes())
}

fn build_cdn_media_json(
    encrypt_query_param: &str,
    aes_key: &[u8; 16],
    encoding: OutboundCdnKeyEncoding,
) -> Value {
    serde_json::json!({
        "encrypt_query_param": encrypt_query_param,
        "aes_key": outbound_cdn_aes_key_b64(aes_key, encoding),
        "encrypt_type": 1,
    })
}

pub fn build_outbound_file(
    bot_account_id: &str,
    to_user_id: &str,
    context_token: &str,
    file_name: &str,
    encrypt_query_param: &str,
    aes_key: &[u8; 16],
    plaintext_len: u64,
    client_id: &str,
) -> Value {
    serde_json::json!({
        "from_user_id": bot_account_id,
        "to_user_id": to_user_id,
        "client_id": client_id,
        "message_type": 2,
        "message_state": 2,
        "context_token": context_token,
        "item_list": [{
            "type": 4,
            "file_item": {
                "file_name": file_name,
                "len": plaintext_len.to_string(),
                "media": build_cdn_media_json(
                    encrypt_query_param,
                    aes_key,
                    OutboundCdnKeyEncoding::FileOrVideo,
                ),
            }
        }]
    })
}

pub fn build_outbound_image(
    bot_account_id: &str,
    to_user_id: &str,
    context_token: &str,
    encrypt_query_param: &str,
    aes_key: &[u8; 16],
    mid_size: u64,
    client_id: &str,
) -> Value {
    serde_json::json!({
        "from_user_id": bot_account_id,
        "to_user_id": to_user_id,
        "client_id": client_id,
        "message_type": 2,
        "message_state": 2,
        "context_token": context_token,
        "item_list": [{
            "type": 2,
            "image_item": {
                "mid_size": mid_size,
                "media": build_cdn_media_json(
                    encrypt_query_param,
                    aes_key,
                    OutboundCdnKeyEncoding::Image,
                ),
            }
        }]
    })
}

pub fn build_outbound_video(
    bot_account_id: &str,
    to_user_id: &str,
    context_token: &str,
    encrypt_query_param: &str,
    aes_key: &[u8; 16],
    video_size: u64,
    client_id: &str,
) -> Value {
    serde_json::json!({
        "from_user_id": bot_account_id,
        "to_user_id": to_user_id,
        "client_id": client_id,
        "message_type": 2,
        "message_state": 2,
        "context_token": context_token,
        "item_list": [{
            "type": 5,
            "video_item": {
                "video_size": video_size,
                "media": build_cdn_media_json(
                    encrypt_query_param,
                    aes_key,
                    OutboundCdnKeyEncoding::FileOrVideo,
                ),
            }
        }]
    })
}

pub struct WxSender {
    api: Arc<WeChatApi>,
    bot_id: String,
    counter: std::sync::atomic::AtomicU64,
    outbound_log: Option<std::path::PathBuf>,
}

impl WxSender {
    pub fn new(api: Arc<WeChatApi>, bot_id: String) -> Self {
        Self {
            api,
            bot_id,
            counter: std::sync::atomic::AtomicU64::new(0),
            outbound_log: None,
        }
    }

    pub fn api(&self) -> &WeChatApi {
        &self.api
    }

    pub fn with_outbound_log(mut self, path: std::path::PathBuf) -> Self {
        self.outbound_log = Some(path);
        self
    }

    fn next_client_id(&self, prefix: &str) -> String {
        let n = self
            .counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        format!(
            "anycode-{prefix}-{}-{n}",
            chrono::Utc::now().timestamp_millis()
        )
    }

    async fn send_message_with_retry(&self, mut msg: Value, label: &str) -> Result<()> {
        let mut delay_ms: u64 = 2_000;
        let mut tried_tokenless = false;
        loop {
            let mut last_err = None;
            for attempt in 0..=3 {
                match self.api.send_message(msg.clone()).await {
                    Ok(()) => return Ok(()),
                    Err(e) if is_stale_wechat_session(&e) && !tried_tokenless => {
                        tried_tokenless = true;
                        if let Some(obj) = msg.as_object_mut() {
                            obj.remove("context_token");
                        }
                        tracing::warn!(
                            label,
                            "wx context_token stale, retrying without context_token"
                        );
                        break;
                    }
                    Err(e) if is_stale_wechat_session(&e) => return Err(e),
                    Err(e) if attempt < 3 => {
                        tracing::warn!(
                            attempt = attempt + 1,
                            delay_ms,
                            error = %e,
                            label,
                            "wx send_message transient failure, retrying"
                        );
                        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                        delay_ms = (delay_ms * 2).min(30_000);
                        last_err = Some(e);
                    }
                    Err(e) => return Err(e),
                }
            }
            if tried_tokenless && last_err.is_none() {
                continue;
            }
            if let Some(e) = last_err {
                return Err(e);
            }
            anyhow::bail!("wx {label} send exhausted retries");
        }
    }

    pub async fn send_text(&self, to_user_id: &str, context_token: &str, text: &str) -> Result<()> {
        use super::outbound_queue::{append_outbound_record, OutboundRecord};

        let marker = text
            .split_whitespace()
            .find(|part| part.starts_with("[anycode-e2e:"))
            .map(|s| s.trim_matches(&['[', ']'][..]).to_string());
        let n = self
            .counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let client_id = format!("anycode-{}-{}", chrono::Utc::now().timestamp_millis(), n);
        if let Some(path) = &self.outbound_log {
            append_outbound_record(
                path,
                &OutboundRecord {
                    ts: chrono::Utc::now().to_rfc3339(),
                    channel: "wechat".into(),
                    to_user_id: to_user_id.to_string(),
                    status: "pending".into(),
                    marker: marker.clone(),
                    retry_count: 0,
                    last_error: String::new(),
                    chars: text.chars().count(),
                },
            );
        }

        let mut use_token: Option<&str> = Some(context_token);
        let mut tried_tokenless = false;
        loop {
            let msg = build_outbound_text(&self.bot_id, to_user_id, use_token, text, &client_id);
            match self.api.send_message(msg).await {
                Ok(()) => {
                    if let Some(path) = &self.outbound_log {
                        append_outbound_record(
                            path,
                            &OutboundRecord {
                                ts: chrono::Utc::now().to_rfc3339(),
                                channel: "wechat".into(),
                                to_user_id: to_user_id.to_string(),
                                status: "sent".into(),
                                marker: marker.clone(),
                                retry_count: 0,
                                last_error: String::new(),
                                chars: text.chars().count(),
                            },
                        );
                    }
                    return Ok(());
                }
                Err(e)
                    if is_stale_wechat_session(&e) && !tried_tokenless && use_token.is_some() =>
                {
                    tried_tokenless = true;
                    use_token = None;
                    tracing::warn!("wx context_token stale, retrying without context_token");
                }
                Err(e) => {
                    if let Some(path) = &self.outbound_log {
                        append_outbound_record(
                            path,
                            &OutboundRecord {
                                ts: chrono::Utc::now().to_rfc3339(),
                                channel: "wechat".into(),
                                to_user_id: to_user_id.to_string(),
                                status: "failed".into(),
                                marker: marker.clone(),
                                retry_count: if tried_tokenless { 1 } else { 0 },
                                last_error: e.to_string(),
                                chars: text.chars().count(),
                            },
                        );
                    }
                    return Err(e);
                }
            }
        }
    }

    /// Upload bytes to WeChat CDN and send as `file_item` (type 4).
    pub async fn send_file(
        &self,
        to_user_id: &str,
        context_token: &str,
        file_name: &str,
        plaintext: &[u8],
    ) -> Result<()> {
        use super::cdn_upload::upload_bytes_to_cdn;

        let media = upload_bytes_to_cdn(self.api.as_ref(), plaintext, to_user_id).await?;
        self.send_file_message(to_user_id, context_token, file_name, &media)
            .await
    }

    pub async fn send_file_message(
        &self,
        to_user_id: &str,
        context_token: &str,
        file_name: &str,
        media: &super::cdn_upload::UploadedCdnMedia,
    ) -> Result<()> {
        let client_id = self.next_client_id("file");
        let msg = build_outbound_file(
            &self.bot_id,
            to_user_id,
            context_token,
            file_name,
            &media.encrypt_query_param,
            &media.aes_key,
            media.raw_size as u64,
            &client_id,
        );
        self.send_message_with_retry(msg, "send_file_message").await
    }

    pub async fn send_image_message(
        &self,
        to_user_id: &str,
        context_token: &str,
        media: &super::cdn_upload::UploadedCdnMedia,
    ) -> Result<()> {
        let client_id = self.next_client_id("image");
        let msg = build_outbound_image(
            &self.bot_id,
            to_user_id,
            context_token,
            &media.encrypt_query_param,
            &media.aes_key,
            media.ciphertext_size as u64,
            &client_id,
        );
        self.send_message_with_retry(msg, "send_image_message")
            .await
    }

    pub async fn send_video_message(
        &self,
        to_user_id: &str,
        context_token: &str,
        media: &super::cdn_upload::UploadedCdnMedia,
    ) -> Result<()> {
        let client_id = self.next_client_id("video");
        let msg = build_outbound_video(
            &self.bot_id,
            to_user_id,
            context_token,
            &media.encrypt_query_param,
            &media.aes_key,
            media.ciphertext_size as u64,
            &client_id,
        );
        self.send_message_with_retry(msg, "send_video_message")
            .await
    }
}

#[cfg(test)]
mod send_retry_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn send_retry_backoff_caps_at_thirty_seconds() {
        let mut delay_ms: u64 = 2_000;
        for _ in 0..5 {
            delay_ms = (delay_ms * 2).min(30_000);
        }
        assert_eq!(delay_ms, 30_000);
    }

    #[test]
    fn stale_session_ret_minus_two_unknown_error() {
        let resp = json!({ "ret": -2, "errmsg": "unknown error" });
        assert!(is_stale_session_ret(
            i64_snake_camel(&resp, "ret", "ret"),
            i64_snake_camel(&resp, "errcode", "errCode"),
            &response_errmsg(&resp),
        ));
        assert!(matches!(
            classify_send_response(&resp),
            Err(SendMessageFailure::StaleSession { .. })
        ));
    }

    #[test]
    fn rate_limit_ret_minus_two_without_unknown_error() {
        let resp = json!({ "ret": -2, "errmsg": "rate limited" });
        assert!(is_rate_limited_ret(
            i64_snake_camel(&resp, "ret", "ret"),
            i64_snake_camel(&resp, "errcode", "errCode"),
            &response_errmsg(&resp),
        ));
        assert!(matches!(
            classify_send_response(&resp),
            Err(SendMessageFailure::RateLimited { .. })
        ));
    }

    #[test]
    fn send_response_ok_when_ret_zero() {
        let resp = json!({ "ret": 0 });
        assert!(send_response_ok(&resp));
        assert!(classify_send_response(&resp).is_ok());
    }

    #[test]
    fn outbound_text_omits_empty_context_token() {
        let msg = build_outbound_text("bot", "user", None, "hi", "cid");
        assert!(msg.get("context_token").is_none());
    }

    #[test]
    fn outbound_file_includes_len_and_encrypt_type() {
        let key = [0xABu8; 16];
        let msg = build_outbound_file(
            "bot",
            "user",
            "ctx",
            "report.pdf",
            "enc_param",
            &key,
            1234,
            "cid",
        );
        let item = &msg["item_list"][0];
        assert_eq!(item["type"], 4);
        assert_eq!(item["file_item"]["len"], "1234");
        assert!(item["file_item"].get("md5").is_none());
        assert_eq!(item["file_item"]["media"]["encrypt_type"], 1);
        let aes_key = item["file_item"]["media"]["aes_key"]
            .as_str()
            .expect("aes_key");
        assert_eq!(
            aes_key,
            outbound_cdn_aes_key_b64(&key, OutboundCdnKeyEncoding::FileOrVideo)
        );
    }

    #[test]
    fn outbound_file_and_image_share_openclaw_aes_key_encoding() {
        let key = [0x11u8; 16];
        let image = outbound_cdn_aes_key_b64(&key, OutboundCdnKeyEncoding::Image);
        let file = outbound_cdn_aes_key_b64(&key, OutboundCdnKeyEncoding::FileOrVideo);
        assert_eq!(image, file);
    }

    #[test]
    fn outbound_image_includes_mid_size() {
        let key = [0x22u8; 16];
        let msg = build_outbound_image("bot", "user", "ctx", "enc", &key, 32, "cid");
        let item = &msg["item_list"][0];
        assert_eq!(item["type"], 2);
        assert_eq!(item["image_item"]["mid_size"], 32);
        assert_eq!(item["image_item"]["media"]["encrypt_type"], 1);
        assert_eq!(
            item["image_item"]["media"]["aes_key"].as_str().unwrap(),
            outbound_cdn_aes_key_b64(&key, OutboundCdnKeyEncoding::Image)
        );
    }

    #[test]
    fn outbound_video_includes_video_size() {
        let key = [0x33u8; 16];
        let msg = build_outbound_video("bot", "user", "ctx", "enc", &key, 64, "cid");
        let item = &msg["item_list"][0];
        assert_eq!(item["type"], 5);
        assert_eq!(item["video_item"]["video_size"], 64);
        assert_eq!(
            item["video_item"]["media"]["aes_key"].as_str().unwrap(),
            outbound_cdn_aes_key_b64(&key, OutboundCdnKeyEncoding::FileOrVideo)
        );
    }

    #[test]
    fn build_ilink_client_version_matches_openclaw_rule() {
        assert_eq!(build_ilink_client_version("2.4.3"), 0x00020403);
        assert_eq!(build_ilink_client_version("0.2.3"), 0x00000203);
    }

    #[test]
    fn with_base_info_injects_channel_version_and_bot_agent() {
        let body = with_base_info(json!({ "filekey": "abc" }));
        let info = body.get("base_info").expect("base_info");
        assert_eq!(info["bot_agent"], "anyCode");
        assert!(info["channel_version"].as_str().is_some());
    }
}

#[cfg(test)]
mod wire_tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn post_json_sends_ilink_app_headers() {
        let server = MockServer::start().await;
        let client_version = build_ilink_client_version(env!("CARGO_PKG_VERSION")).to_string();
        Mock::given(method("POST"))
            .and(path("/ilink/bot/getupdates"))
            .and(header("iLink-App-Id", "bot"))
            .and(header("iLink-App-ClientVersion", client_version.as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "ret": 0 })))
            .mount(&server)
            .await;

        let api = WeChatApi::new("tok".into(), server.uri());
        api.get_updates(None).await.expect("getupdates");
    }
}
