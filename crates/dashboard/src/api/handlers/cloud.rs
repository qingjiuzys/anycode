use super::*;
use crate::config_patch::{self, LlmConfigPatchBody};
use anycode_llm::{
    account_api_url, capability_catalog::ModelCapability, clear_cloud_session,
    default_gateway_chat_url, migrate_legacy_llm_section, read_cloud_access_token,
    refresh_cloud_access_token, resolve_gateway_host, set_active_capability,
    sync_legacy_models_section, upsert_registry_item, ConfiguredModelFile, ResolvedModelRegistry,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct CloudLinkStartResponse {
    pub device_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_code: Option<String>,
    pub verification_uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_uri_complete: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in: Option<u64>,
    pub browser_url: String,
    pub redirect_uri: &'static str,
}

pub async fn post_cloud_link_start() -> impl IntoResponse {
    match anycode_setup::start_device_link().await {
        Ok(start) => {
            let browser_url = anycode_setup::browser_url_for_device_link(&start);
            Json(CloudLinkStartResponse {
                device_code: start.device_code,
                user_code: start.user_code,
                verification_uri: start.verification_uri,
                verification_uri_complete: start.verification_uri_complete,
                expires_in: start.expires_in,
                browser_url,
                redirect_uri: anycode_setup::DEVICE_LINK_REDIRECT_URI,
            })
            .into_response()
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct CloudLinkPollBody {
    pub device_code: String,
}

#[derive(Serialize)]
pub struct CloudLinkPollResponse {
    pub linked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub async fn post_cloud_link_poll(Json(body): Json<CloudLinkPollBody>) -> impl IntoResponse {
    match anycode_setup::try_poll_device_link(&body.device_code).await {
        Ok(Some(_)) => Json(CloudLinkPollResponse {
            linked: true,
            pending: None,
            error: None,
        })
        .into_response(),
        Ok(None) => Json(CloudLinkPollResponse {
            linked: false,
            pending: Some(true),
            error: None,
        })
        .into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(CloudLinkPollResponse {
                linked: false,
                pending: None,
                error: Some(e.to_string()),
            }),
        )
            .into_response(),
    }
}

#[derive(Serialize)]
pub struct CloudSessionResponse {
    pub linked: bool,
    pub identity_verified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub portal_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Loopback workbench only: sync into browser sessionStorage for account portal API calls.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
}

async fn cloud_me_profile(token: &str) -> Option<(bool, Option<String>, Option<String>)> {
    let url = format!("{}/api/v1/auth/me", account_api_url().trim_end_matches('/'));
    let response = reqwest::Client::new()
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let value = response.json::<serde_json::Value>().await.ok()?;
    let identity_verified = value["identity_status"].as_str() == Some("approved");
    let user = value.get("user").or(Some(&value));
    let email = user.and_then(|u| u["email"].as_str()).map(str::to_string);
    let display_name = user
        .and_then(|u| u["display_name"].as_str())
        .map(str::to_string);
    Some((identity_verified, email, display_name))
}

pub async fn get_cloud_session() -> Json<CloudSessionResponse> {
    let portal_url = Some(anycode_llm::cloud_portal_url());
    let gateway_url = Some(anycode_llm::resolve_gateway_host());
    let path = anycode_llm::cloud_session_path();
    if !path.is_file() {
        return Json(CloudSessionResponse {
            linked: false,
            identity_verified: false,
            portal_url,
            gateway_url,
            user_email: None,
            display_name: None,
            access_token: None,
        });
    }
    let token = read_cloud_access_token();
    let linked = token.is_some();
    let file_session = anycode_llm::read_cloud_session();
    let mut user_email = file_session
        .as_ref()
        .and_then(|s| s.user_email.clone())
        .filter(|s| !s.trim().is_empty());
    let mut display_name = None;
    let mut identity_verified = false;

    if let Some(token) = token.as_deref() {
        if let Some((verified, email, name)) = cloud_me_profile(token).await {
            identity_verified = verified;
            if let Some(email) = email.filter(|s| !s.trim().is_empty()) {
                user_email = Some(email);
            }
            if let Some(name) = name.filter(|s| !s.trim().is_empty()) {
                display_name = Some(name);
            }
        }
    }
    if display_name.is_none() {
        display_name = user_email.as_ref().map(|email| {
            email
                .split('@')
                .next()
                .filter(|part| !part.is_empty())
                .unwrap_or(email.as_str())
                .to_string()
        });
    }

    Json(CloudSessionResponse {
        linked,
        identity_verified,
        portal_url,
        gateway_url,
        user_email,
        display_name,
        access_token: if linked { token } else { None },
    })
}

#[derive(Serialize)]
pub struct CloudGatewayTestResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Ping the hosted model-gateway using the linked cloud session bearer token.
pub async fn post_cloud_gateway_test() -> impl IntoResponse {
    let token = match read_cloud_access_token() {
        Some(t) if !t.trim().is_empty() => t,
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(CloudGatewayTestResponse {
                    ok: false,
                    status: None,
                    gateway: Some(resolve_gateway_host()),
                    snippet: None,
                    error: Some("cloud_session_required".into()),
                }),
            )
                .into_response();
        }
    };

    let gateway = default_gateway_chat_url();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_default();
    let body = serde_json::json!({
        "model": "auto",
        "messages": [{"role": "user", "content": "ping"}],
        "max_tokens": 8,
    });

    match client
        .post(&gateway)
        .header("Authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let ok = resp.status().is_success();
            let text = resp.text().await.unwrap_or_default();
            Json(CloudGatewayTestResponse {
                ok,
                status: Some(status),
                gateway: Some(gateway),
                snippet: Some(text.chars().take(200).collect()),
                error: None,
            })
            .into_response()
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(CloudGatewayTestResponse {
                ok: false,
                status: None,
                gateway: Some(gateway),
                snippet: None,
                error: Some(e.to_string()),
            }),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct CloudCatalogModel {
    id: String,
    display_name: String,
    #[serde(default)]
    available: bool,
}

#[derive(Serialize)]
pub struct CloudSyncModelsResponse {
    pub ok: bool,
    pub synced: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Hosted named models synced from account catalog (excluding synthetic `auto`).
pub const ALLOWED_HOSTED_MODEL_IDS: &[&str] =
    &["deepseek-v4-flash", "deepseek-v4-pro", "agnes-chat"];

pub fn is_allowed_hosted_catalog_model(model_id: &str) -> bool {
    ALLOWED_HOSTED_MODEL_IDS.contains(&model_id)
}

fn cloud_model_supports_vision(model_id: &str) -> bool {
    matches!(
        model_id,
        "agnes-chat" | "gpt-4o" | "gpt-4o-mini" | "gemini-2.0-flash" | "gemini-1.5-pro"
    )
}

fn cloud_registry_item(id: &str, model: &str, display_name: &str) -> ConfiguredModelFile {
    let mut capabilities = vec![ModelCapability::Chat];
    if cloud_model_supports_vision(model) {
        capabilities.push(ModelCapability::Vision);
    }
    ConfiguredModelFile {
        id: id.to_string(),
        display_name: Some(display_name.to_string()),
        provider: "anycode_cloud".into(),
        model: model.to_string(),
        capabilities,
        api_key: None,
        api_key_ref: None,
        plan: None,
        base_url: None,
        temperature: None,
        max_tokens: None,
        extra_headers: None,
        endpoint_overrides: None,
        enabled: true,
        tags: Some(vec!["cloud".into()]),
        source: Some("cloud".into()),
    }
}

/// Fetch hosted model catalog and upsert `anycode_cloud` entries into local registry.
pub async fn post_cloud_sync_models() -> impl IntoResponse {
    let token = match read_cloud_access_token() {
        Some(t) if !t.trim().is_empty() => t,
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(CloudSyncModelsResponse {
                    ok: false,
                    synced: 0,
                    error: Some("cloud_session_required".into()),
                }),
            )
                .into_response();
        }
    };

    let catalog_url = format!(
        "{}/api/v1/models/catalog",
        account_api_url().trim_end_matches('/')
    );
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_default();

    let catalog_resp = match client.get(&catalog_url).bearer_auth(&token).send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(CloudSyncModelsResponse {
                    ok: false,
                    synced: 0,
                    error: Some(e.to_string()),
                }),
            )
                .into_response();
        }
    };
    if !catalog_resp.status().is_success() {
        let status = catalog_resp.status();
        let text = catalog_resp.text().await.unwrap_or_default();
        return (
            StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY),
            Json(CloudSyncModelsResponse {
                ok: false,
                synced: 0,
                error: Some(text),
            }),
        )
            .into_response();
    }

    let catalog: serde_json::Value = match catalog_resp.json().await {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(CloudSyncModelsResponse {
                    ok: false,
                    synced: 0,
                    error: Some(e.to_string()),
                }),
            )
                .into_response();
        }
    };
    let models: Vec<CloudCatalogModel> = catalog
        .get("models")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let (_, mut cfg) = match config_patch::read_config_value(None) {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(CloudSyncModelsResponse {
                    ok: false,
                    synced: 0,
                    error: Some(e.to_string()),
                }),
            )
                .into_response();
        }
    };
    if !cfg.is_object() {
        cfg = json!({});
    }
    migrate_legacy_llm_section(&mut cfg);
    let mut registry = ResolvedModelRegistry::from_config(&cfg);
    registry
        .items
        .retain(|item| item.source.as_deref() != Some("cloud"));

    let mut synced = 0usize;
    upsert_registry_item(
        &mut registry.items,
        cloud_registry_item("cloud-auto", "auto", "Cloud Auto"),
    );
    synced += 1;

    for m in models
        .iter()
        .filter(|m| m.available && is_allowed_hosted_catalog_model(&m.id))
    {
        let reg_id = format!("cloud-{}", m.id);
        let display = if m.id == "agnes-chat" {
            "Agnes Chat".to_string()
        } else {
            m.display_name.clone()
        };
        upsert_registry_item(
            &mut registry.items,
            cloud_registry_item(&reg_id, &m.id, &display),
        );
        synced += 1;
    }

    if registry.active.get(&ModelCapability::Chat).is_none() {
        set_active_capability(&mut registry.active, ModelCapability::Chat, "cloud-auto");
    }

    let legacy = sync_legacy_models_section(&registry);
    if let Err(e) = config_patch::patch_llm_config(&LlmConfigPatchBody {
        models: Some(legacy),
        ..Default::default()
    }) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(CloudSyncModelsResponse {
                ok: false,
                synced: 0,
                error: Some(e.to_string()),
            }),
        )
            .into_response();
    }

    Json(CloudSyncModelsResponse {
        ok: true,
        synced,
        error: None,
    })
    .into_response()
}

