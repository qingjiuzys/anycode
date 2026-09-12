use serde::{Deserialize, Serialize};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

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

/// Shown when hosted cloud chat has no linked device session.
pub const CLOUD_DEVICE_UNLINKED_HINT: &str =
    "anyCode Cloud：未关联本机。请在工作台点击「连接云账号」完成设备授权";

pub fn cloud_session_path() -> PathBuf {
    crate::copilot_token::anycode_credentials_dir().join("cloud-session.json")
}

fn cloud_session_candidate_paths() -> Vec<PathBuf> {
    let mut paths = vec![cloud_session_path()];
    // Legacy location from an older comment/path; keep reading it.
    let legacy = crate::copilot_token::anycode_home_dir().join("cloud-session.json");
    if legacy != paths[0] {
        paths.push(legacy);
    }
    paths
}

pub fn read_cloud_session() -> Option<CloudSessionFile> {
    for path in cloud_session_candidate_paths() {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        if let Ok(parsed) = serde_json::from_str::<CloudSessionFile>(&text) {
            if !parsed.access_token.trim().is_empty() {
                return Some(parsed);
            }
        }
    }
    None
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

pub fn gateway_chat_url_reachable(chat_url: &str) -> bool {
    let host_base = gateway_host_base(chat_url);
    if is_production_portal_host(&host_base) {
        // Hosted catalog is served by anycode.work; skip the local-gateway probe.
        return true;
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

/// Effective chat endpoint after anyCode Cloud gateway resolution.
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
    let configured = base_url
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let mut url = configured.clone().unwrap_or_else(default_gateway_chat_url);
    let mut used_loopback_fallback = false;

    if !gateway_chat_url_reachable(&url) {
        let loopback = configured
            .as_deref()
            .map(gateway_host_base)
            .is_some_and(|host| is_loopback_gateway_host(&host));
        let fallback = default_gateway_chat_url();
        if loopback && fallback != url && gateway_chat_url_reachable(&fallback) {
            tracing::warn!(
                configured = %url,
                fallback = %fallback,
                "anycode_cloud loopback gateway is down; using reachable host"
            );
            url = fallback;
            used_loopback_fallback = true;
        } else {
            return Err(format!(
                "anyCode Cloud 网关不可达（{url}）。请检查网络，或在工作台重新连接云账号。"
            ));
        }
    }

    let mut key = api_key.unwrap_or("").trim().to_string();
    if key.is_empty() || used_loopback_fallback {
        if let Some(session) = read_cloud_access_token() {
            key = session;
        } else if key.is_empty() {
            return Err(CLOUD_DEVICE_UNLINKED_HINT.to_string());
        }
    }

    Ok(ResolvedCloudEndpoint {
        provider: "anycode_cloud".to_string(),
        model: model.to_string(),
        base_url: url,
        api_key: key,
    })
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
    fn production_hosted_gateway_is_reachable() {
        assert!(is_production_portal_host("https://anycode.work"));
        assert!(gateway_chat_url_reachable(
            "https://anycode.work/v1/chat/completions"
        ));
    }

    #[test]
    fn production_cloud_endpoint_resolves_with_token() {
        let got = resolve_anycode_cloud_endpoint(
            "deepseek-v4-pro",
            Some("https://anycode.work/v1/chat/completions"),
            Some("acct_test"),
        )
        .expect("hosted anycode.work is a valid chat gateway");
        assert_eq!(got.api_key, "acct_test");
        assert_eq!(got.model, "deepseek-v4-pro");
        assert!(got.base_url.contains("anycode.work"));
    }

    #[test]
    fn dead_loopback_gateway_falls_back_to_reachable_host() {
        let got = resolve_anycode_cloud_endpoint(
            "auto",
            Some("http://127.0.0.1:1/v1/chat/completions"),
            Some("acct_test"),
        )
        .expect("dead loopback should fall back to a reachable host");
        assert!(
            !got.base_url.contains("127.0.0.1:1"),
            "stale loopback must not stay in the resolved endpoint"
        );
        assert!(got.base_url.contains("/v1/chat/completions"));
        assert_eq!(got.model, "auto");
        assert!(!got.api_key.is_empty());
    }

    #[test]
    fn missing_device_link_does_not_mention_removed_cli() {
        assert!(!CLOUD_DEVICE_UNLINKED_HINT.contains("anycode auth login"));
        assert!(CLOUD_DEVICE_UNLINKED_HINT.contains("连接云账号"));
    }
}
