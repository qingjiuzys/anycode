//! Cloud account device link and session storage (`~/.anycode/cloud-session.json`).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudSession {
    pub access_token: String,
    pub refresh_token: String,
    pub user_email: Option<String>,
    pub gateway_url: Option<String>,
}

pub fn cloud_session_path() -> PathBuf {
    anycode_llm::copilot_token::anycode_credentials_dir().join("cloud-session.json")
}

pub fn read_cloud_session() -> Option<CloudSession> {
    let path = cloud_session_path();
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn write_cloud_session(session: &CloudSession) -> Result<()> {
    let path = cloud_session_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(session)?;
    std::fs::write(&path, text).with_context(|| format!("write {}", path.display()))
}

pub fn account_api_url() -> String {
    std::env::var("ANYCODE_ACCOUNT_API_URL").unwrap_or_else(|_| "http://127.0.0.1:43200".into())
}

pub fn portal_url() -> String {
    std::env::var("ANYCODE_ACCOUNT_PORTAL_URL").unwrap_or_else(|_| account_api_url())
}

pub fn gateway_url() -> String {
    std::env::var("ANYCODE_MODEL_GATEWAY_URL").unwrap_or_else(|_| "http://127.0.0.1:43210".into())
}

pub async fn poll_device_link(device_code: &str) -> Result<CloudSession> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/v1/devices/link/poll",
        account_api_url().trim_end_matches('/')
    );
    let expires = std::time::Instant::now() + Duration::from_secs(120);
    loop {
        if std::time::Instant::now() >= expires {
            anyhow::bail!("device link timed out");
        }
        let resp = client
            .post(&url)
            .json(&serde_json::json!({ "device_code": device_code }))
            .send()
            .await?;
        if resp.status() == reqwest::StatusCode::ACCEPTED {
            tokio::time::sleep(Duration::from_secs(2)).await;
            continue;
        }
        let v: serde_json::Value = resp.error_for_status()?.json().await?;
        let gw = v["gateway_url"]
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_else(gateway_url);
        let session = CloudSession {
            access_token: v["access_token"]
                .as_str()
                .context("missing access_token")?
                .to_string(),
            refresh_token: v["refresh_token"]
                .as_str()
                .context("missing refresh_token")?
                .to_string(),
            user_email: v["user"]["email"].as_str().map(|s| s.to_string()),
            gateway_url: Some(gw),
        };
        write_cloud_session(&session)?;
        return Ok(session);
    }
}

pub fn open_portal_login() -> Result<()> {
    let url = format!("{}/login", portal_url().trim_end_matches('/'));
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&url)
            .status()
            .context("open browser for cloud login")?;
    }
    #[cfg(not(target_os = "macos"))]
    {
        std::process::Command::new("xdg-open")
            .arg(&url)
            .status()
            .context("open browser for cloud login")?;
    }
    Ok(())
}

pub async fn run_auth_link(device_code: &str) -> Result<()> {
    let session = poll_device_link(device_code).await?;
    println!(
        "Cloud account linked: {}",
        session.user_email.as_deref().unwrap_or("(unknown)")
    );
    Ok(())
}

pub async fn run_auth_login() -> Result<()> {
    open_portal_login()?;
    println!("在浏览器中登录后，前往「设备」页点击「打开 anyCode 桌面应用」。");
    println!("若使用 CLI，可复制设备码后运行: anycode auth link --code <code>");
    Ok(())
}

pub fn read_access_token() -> Option<String> {
    read_cloud_session().map(|s| s.access_token)
}
