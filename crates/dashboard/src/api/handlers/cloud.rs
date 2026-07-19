use super::*;
use crate::config_patch::{self, LlmConfigPatchBody};
use anycode_llm::{
    account_api_url, capability_catalog::ModelCapability, default_gateway_chat_url,
    migrate_legacy_llm_section, read_cloud_access_token, resolve_gateway_host,
    set_active_capability, sync_legacy_models_section, upsert_registry_item, ConfiguredModelFile,
    ResolvedModelRegistry,
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
}