/// Clear cloud session and remove `source: cloud` registry entries.
pub async fn clear_linked_cloud_state() -> Result<usize, String> {
    let _ = anycode_llm::clear_cloud_session();

    let (_, mut cfg) = config_patch::read_config_value(None).map_err(|e| e.to_string())?;
    if !cfg.is_object() {
        cfg = json!({});
    }
    migrate_legacy_llm_section(&mut cfg);
    let mut registry = ResolvedModelRegistry::from_config(&cfg);
    let removed = registry
        .items
        .iter()
        .filter(|item| item.source.as_deref() == Some("cloud"))
        .count();
    registry
        .items
        .retain(|item| item.source.as_deref() != Some("cloud"));

    for cap in [
        ModelCapability::Chat,
        ModelCapability::Embedding,
        ModelCapability::Vision,
        ModelCapability::Stt,
        ModelCapability::Tts,
    ] {
        if let Some(active_id) = registry.active.get(&cap).cloned() {
            if !registry.items.iter().any(|i| i.id == active_id) {
                registry.active.remove(&cap);
            }
        }
    }

    let legacy = sync_legacy_models_section(&registry);
    config_patch::patch_llm_config(&LlmConfigPatchBody {
        models: Some(legacy),
        ..Default::default()
    })
    .map_err(|e| e.to_string())?;
    Ok(removed)
}

