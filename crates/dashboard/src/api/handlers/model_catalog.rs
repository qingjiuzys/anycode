use super::*;
use anycode_llm::{aggregate_catalog_view, refresh_provider_catalog};
use axum::Json;
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

pub async fn refresh_model_catalog(Json(body): Json<RefreshCatalogBody>) -> impl IntoResponse {
    let provider = body
        .provider
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("openai");
    match refresh_provider_catalog(provider, body.base_url.as_deref()).await {
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
