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

#[derive(Serialize)]
pub struct CloudSessionResponse {
    pub linked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub portal_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway_url: Option<String>,
}

pub async fn get_cloud_session() -> Json<CloudSessionResponse> {
    let portal_url = Some(anycode_llm::cloud_portal_url());
    let gateway_url = Some(anycode_llm::resolve_gateway_host());
    let path = anycode_llm::cloud_session_path();
    if !path.is_file() {
        return Json(CloudSessionResponse {
            linked: false,
            access_token: None,
            portal_url,
            gateway_url,
        });
    }
    let token = read_cloud_access_token();
    Json(CloudSessionResponse {
        linked: token.is_some(),
        access_token: token,
        portal_url,
        gateway_url,
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

fn cloud_registry_item(id: &str, model: &str, display_name: &str) -> ConfiguredModelFile {
    ConfiguredModelFile {
        id: id.to_string(),
        display_name: Some(display_name.to_string()),
        provider: "anycode_cloud".into(),
        model: model.to_string(),
        capabilities: vec![ModelCapability::Chat],
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

    for m in models.iter().filter(|m| m.available) {
        let reg_id = format!("cloud-{}", m.id);
        upsert_registry_item(
            &mut registry.items,
            cloud_registry_item(&reg_id, &m.id, &m.display_name),
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
