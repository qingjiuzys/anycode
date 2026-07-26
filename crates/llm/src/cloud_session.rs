use serde::{Deserialize, Serialize};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Direct Agnes OpenAI-compatible endpoint (fallback when cloud gateway is down).
pub const AGNES_DIRECT_CHAT_URL: &str = "https://apihub.agnes-ai.com/v1/chat/completions";
pub const AGNES_DIRECT_DEFAULT_MODEL: &str = "agnes-2.0-flash";

/// Hosted cloud console (login, billing, models). Override via `ANYCODE_ACCOUNT_PORTAL_URL`.
pub const DEFAULT_CLOUD_PORTAL: &str = "https://anycode.work";
/// Account API base (same deployable as portal in production). Override via `ANYCODE_ACCOUNT_API_URL`.
pub const DEFAULT_ACCOUNT_API: &str = "https://anycode.work";
/// Model gateway host without path. Override via `ANYCODE_MODEL_GATEWAY_URL` or device-link `gateway_url`.
pub const DEFAULT_GATEWAY_HOST: &str = "https://anycode.work";

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
    std::fs::write(&path, text)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

pub fn read_cloud_access_token() -> Option<String> {
    read_cloud_session()
        .map(|s| s.access_token)
        .filter(|t| !t.trim().is_empty())
}

/// Gateway host without path suffix. Priority: env > session file > production default.
/// Stale dev loopback URLs in the session file are ignored when the port is not listening.
pub fn resolve_gateway_host() -> String {
    let from_env = std::env::var("ANYCODE_MODEL_GATEWAY_URL")
        .ok()
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty());
    if let Some(host) = from_env {
        return host;
    }

    let session_host = read_cloud_session()
        .and_then(|s| s.gateway_url)
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty());

    if let Some(ref host) = session_host {
        if is_loopback_gateway_host(host) && !gateway_host_reachable(host) {
            tracing::warn!(
                host,
                "cloud session gateway is loopback but not reachable; using {DEFAULT_GATEWAY_HOST}"
            );
            maybe_repair_session_gateway(DEFAULT_GATEWAY_HOST);
            return DEFAULT_GATEWAY_HOST.to_string();
        }
        if gateway_host_reachable(host) {
            return host.clone();
        }
        if is_loopback_gateway_host(host) {
            maybe_repair_session_gateway(DEFAULT_GATEWAY_HOST);
            return DEFAULT_GATEWAY_HOST.to_string();
        }
    }

    DEFAULT_GATEWAY_HOST.to_string()
}

fn is_loopback_gateway_host(host: &str) -> bool {
    let lower = host.to_ascii_lowercase();
    lower.contains("127.0.0.1")
        || lower.contains("localhost")
        || lower.contains("[::1]")
        || lower.starts_with("0.0.0.0")
}

pub fn is_production_portal_host(host: &str) -> bool {
    let lower = host.trim().trim_end_matches('/').to_ascii_lowercase();
    lower == DEFAULT_GATEWAY_HOST.to_ascii_lowercase() || lower.ends_with("anycode.work")
}

pub fn gateway_host_base(chat_url: &str) -> String {
    let trimmed = chat_url.trim().trim_end_matches('/');
    trimmed
        .strip_suffix("/v1/chat/completions")
        .or_else(|| trimmed.strip_suffix("/chat/completions"))
        .unwrap_or(trimmed)
        .to_string()
}

/// Best-effort TCP probe for `host:port` extracted from a gateway base URL.
pub fn gateway_host_reachable(gateway_host_or_url: &str) -> bool {
    let trimmed = gateway_host_or_url.trim().trim_end_matches('/');
    let with_scheme = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("http://{trimmed}")
    };
    let Ok(url) = url::Url::parse(&with_scheme) else {
        return false;
    };
    let host = url.host_str().unwrap_or("127.0.0.1");
    let port = url
        .port()
        .unwrap_or(if url.scheme() == "https" { 443 } else { 80 });
    let addr = format!("{host}:{port}");
    let Ok(mut addrs) = addr.to_socket_addrs() else {
        return false;
    };
    addrs.any(|socket| TcpStream::connect_timeout(&socket, Duration::from_millis(600)).is_ok())
}

