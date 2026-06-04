use super::*;
use anycode_llm::{
    aggregate_catalog_view, normalize_provider_id, read_config_value, refresh_provider_catalog,
    ResolvedModelRegistry,
};
use serde::Deserialize;
use serde_json::json;

pub async fn get_model_catalog() -> impl IntoResponse {
    Json(aggregate_catalog_view()).into_response()
}

#[derive(Deserialize, Default)]
pub struct RefreshCatalogBody {
    pub provider: Option<String>,
    pub base_url: Option<String>,
}

fn catalog_api_key_for_provider(
    registry: &ResolvedModelRegistry,
    provider: &str,
) -> Option<String> {
    let norm = normalize_provider_id(provider);
    if let Some(k) = registry.provider_credentials.get(norm.as_str()) {
        if !k.trim().is_empty() {
            return Some(k.trim().to_string());
        }
    }
    for item in &registry.items {
        if normalize_provider_id(&item.provider) == norm {
            if let Some(k) = registry.resolve_api_key(item) {
                return Some(k);
            }
        }
    }
    if registry
        .global_provider
        .as_deref()
        .map(normalize_provider_id)
        .as_deref()
        == Some(norm.as_str())
        && !registry.global_api_key.trim().is_empty()
    {
        return Some(registry.global_api_key.trim().to_string());
    }
    let env = match norm.as_str() {
        "deepseek" => "DEEPSEEK_API_KEY",
        "openai" => "OPENAI_API_KEY",
        "google" | "gemini" => "GEMINI_API_KEY",
        "groq" => "GROQ_API_KEY",
        "mistral" => "MISTRAL_API_KEY",
        "xai" => "XAI_API_KEY",
        _ => return None,
    };
    std::env::var(env)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string())
}

pub async fn refresh_model_catalog(Json(body): Json<RefreshCatalogBody>) -> impl IntoResponse {
    let provider = body
        .provider
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("openai");
    let api_key = read_config_value(None)
        .ok()
        .map(|(_, cfg)| ResolvedModelRegistry::from_config(&cfg))
        .and_then(|reg| catalog_api_key_for_provider(&reg, provider));
    match refresh_provider_catalog(provider, body.base_url.as_deref(), api_key.as_deref()).await {
        Ok(catalog) => Json(json!({
            "ok": true,
            "provider": catalog.provider,
            "models": catalog.models,
            "meta": catalog.meta,
        }))
        .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": e.to_string() })),
        )
            .into_response(),
    }
}
