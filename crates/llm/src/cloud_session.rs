use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct CloudSessionFile {
    access_token: String,
    #[serde(default)]
    gateway_url: Option<String>,
}

pub fn cloud_session_path() -> PathBuf {
    crate::copilot_token::anycode_credentials_dir().join("cloud-session.json")
}

pub fn read_cloud_access_token() -> Option<String> {
    let text = std::fs::read_to_string(cloud_session_path()).ok()?;
    serde_json::from_str::<CloudSessionFile>(&text)
        .ok()
        .map(|s| s.access_token)
}

pub fn default_gateway_chat_url() -> String {
    std::env::var("ANYCODE_MODEL_GATEWAY_URL")
        .map(|u| format!("{}/v1/chat/completions", u.trim_end_matches('/')))
        .unwrap_or_else(|_| "http://127.0.0.1:43210/v1/chat/completions".into())
}