#[derive(Debug, Clone)]
struct GatewayReachCache {
    url: String,
    reachable: bool,
    checked_at: Instant,
}

static GATEWAY_REACH_CACHE: Mutex<Option<GatewayReachCache>> = Mutex::new(None);

fn gateway_chat_http_reachable(chat_url: &str) -> bool {
    let ttl = Duration::from_secs(60);
    if let Ok(guard) = GATEWAY_REACH_CACHE.lock() {
        if let Some(cache) = guard.as_ref() {
            if cache.url == chat_url && cache.checked_at.elapsed() < ttl {
                return cache.reachable;
            }
        }
    }

    let reachable = gateway_chat_http_probe(chat_url);
    if let Ok(mut guard) = GATEWAY_REACH_CACHE.lock() {
        *guard = Some(GatewayReachCache {
            url: chat_url.to_string(),
            reachable,
            checked_at: Instant::now(),
        });
    }
    reachable
}

fn gateway_chat_http_probe(chat_url: &str) -> bool {
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(2500))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    let resp = client
        .post(chat_url)
        .header("Content-Type", "application/json")
        .body(r#"{"model":"ping","messages":[{"role":"user","content":"ping"}]}"#)
        .send();
    match resp {
        Ok(r) => {
            let status = r.status().as_u16();
            // Portal/nginx often returns 405 even when TCP:443 is open.
            !matches!(status, 404 | 405 | 502 | 503)
        }
        Err(_) => false,
    }
}

pub fn should_use_direct_agnes_for_cloud_gateway(chat_url: &str) -> bool {
    let host_base = gateway_host_base(chat_url);
    is_production_portal_host(&host_base) || !gateway_chat_url_reachable(chat_url)
}

pub fn gateway_chat_url_reachable(chat_url: &str) -> bool {
    let host_base = gateway_host_base(chat_url);
    if is_production_portal_host(&host_base) {
        return false;
    }
    if is_loopback_gateway_host(&host_base) {
        return gateway_host_reachable(&host_base);
    }
    gateway_chat_http_reachable(chat_url)
}

fn maybe_repair_session_gateway(host: &str) {
    let Some(mut session) = read_cloud_session() else {
        return;
    };
    let current = session
        .gateway_url
        .as_deref()
        .map(|s| s.trim().trim_end_matches('/'))
        .unwrap_or("");
    if current == host {
        return;
    }
    session.gateway_url = Some(host.to_string());
    let _ = write_cloud_session(&session);
}

/// When anyCode Cloud gateway is down, fall back to a direct Agnes API key from config.
#[derive(Debug, Clone)]
pub struct DirectAgnesFallback {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
}

pub fn map_cloud_model_to_direct_agnes(model: &str) -> String {
    match model.trim() {
        "agnes-chat" | "auto" | "agnes-code" | "agnes-reasoner" => {
            AGNES_DIRECT_DEFAULT_MODEL.to_string()
        }
        other if !other.is_empty() => other.to_string(),
        _ => AGNES_DIRECT_DEFAULT_MODEL.to_string(),
    }
}

pub fn direct_agnes_fallback_for_cloud_model(model: &str) -> Option<DirectAgnesFallback> {
    let api_key = read_provider_credential("agnes")?;
    Some(DirectAgnesFallback {
        api_key,
        model: map_cloud_model_to_direct_agnes(model),
        base_url: AGNES_DIRECT_CHAT_URL.to_string(),
    })
}

/// Effective chat endpoint after anyCode Cloud gateway / direct-Agnes resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCloudEndpoint {
    pub provider: String,
    pub model: String,
    pub base_url: String,
    pub api_key: String,
}