/// HTTP handler: unlink cloud account from local registry.
pub async fn post_cloud_unlink() -> impl IntoResponse {
    match clear_linked_cloud_state().await {
        Ok(removed) => Json(json!({ "ok": true, "removed": removed })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "ok": false, "error": e })),
        )
            .into_response(),
    }
}

/// Allow-list account-service paths for the workbench upstream proxy (SSRF guard).
fn sanitize_cloud_upstream_path(path: &str) -> Option<String> {
    let p = path.trim().trim_start_matches('/');
    if p.is_empty() || p.contains("..") || p.contains('\\') {
        return None;
    }
    if p.starts_with("api/v1/") {
        return Some(p.to_string());
    }
    None
}

/// Billing API paths that may be served by a loopback account-service with WeChat PEMs
/// when the public cloud deployment has not mounted payment secrets yet.
fn is_billing_upstream_path(path: &str) -> bool {
    path.starts_with("api/v1/billing/") && !path.starts_with("api/v1/billing/webhooks/")
}

fn local_billing_account_base() -> String {
    std::env::var("ANYCODE_LOCAL_ACCOUNT_API_URL")
        .ok()
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "http://127.0.0.1:43200".to_string())
}

async fn local_account_wechat_ready(client: &reqwest::Client) -> bool {
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    static CACHE: Mutex<Option<(Instant, bool)>> = Mutex::new(None);
    const TTL: Duration = Duration::from_secs(20);

    if let Ok(guard) = CACHE.lock() {
        if let Some((at, ok)) = *guard {
            if at.elapsed() < TTL {
                return ok;
            }
        }
    }

    let url = format!("{}/health", local_billing_account_base());
    let ok = match client
        .get(&url)
        .timeout(Duration::from_secs(2))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => resp
            .json::<serde_json::Value>()
            .await
            .ok()
            .and_then(|v| v.get("wechat_pay_configured").and_then(|b| b.as_bool()))
            .unwrap_or(false),
        _ => false,
    };

    if let Ok(mut guard) = CACHE.lock() {
        *guard = Some((Instant::now(), ok));
    }
    ok
}

