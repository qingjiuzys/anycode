//! Model registry API: configured models, enable, test, patch.

use super::*;
use crate::llm_probe::LlmProbeService;
use anycode_llm::{
    capability_catalog::ModelCapability, patch_llm_config_value, read_config_value,
    set_active_capability, upsert_registry_item, write_config_value, ConfiguredModelFile,
    LlmConfigPatch, MaskedSecret, ResolvedModelRegistry,
};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

fn mask_item(item: &ConfiguredModelFile) -> Value {
    json!({
        "id": item.id,
        "display_name": item.display_name,
        "provider": item.provider,
        "model": item.model,
        "capabilities": item.capabilities.iter().map(|c| c.as_str()).collect::<Vec<_>>(),
        "plan": item.plan,
        "base_url": item.base_url,
        "api_key": MaskedSecret::from_value(item.api_key.as_deref()),
        "api_key_ref": item.api_key_ref,
        "temperature": item.temperature,
        "max_tokens": item.max_tokens,
        "extra_headers": item.extra_headers.as_ref().map(|h| h.keys().collect::<Vec<_>>()),
        "endpoint_overrides": item.endpoint_overrides,
        "enabled": item.enabled,
        "tags": item.tags,
        "source": item.source,
    })
}

pub async fn get_models_registry() -> impl IntoResponse {
    let (_, cfg) = match read_config_value(None) {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };
    let view = anycode_llm::RegistryView::from_config(&cfg);
    Json(json!({
        "config_present": view.config_present,
        "active": view.active,
        "items": view.items,
        "routing": cfg.get("routing").cloned().unwrap_or(json!({})),
        "model_fallback": view.model_fallback,
        "global": {
            "provider": view.provider,
            "model": view.model,
        }
    }))
    .into_response()
}

#[derive(Deserialize)]
pub struct PutModelsBody {
    #[serde(default)]
    pub items: Option<Vec<ConfiguredModelFile>>,
    #[serde(default)]
    pub active: Option<HashMap<String, String>>,
    #[serde(default)]
    pub delete_ids: Option<Vec<String>>,
}

pub async fn put_models_registry(
    State(state): State<AppState>,
    Json(body): Json<PutModelsBody>,
) -> impl IntoResponse {
    let (path, mut cfg) = match read_config_value(None) {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };
    if !cfg.is_object() {
        cfg = json!({});
    }
    anycode_llm::migrate_legacy_llm_section(&mut cfg);

    let mut registry = ResolvedModelRegistry::from_config(&cfg);
    if let Some(delete) = body.delete_ids.as_ref() {
        for id in delete {
            anycode_llm::remove_registry_item(&mut registry.items, id);
            registry.active.retain(|_, v| v != id);
        }
    }
    if let Some(items) = body.items.as_ref() {
        for item in items {
            upsert_registry_item(&mut registry.items, item.clone());
        }
    }
    if let Some(active) = body.active.as_ref() {
        for (cap, id) in active {
            if let Some(c) = ModelCapability::parse(cap) {
                if id.trim().is_empty() {
                    registry.active.remove(&c);
                } else {
                    set_active_capability(&mut registry.active, c, id.trim());
                }
            }
        }
    }

    let legacy = anycode_llm::sync_legacy_models_section(&registry);
    let patch = LlmConfigPatch {
        models: Some(legacy),
        ..Default::default()
    };
    if let Err(e) = patch_llm_config_value(&mut cfg, &patch) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response();
    }
    if let Err(e) = write_config_value(&path, &cfg) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response();
    }

    let _ = crate::audit::record_audit(
        &state.db,
        crate::audit::AuditEventInput {
            project_id: None,
            session_id: None,
            action: "config_models_updated".into(),
            risk: "medium".into(),
            detail: json!({ "config_path": path.display().to_string() }),
        },
    )
    .await;

    Json(json!({ "ok": true, "config_path": path.display().to_string() })).into_response()
}

#[derive(Deserialize)]
pub struct EnableModelBody {
    pub capabilities: Vec<String>,
}

pub async fn enable_model(
    State(state): State<AppState>,
    axum::extract::Path(model_id): axum::extract::Path<String>,
    Json(body): Json<EnableModelBody>,
) -> impl IntoResponse {
    let caps: Vec<ModelCapability> = body
        .capabilities
        .iter()
        .filter_map(|c| ModelCapability::parse(c))
        .collect();
    if caps.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "capabilities required" })),
        )
            .into_response();
    }
    let mut active = HashMap::new();
    for c in caps {
        active.insert(c.as_str().to_string(), model_id.clone());
    }
    put_models_registry(
        State(state),
        Json(PutModelsBody {
            items: None,
            active: Some(active),
            delete_ids: None,
        }),
    )
    .await
    .into_response()
}

#[derive(Deserialize, Default)]
pub struct TestModelBody {
    #[serde(default)]
    pub capability: Option<String>,
    #[serde(default)]
    pub draft: Option<ConfiguredModelFile>,
}

pub async fn test_model(
    axum::extract::Path(model_id): axum::extract::Path<String>,
    Json(body): Json<TestModelBody>,
) -> impl IntoResponse {
    let (_, cfg) = match read_config_value(None) {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };

    let cap = body
        .capability
        .as_deref()
        .and_then(ModelCapability::parse)
        .or_else(|| {
            body.draft
                .as_ref()
                .and_then(|d| d.capabilities.first().copied())
        })
        .unwrap_or(ModelCapability::Chat);

    let mut registry = ResolvedModelRegistry::from_config(&cfg);
    if let Some(draft) = body.draft.as_ref() {
        upsert_registry_item(&mut registry.items, draft.clone());
        set_active_capability(&mut registry.active, cap, &draft.id);
    } else {
        if registry.item_by_id(&model_id).is_none() {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "model not found" })),
            )
                .into_response();
        }
        set_active_capability(&mut registry.active, cap, &model_id);
    }

    match LlmProbeService::from_registry(registry).probe(cap).await {
        Ok(msg) => Json(json!({ "ok": true, "message": msg })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": e })),
        )
            .into_response(),
    }
}
