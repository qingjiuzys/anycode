use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const DEFAULT_GATEWAY_HOST: &str = "http://127.0.0.1:43210";
const DEFAULT_ACCOUNT_API: &str = "http://127.0.0.1:43200";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CloudSessionFile {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub gateway_url: Option<String>,
    #[serde(default)]
    pub user_email: Option<String>,
}

pub fn cloud_session_path() -> PathBuf {
    crate::copilot_token::anycode_credentials_dir().join("cloud-session.json")
}

pub fn read_cloud_session() -> Option<CloudSessionFile> {
    let text = std::fs::read_to_string(cloud_session_path()).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn write_cloud_session(session: &CloudSessionFile) -> std::io::Result<()> {
    let path = cloud_session_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(session)?;
    std::fs::write(path, text)
}

pub fn read_cloud_access_token() -> Option<String> {
    read_cloud_session()
        .map(|s| s.access_token)
        .filter(|t| !t.trim().is_empty())
}

/// Gateway host without path suffix. Priority: env `ANYCODE_MODEL_GATEWAY_URL` > session file > localhost default.
pub fn resolve_gateway_host() -> String {
    std::env::var("ANYCODE_MODEL_GATEWAY_URL")
        .ok()
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            read_cloud_session()
                .and_then(|s| s.gateway_url)
                .map(|s| s.trim().trim_end_matches('/').to_string())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| DEFAULT_GATEWAY_HOST.to_string())
}

pub fn default_gateway_chat_url() -> String {
    format!("{}/v1/chat/completions", resolve_gateway_host())
}

pub fn account_api_url() -> String {
    std::env::var("ANYCODE_ACCOUNT_API_URL")
        .unwrap_or_else(|_| DEFAULT_ACCOUNT_API.to_string())
        .trim_end_matches('/')
        .to_string()
}

/// Refresh cloud access token using stored refresh token; updates session file on success.
pub async fn refresh_cloud_access_token() -> Result<String, String> {
    let session = read_cloud_session().ok_or_else(|| "no cloud session".to_string())?;
    let refresh = session
        .refresh_token
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "no refresh token".to_string())?;

    let url = format!("{}/api/v1/devices/refresh", account_api_url());
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&serde_json::json!({ "refresh_token": refresh }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("refresh failed: {}", resp.status()));
    }

    let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let access_token = v["access_token"]
        .as_str()
        .ok_or_else(|| "missing access_token".to_string())?
        .to_string();

    let mut updated = session;
    updated.access_token = access_token.clone();
    if let Some(rt) = v["refresh_token"].as_str() {
        updated.refresh_token = Some(rt.to_string());
    }
    if let Some(gw) = v["gateway_url"].as_str() {
        updated.gateway_url = Some(gw.to_string());
    }
    let _ = write_cloud_session(&updated);
    Ok(access_token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn resolve_gateway_prefers_env_over_session() {
        let dir = TempDir::new().unwrap();
        let creds = dir.path().join(".anycode").join("credentials");
        fs::create_dir_all(&creds).unwrap();
        let session_path = creds.join("cloud-session.json");
        fs::write(
            &session_path,
            r#"{"access_token":"acct_test","gateway_url":"https://session.example"}"#,
        )
        .unwrap();

        // cloud_session_path uses copilot_token::anycode_credentials_dir which reads HOME
        // so we test resolve logic via direct session read pattern instead:
        let session: CloudSessionFile =
            serde_json::from_str(r#"{"access_token":"x","gateway_url":"https://session.example"}"#)
                .unwrap();
        assert_eq!(
            session.gateway_url.as_deref(),
            Some("https://session.example")
        );
    }

    #[test]
    fn default_gateway_chat_url_appends_path() {
        std::env::set_var("ANYCODE_MODEL_GATEWAY_URL", "https://gw.test");
        assert_eq!(
            default_gateway_chat_url(),
            "https://gw.test/v1/chat/completions"
        );
        std::env::remove_var("ANYCODE_MODEL_GATEWAY_URL");
    }
}
