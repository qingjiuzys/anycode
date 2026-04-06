//! iLink Bot HTTP（getupdates / sendmessage）。

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use rand::RngCore;
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;

const DEFAULT_BASE: &str = "https://ilinkai.weixin.qq.com";
const CDN_BASE: &str = "https://novac2c.cdn.weixin.qq.com/c2c";

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
            client: reqwest::Client::builder()
                .user_agent("anycode-wx/0.1")
                .build()
                .expect("reqwest client"),
        }
    }

    fn headers(&self) -> reqwest::header::HeaderMap {
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
        h
    }

    async fn post_json(&self, path: &str, body: Value, timeout_ms: u64) -> Result<Value> {
        let url = format!("{}/{}", self.base_url, path.trim_start_matches('/'));
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
            if v.get("ret").and_then(|x| x.as_i64()) == Some(-2) {
                if attempt == 3 {
                    tracing::warn!("sendmessage 限流，已放弃");
                    return Ok(());
                }
                tracing::warn!(delay_ms, "sendmessage 限流，重试");
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                delay_ms = (delay_ms * 2).min(60_000);
                continue;
            }
            return Ok(());
        }
        Ok(())
    }
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

pub fn build_outbound_text(
    bot_account_id: &str,
    to_user_id: &str,
    context_token: &str,
    text: &str,
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
            "type": 1,
            "text_item": { "text": text }
        }]
    })
}

pub struct WxSender {
    api: Arc<WeChatApi>,
    bot_id: String,
    counter: std::sync::atomic::AtomicU64,
}

impl WxSender {
    pub fn new(api: Arc<WeChatApi>, bot_id: String) -> Self {
        Self {
            api,
            bot_id,
            counter: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub async fn send_text(&self, to_user_id: &str, context_token: &str, text: &str) -> Result<()> {
        let n = self
            .counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let client_id = format!("anycode-{}-{}", chrono::Utc::now().timestamp_millis(), n);
        let msg = build_outbound_text(&self.bot_id, to_user_id, context_token, text, &client_id);
        self.api.send_message(msg).await
    }
}
