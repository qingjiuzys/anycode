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
    anycode_llm::account_api_url()
}

pub fn portal_url() -> String {
    anycode_llm::cloud_portal_url()
}

pub fn gateway_url() -> String {
    anycode_llm::resolve_gateway_host()
}

/// RFC 8628–style device authorization kickoff (`POST /api/v1/devices/link/start`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceLinkStart {
    pub device_code: String,
    #[serde(default)]
    pub user_code: Option<String>,
    pub verification_uri: String,
    #[serde(default)]
    pub verification_uri_complete: Option<String>,
    #[serde(default)]
    pub expires_in: Option<u64>,
}

pub const DEVICE_LINK_REDIRECT_URI: &str = "anycode://link";

pub fn portal_login_url_for_device(device_code: &str) -> String {
    let portal = portal_url();
    let portal = portal.trim_end_matches('/');
    format!(
        "{portal}/login?device_code={}&redirect_uri={}",
        urlencoding::encode(device_code),
        urlencoding::encode(DEVICE_LINK_REDIRECT_URI)
    )
}

pub fn browser_url_for_device_link(start: &DeviceLinkStart) -> String {
    if let Some(complete) = start
        .verification_uri_complete
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        return complete.to_string();
    }
    portal_login_url_for_device(&start.device_code)
}

pub async fn start_device_link() -> Result<DeviceLinkStart> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/v1/devices/link/start",
        account_api_url().trim_end_matches('/')
    );
    let resp = client
        .post(&url)
        .json(&serde_json::json!({
            "client_name": "anyCode",
            "redirect_uri": DEVICE_LINK_REDIRECT_URI,
        }))
        .timeout(Duration::from_secs(30))
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("device link start failed ({status}): {body}");
    }
    resp.json()
        .await
        .context("parse device link start response")
}

pub async fn poll_device_link(device_code: &str) -> Result<CloudSession> {
    let expires = std::time::Instant::now() + Duration::from_secs(120);
    loop {
        if std::time::Instant::now() >= expires {
            anyhow::bail!("device link timed out");
        }
        if let Some(session) = try_poll_device_link(device_code).await? {
            return Ok(session);
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

/// Single poll attempt; `Ok(None)` means still pending (HTTP 202).
pub async fn try_poll_device_link(device_code: &str) -> Result<Option<CloudSession>> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/v1/devices/link/poll",
        account_api_url().trim_end_matches('/')
    );
    let resp = client
        .post(&url)
        .json(&serde_json::json!({ "device_code": device_code }))
        .timeout(Duration::from_secs(30))
        .send()
        .await?;
    if resp.status() == reqwest::StatusCode::ACCEPTED {
        return Ok(None);
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
    Ok(Some(session))
}

pub async fn link_device(device_code: &str) -> Result<CloudSession> {
    poll_device_link(device_code).await
}

pub fn read_access_token() -> Option<String> {
    read_cloud_session().map(|s| s.access_token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portal_login_url_encodes_device_and_redirect() {
        let url = portal_login_url_for_device("dev-abc");
        assert!(url.contains("anycode.work"));
        assert!(url.contains("device_code=dev-abc"));
        assert!(url.contains("redirect_uri=anycode"));
    }
}
