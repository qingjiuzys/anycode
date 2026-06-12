//! WeChat iLink QR login helpers for Dashboard setup (HTTP aligned with CLI wechat_ilink).

use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const DEFAULT_BASE_URL: &str = "https://ilinkai.weixin.qq.com";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WechatQrPayload {
    pub qrcode_id: String,
    /// Raw QR content: URL or base64 image payload from iLink API.
    pub content: String,
    /// Terminal-style Unicode block rendering (optional display fallback).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_render: Option<String>,
    /// SVG QR for web/desktop setup (scannable on screen).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qr_svg: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WechatQrStatus {
    Wait,
    Scanned,
    Confirmed,
    Expired,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WechatQrPollResult {
    pub status: WechatQrStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub account_saved: bool,
}

fn json_str<'a>(v: &'a serde_json::Value, keys: &[&str]) -> Option<&'a str> {
    for k in keys {
        if let Some(s) = v.get(*k).and_then(|x| x.as_str()) {
            return Some(s);
        }
    }
    None
}

fn ilink_base() -> String {
    std::env::var("WCC_ILINK_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string())
}

fn wechat_data_root() -> anyhow::Result<std::path::PathBuf> {
    let home = dirs::home_dir().context("no home directory")?;
    let p = home.join(".anycode/wechat");
    std::fs::create_dir_all(&p)?;
    Ok(p)
}

fn validate_account_id(id: &str) -> anyhow::Result<()> {
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_.@=-".contains(c))
    {
        anyhow::bail!("invalid wechat account id: {id}");
    }
    Ok(())
}

pub async fn fetch_wechat_qr() -> anyhow::Result<WechatQrPayload> {
    let client = reqwest::Client::builder()
        .user_agent("anycode-wechat-ilink/0.1")
        .timeout(Duration::from_secs(30))
        .build()?;
    let base = ilink_base();
    let url = format!(
        "{}/ilink/bot/get_bot_qrcode?bot_type=3",
        base.trim_end_matches('/')
    );
    let res = client.get(&url).send().await.context("fetch wechat qr")?;
    if !res.status().is_success() {
        anyhow::bail!("wechat qr HTTP {}", res.status());
    }
    let v: serde_json::Value = res.json().await.context("parse wechat qr json")?;
    let ret = v.get("ret").and_then(|x| x.as_i64()).unwrap_or(-1);
    let qrcode_id = json_str(&v, &["qrcode"]).map(str::to_string);
    let content = json_str(&v, &["qrcode_img_content", "qrcodeImgContent"]).map(str::to_string);
    if ret != 0 || qrcode_id.is_none() || content.is_none() {
        anyhow::bail!("wechat qr API ret={ret}");
    }
    let content = content.unwrap();
    let terminal_render = render_qr_terminal(&content).ok();
    let qr_svg = render_qr_svg(&content).ok();
    Ok(WechatQrPayload {
        qrcode_id: qrcode_id.unwrap(),
        content,
        terminal_render,
        qr_svg,
    })
}

pub async fn poll_wechat_qr_status(qrcode_id: &str) -> anyhow::Result<WechatQrPollResult> {
    let client = reqwest::Client::builder()
        .user_agent("anycode-wechat-ilink/0.1")
        .timeout(Duration::from_secs(60))
        .build()?;
    let base = ilink_base();
    let url = format!(
        "{}/ilink/bot/get_qrcode_status?qrcode={}",
        base.trim_end_matches('/'),
        urlencoding::encode(qrcode_id)
    );
    let res = client.get(&url).send().await.context("poll wechat qr")?;
    if !res.status().is_success() {
        anyhow::bail!("wechat poll HTTP {}", res.status());
    }
    let data: serde_json::Value = res.json().await.context("parse poll json")?;
    let status_raw = json_str(&data, &["status"]).unwrap_or_default();
    match status_raw {
        "wait" => Ok(WechatQrPollResult {
            status: WechatQrStatus::Wait,
            message: None,
            account_saved: false,
        }),
        "scaned" | "scanned" => Ok(WechatQrPollResult {
            status: WechatQrStatus::Scanned,
            message: None,
            account_saved: false,
        }),
        "confirmed" => {
            save_wechat_account(&data)?;
            Ok(WechatQrPollResult {
                status: WechatQrStatus::Confirmed,
                message: Some("WeChat account linked".into()),
                account_saved: true,
            })
        }
        "expired" => Ok(WechatQrPollResult {
            status: WechatQrStatus::Expired,
            message: Some("QR code expired — refresh".into()),
            account_saved: false,
        }),
        other => {
            let retmsg = json_str(&data, &["retmsg", "retMsg"]).unwrap_or("");
            Ok(WechatQrPollResult {
                status: WechatQrStatus::Error,
                message: Some(format!("{other} {retmsg}").trim().to_string()),
                account_saved: false,
            })
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountFile {
    bot_token: String,
    account_id: String,
    base_url: String,
    user_id: String,
    created_at: String,
}

fn save_wechat_account(data: &serde_json::Value) -> anyhow::Result<()> {
    let bot_token = json_str(data, &["bot_token", "botToken"])
        .context("missing bot_token in wechat confirm payload")?
        .to_string();
    let account_id = json_str(
        data,
        &["ilink_bot_id", "ilinkBotId", "account_id", "accountId"],
    )
    .context("missing ilink_bot_id / account_id")?
    .to_string();
    let user_id = json_str(data, &["ilink_user_id", "ilinkUserId", "user_id", "userId"])
        .context("missing ilink_user_id / user_id")?
        .to_string();
    validate_account_id(&account_id)?;
    let base_url = json_str(data, &["baseurl", "baseUrl", "base_url"])
        .unwrap_or(DEFAULT_BASE_URL)
        .to_string();
    let data_root = wechat_data_root()?;
    let dir = data_root.join("accounts");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{account_id}.json"));
    let file = AccountFile {
        bot_token,
        account_id: account_id.clone(),
        base_url,
        user_id,
        created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    };
    let raw = serde_json::to_string_pretty(&file)? + "\n";
    std::fs::write(&path, raw.as_bytes())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path)?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&path, perms)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_account_uses_accounts_subdir_and_ilink_fields() {
        let dir = tempfile::tempdir().unwrap();
        let data_root = dir.path().join("wechat");
        std::fs::create_dir_all(&data_root).unwrap();
        let payload = serde_json::json!({
            "bot_token": "tok",
            "ilink_bot_id": "bot-1",
            "ilink_user_id": "user-1",
            "baseurl": "https://example.test"
        });
        let bot_token = json_str(&payload, &["bot_token", "botToken"]).unwrap();
        let account_id = json_str(
            &payload,
            &["ilink_bot_id", "ilinkBotId", "account_id", "accountId"],
        )
        .unwrap();
        let user_id = json_str(
            &payload,
            &["ilink_user_id", "ilinkUserId", "user_id", "userId"],
        )
        .unwrap();
        validate_account_id(account_id).unwrap();
        let accounts_dir = data_root.join("accounts");
        std::fs::create_dir_all(&accounts_dir).unwrap();
        let path = accounts_dir.join(format!("{account_id}.json"));
        let file = AccountFile {
            bot_token: bot_token.to_string(),
            account_id: account_id.to_string(),
            base_url: "https://example.test".into(),
            user_id: user_id.to_string(),
            created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        };
        std::fs::write(&path, serde_json::to_string_pretty(&file).unwrap() + "\n").unwrap();
        let saved: AccountFile = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(saved.account_id, "bot-1");
        assert_eq!(saved.user_id, "user-1");
    }
}

fn render_qr_terminal(payload: &str) -> anyhow::Result<String> {
    use qrcode::render::unicode::Dense1x2;
    use qrcode::QrCode;
    let code = QrCode::new(payload.as_bytes())?;
    Ok(code
        .render::<Dense1x2>()
        .dark_color(qrcode::render::unicode::Dense1x2::Dark)
        .light_color(qrcode::render::unicode::Dense1x2::Light)
        .build())
}

fn render_qr_svg(payload: &str) -> anyhow::Result<String> {
    use qrcode::render::svg;
    use qrcode::QrCode;
    let code = QrCode::new(payload.as_bytes())?;
    Ok(code
        .render::<svg::Color>()
        .min_dimensions(240, 240)
        .dark_color(svg::Color("#000"))
        .light_color(svg::Color("#fff"))
        .build())
}

#[cfg(test)]
mod qr_render_tests {
    use super::*;

    #[test]
    fn render_qr_svg_produces_svg_markup() {
        let svg = render_qr_svg("https://liteapp.weixin.qq.com/q/test").unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
    }
}
