//! GitHub → Copilot token 交换与缓存（对齐 OpenClaw `github-copilot-token.ts`）。

use anycode_core::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const COPILOT_TOKEN_URL: &str = "https://api.github.com/copilot_internal/v2/token";
pub const DEFAULT_COPILOT_API_BASE_URL: &str = "https://api.individual.githubcopilot.com";

const COPILOT_EDITOR_VERSION: &str = "vscode/1.96.2";
const COPILOT_USER_AGENT: &str = "GitHubCopilotChat/0.26.7";
const COPILOT_GITHUB_API_VERSION: &str = "2025-04-01";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedCopilotToken {
    pub token: String,
    pub expires_at: i64,
    #[serde(default)]
    pub updated_at: i64,
}

#[derive(Debug, Deserialize)]
struct CopilotTokenApiBody {
    token: String,
    expires_at: serde_json::Value,
}

pub fn anycode_credentials_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".anycode")
        .join("credentials")
}

pub fn copilot_token_cache_path() -> PathBuf {
    anycode_credentials_dir().join("github-copilot.token.json")
}

pub fn github_oauth_token_path() -> PathBuf {
    anycode_credentials_dir().join("github-oauth.json")
}

#[derive(Debug, Deserialize)]
struct OAuthFile {
    access_token: String,
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn token_usable(cache: &CachedCopilotToken) -> bool {
    cache.expires_at - now_ms() > 5 * 60 * 1000
}

fn parse_expires_at(v: &serde_json::Value) -> Result<i64, CoreError> {
    let n = match v {
        serde_json::Value::Number(x) => x.as_i64().or_else(|| x.as_f64().map(|f| f as i64)),
        serde_json::Value::String(s) => s.parse::<i64>().ok(),
        _ => None,
    }
    .ok_or_else(|| CoreError::LLMError("copilot token: invalid expires_at".to_string()))?;
    Ok(if n < 100_000_000_000 {
        n * 1000
    } else {
        n
    })
}

/// 从 Copilot JWT 的 `proxy-ep=` 推导 API base（与 OpenClaw `deriveCopilotApiBaseUrlFromToken` 一致）。
pub fn derive_copilot_api_base_url_from_token(token: &str) -> Option<String> {
    let trimmed = token.trim();
    for part in trimmed.split(';') {
        let p = part.trim();
        let Some(rest) = p.strip_prefix("proxy-ep=") else {
            continue;
        };
        let rest = rest.trim();
        if rest.is_empty() {
            continue;
        }
        let url_text = if rest.starts_with("http://") || rest.starts_with("https://") {
            rest.to_string()
        } else {
            format!("https://{}", rest)
        };
        let host = url::Url::parse(&url_text).ok()?.host_str()?.to_lowercase();
        let api_host = if let Some(stripped) = host.strip_prefix("proxy.") {
            format!("api.{}", stripped)
        } else {
            host
        };
        return Some(format!("https://{}", api_host));
    }
    None
}

fn copilot_ide_headers_for_github_api() -> reqwest::header::HeaderMap {
    let mut m = reqwest::header::HeaderMap::new();
    let _ = m.insert(
        "Editor-Version",
        COPILOT_EDITOR_VERSION.parse().expect("header"),
    );
    let _ = m.insert(
        "User-Agent",
        COPILOT_USER_AGENT.parse().expect("header"),
    );
    let _ = m.insert(
        "X-Github-Api-Version",
        COPILOT_GITHUB_API_VERSION.parse().expect("header"),
    );
    m
}

/// 读取设备码登录写入的 GitHub OAuth token（`github-oauth.json`）。
pub fn read_github_oauth_access_token() -> Option<String> {
    let path = github_oauth_token_path();
    let data = std::fs::read(&path).ok()?;
    let v: OAuthFile = serde_json::from_slice(&data).ok()?;
    let t = v.access_token.trim();
    if t.is_empty() {
        None
    } else {
        Some(v.access_token)
    }
}

/// 用 GitHub token 换取 Copilot API token，带磁盘缓存。
pub async fn resolve_copilot_api_token(github_token: &str) -> Result<(String, String), CoreError> {
    let cache_path = copilot_token_cache_path();
    if let Ok(bytes) = tokio::fs::read(&cache_path).await {
        if let Ok(cached) = serde_json::from_slice::<CachedCopilotToken>(&bytes) {
            if token_usable(&cached) {
                let base = derive_copilot_api_base_url_from_token(&cached.token)
                    .unwrap_or_else(|| DEFAULT_COPILOT_API_BASE_URL.to_string());
                return Ok((cached.token, base));
            }
        }
    }

    let client = reqwest::Client::new();
    let mut headers = copilot_ide_headers_for_github_api();
    let _ = headers.insert(
        reqwest::header::ACCEPT,
        "application/json".parse().expect("accept"),
    );
    let _ = headers.insert(
        reqwest::header::AUTHORIZATION,
        format!("Bearer {}", github_token.trim())
            .parse()
            .map_err(|e| CoreError::LLMError(format!("auth header: {}", e)))?,
    );

    let res = client
        .get(COPILOT_TOKEN_URL)
        .headers(headers)
        .send()
        .await
        .map_err(|e| CoreError::LLMError(e.to_string()))?;

    if !res.status().is_success() {
        let txt = res.text().await.unwrap_or_default();
        return Err(CoreError::LLMError(format!(
            "Copilot token exchange HTTP error: {}",
            txt
        )));
    }

    let body: CopilotTokenApiBody = res
        .json()
        .await
        .map_err(|e| CoreError::LLMError(e.to_string()))?;

    let expires_at = parse_expires_at(&body.expires_at)?;
    let payload = CachedCopilotToken {
        token: body.token.clone(),
        expires_at,
        updated_at: now_ms(),
    };

    if let Some(parent) = cache_path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    if let Ok(json) = serde_json::to_vec_pretty(&payload) {
        let _ = tokio::fs::write(&cache_path, json).await;
    }

    let base = derive_copilot_api_base_url_from_token(&body.token)
        .unwrap_or_else(|| DEFAULT_COPILOT_API_BASE_URL.to_string());
    Ok((body.token, base))
}