async fn resolve_account_upstream_base(client: &reqwest::Client, safe_path: &str) -> String {
    if is_billing_upstream_path(safe_path) && local_account_wechat_ready(client).await {
        local_billing_account_base()
    } else {
        account_api_url().trim_end_matches('/').to_string()
    }
}

fn bearer_from_headers(headers: &axum::http::HeaderMap) -> Option<String> {
    let auth = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())?;
    let token = auth
        .strip_prefix("Bearer ")
        .or_else(|| auth.strip_prefix("bearer "))?
        .trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

fn build_cloud_upstream_request(
    client: &reqwest::Client,
    method: &axum::http::Method,
    url: &str,
    token: &str,
    content_type: Option<&str>,
    body: &[u8],
) -> Result<reqwest::RequestBuilder, StatusCode> {
    let req = match *method {
        axum::http::Method::GET => client.get(url),
        axum::http::Method::POST => client.post(url),
        axum::http::Method::PATCH => client.patch(url),
        axum::http::Method::PUT => client.put(url),
        axum::http::Method::DELETE => client.delete(url),
        _ => return Err(StatusCode::METHOD_NOT_ALLOWED),
    };
    let mut req = req
        .bearer_auth(token)
        .timeout(std::time::Duration::from_secs(30));
    if !body.is_empty()
        && matches!(
            *method,
            axum::http::Method::POST | axum::http::Method::PATCH | axum::http::Method::PUT
        )
    {
        // String keys for reqwest — axum/http 1.x HeaderName ≠ reqwest's http 0.2.
        req = req
            .header("content-type", content_type.unwrap_or("application/json"))
            .body(body.to_vec());
    }
    Ok(req)
}

async fn forward_cloud_upstream_response(resp: reqwest::Response) -> axum::response::Response {
    let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json")
        .to_string();
    let bytes = resp.bytes().await.unwrap_or_default();
    (status, [(header::CONTENT_TYPE, ct)], bytes).into_response()
}