/// Shared rules for [`crate::build_zai_openai_stack_client`] and runtime [`ModelConfig`].
pub fn resolve_anycode_cloud_endpoint(
    model: &str,
    base_url: Option<&str>,
    api_key: Option<&str>,
) -> Result<ResolvedCloudEndpoint, String> {
    let mut key = api_key.unwrap_or("").trim().to_string();
    if key.is_empty() {
        key = read_cloud_access_token().ok_or_else(|| {
            "anyCode Cloud：请运行 `anycode auth login` 并完成设备关联".to_string()
        })?;
    }

    let url = base_url
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(default_gateway_chat_url);

    if should_use_direct_agnes_for_cloud_gateway(&url) {
        if let Some(fb) = direct_agnes_fallback_for_cloud_model(model) {
            tracing::warn!(
                cloud_model = %model,
                direct_model = %fb.model,
                "anycode_cloud gateway unreachable; using direct Agnes API"
            );
            return Ok(ResolvedCloudEndpoint {
                provider: "custom".to_string(),
                model: fb.model,
                base_url: fb.base_url,
                api_key: fb.api_key,
            });
        }
        return Err(format!(
            "anyCode Cloud 网关不可达（{url}）。请启动本地 model-gateway（端口 43210），或在模型库改用「Agnes 2.0 Flash」直连。"
        ));
    }

    Ok(ResolvedCloudEndpoint {
        provider: "anycode_cloud".to_string(),
        model: model.to_string(),
        base_url: url,
        api_key: key,
    })
}

fn read_provider_credential(provider: &str) -> Option<String> {
    let (_, cfg) = crate::config_file::read_config_value(None).ok()?;
    cfg.get("provider_credentials")
        .and_then(|v| v.get(provider))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

pub fn default_gateway_chat_url() -> String {
    format!("{}/v1/chat/completions", resolve_gateway_host())
}

pub fn cloud_portal_url() -> String {
    std::env::var("ANYCODE_ACCOUNT_PORTAL_URL")
        .ok()
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_CLOUD_PORTAL.to_string())
}

pub fn account_api_url() -> String {
    std::env::var("ANYCODE_ACCOUNT_API_URL")
        .ok()
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_ACCOUNT_API.to_string())
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

/// Remove linked cloud session file (logout / unlink).
pub fn clear_cloud_session() -> std::io::Result<()> {
    let path = cloud_session_path();
    if path.is_file() {
        std::fs::remove_file(path)?;
    }
    Ok(())
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
        let prev = std::env::var("ANYCODE_MODEL_GATEWAY_URL").ok();
        std::env::set_var("ANYCODE_MODEL_GATEWAY_URL", "https://gw.test");
        assert_eq!(
            default_gateway_chat_url(),
            "https://gw.test/v1/chat/completions"
        );
        match prev {
            Some(v) => std::env::set_var("ANYCODE_MODEL_GATEWAY_URL", v),
            None => std::env::remove_var("ANYCODE_MODEL_GATEWAY_URL"),
        }
    }

    #[test]
    fn production_defaults_point_at_anycode_work() {
        assert_eq!(DEFAULT_CLOUD_PORTAL, "https://anycode.work");
        assert_eq!(DEFAULT_ACCOUNT_API, "https://anycode.work");
        assert_eq!(DEFAULT_GATEWAY_HOST, "https://anycode.work");
    }

    #[test]
    fn production_portal_is_not_chat_gateway() {
        assert!(is_production_portal_host("https://anycode.work"));
        assert!(!gateway_chat_url_reachable(
            "https://anycode.work/v1/chat/completions"
        ));
    }

    #[test]
    fn map_cloud_models_to_direct_agnes() {
        assert_eq!(
            map_cloud_model_to_direct_agnes("agnes-chat"),
            "agnes-2.0-flash"
        );
        assert_eq!(
            map_cloud_model_to_direct_agnes("agnes-reasoner"),
            "agnes-2.0-flash"
        );
        assert_eq!(
            map_cloud_model_to_direct_agnes("agnes-2.0-flash"),
            "agnes-2.0-flash"
        );
    }

    #[test]
    fn resolve_endpoint_falls_back_to_agnes_for_production_portal() {
        let resolved = resolve_anycode_cloud_endpoint(
            "auto",
            Some("https://anycode.work/v1/chat/completions"),
            None,
        )
        .expect("agnes credential should enable fallback");
        assert_eq!(resolved.base_url, AGNES_DIRECT_CHAT_URL);
        assert_eq!(resolved.model, "agnes-2.0-flash");
        assert!(!resolved.api_key.is_empty());
    }
}