/// Proxy account-service calls through the local Workbench to avoid browser CORS /
/// WebKit "Load failed" when the UI talks to anycode.work directly.
/// On upstream 401, refreshes the device token once and retries.
pub async fn proxy_cloud_upstream(
    Path(path): Path<String>,
    method: axum::http::Method,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let Some(safe_path) = sanitize_cloud_upstream_path(&path) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "invalid upstream path" })),
        )
            .into_response();
    };
    let mut token = match read_cloud_access_token().or_else(|| bearer_from_headers(&headers)) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "cloud access token required — link your cloud account" })),
            )
                .into_response();
        }
    };
    let client = reqwest::Client::new();
    let upstream_base = resolve_account_upstream_base(&client, &safe_path).await;
    let url = format!("{}/{}", upstream_base, safe_path);
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let req = match build_cloud_upstream_request(
        &client,
        &method,
        &url,
        &token,
        content_type.as_deref(),
        &body,
    ) {
        Ok(r) => r,
        Err(status) => {
            return (status, Json(json!({ "error": "method not allowed" }))).into_response();
        }
    };
    let first = match req.send().await {
        Ok(resp) => resp,
        Err(e) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": format!("upstream request failed: {e}") })),
            )
                .into_response();
        }
    };
    if first.status() != reqwest::StatusCode::UNAUTHORIZED {
        return forward_cloud_upstream_response(first).await;
    }
    // Access token often expires while the device refresh token is still valid.
    match refresh_cloud_access_token().await {
        Ok(new_token) => token = new_token,
        Err(e) => {
            // Stale/rotated refresh tokens leave a "linked" zombie session — clear so UI
            // can show the re-link flow instead of endless entitlement failures.
            let _ = clear_cloud_session();
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "cloud_session_expired",
                    "hint": format!("re-link your cloud account ({e})")
                })),
            )
                .into_response();
        }
    }
    let retry = match build_cloud_upstream_request(
        &client,
        &method,
        &url,
        &token,
        content_type.as_deref(),
        &body,
    ) {
        Ok(r) => r,
        Err(status) => {
            return (status, Json(json!({ "error": "method not allowed" }))).into_response();
        }
    };
    match retry.send().await {
        Ok(resp) => forward_cloud_upstream_response(resp).await,
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": format!("upstream request failed: {e}") })),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowed_catalog_only_agnes_chat() {
        assert!(is_allowed_hosted_catalog_model("agnes-chat"));
        assert!(!is_allowed_hosted_catalog_model("agnes-code"));
        assert!(!is_allowed_hosted_catalog_model("agnes-reasoner"));
    }

    #[test]
    fn cloud_registry_item_tags_cloud_source() {
        let item = cloud_registry_item("cloud-agnes-chat", "agnes-chat", "Agnes Chat");
        assert_eq!(item.source.as_deref(), Some("cloud"));
        assert_eq!(item.provider, "anycode_cloud");
        assert_eq!(item.model, "agnes-chat");
    }

    #[test]
    fn is_billing_upstream_path_excludes_webhooks() {
        assert!(is_billing_upstream_path("api/v1/billing/checkout"));
        assert!(is_billing_upstream_path("api/v1/billing/orders/ord_1"));
        assert!(!is_billing_upstream_path("api/v1/billing/webhooks/wechat"));
        assert!(!is_billing_upstream_path("api/v1/account/bundle"));
    }

    #[test]
    fn sanitize_cloud_upstream_path_allows_v1_only() {
        assert_eq!(
            sanitize_cloud_upstream_path("/api/v1/auth/me").as_deref(),
            Some("api/v1/auth/me")
        );
        assert_eq!(
            sanitize_cloud_upstream_path("api/v1/account/bundle").as_deref(),
            Some("api/v1/account/bundle")
        );
        assert_eq!(sanitize_cloud_upstream_path("../etc/passwd"), None);
        assert_eq!(sanitize_cloud_upstream_path("/health"), None);
    }
}
