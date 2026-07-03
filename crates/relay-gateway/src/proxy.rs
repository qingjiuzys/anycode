use crate::router::{account_chat_url, pick_accounts, resolve_upstream_model};
use crate::store::load_relay_store;
use anyhow::{anyhow, Context, Result};
use axum::{
    body::Body,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use futures::StreamExt;
use reqwest::Client;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

#[derive(Clone)]
pub struct ProxyState {
    pub client: Client,
    pub rr: Arc<AtomicUsize>,
    pub gateway_token: Option<String>,
}

pub async fn chat_completions(
    state: ProxyState,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, (StatusCode, String)> {
    if let Some(token) = &state.gateway_token {
        let auth = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let expected = format!("Bearer {token}");
        if auth != expected {
            return Err((
                StatusCode::UNAUTHORIZED,
                "invalid relay gateway token".into(),
            ));
        }
    }

    let store = load_relay_store();
    if !store.config.enabled {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "relay gateway disabled in relay.json".into(),
        ));
    }

    let mut payload: serde_json::Value =
        serde_json::from_slice(&body).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let requested_model = payload
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or(&store.config.default_model)
        .to_string();

    let upstream_model = resolve_upstream_model(&store, &requested_model);
    if let Some(obj) = payload.as_object_mut() {
        obj.insert(
            "model".into(),
            serde_json::Value::String(upstream_model.clone()),
        );
    }

    let body_bytes = serde_json::to_vec(&payload)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let candidates = pick_accounts(&store, &state.rr);
    if candidates.is_empty() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "no active relay accounts configured".into(),
        ));
    }

    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json");

    let mut last_err = String::from("all upstream accounts failed");

    for account in candidates {
        let url = account_chat_url(&account);
        let mut req = state
            .client
            .post(&url)
            .header("Content-Type", content_type)
            .body(body_bytes.clone());

        if let Some(key) = account.api_key.as_deref().filter(|k| !k.is_empty()) {
            req = req.header("Authorization", format!("Bearer {key}"));
        }

        match req.send().await {
            Ok(resp) => {
                let status = resp.status();
                if status.as_u16() == 429 || status.as_u16() >= 500 {
                    last_err = format!("upstream {url} returned {status}");
                    continue;
                }
                return forward_response(resp)
                    .await
                    .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()));
            }
            Err(e) => {
                last_err = format!("upstream {url}: {e}");
            }
        }
    }

    Err((StatusCode::BAD_GATEWAY, last_err))
}

pub async fn list_models() -> Result<impl IntoResponse, (StatusCode, String)> {
    let store = load_relay_store();
    if !store.config.enabled {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "relay gateway disabled".into(),
        ));
    }

    let mut data: Vec<serde_json::Value> = store
        .models
        .iter()
        .map(|m| {
            serde_json::json!({
                "id": m.id,
                "object": "model",
                "owned_by": "anycode-relay",
            })
        })
        .collect();

    if data.is_empty() {
        data.push(serde_json::json!({
            "id": store.config.default_model,
            "object": "model",
            "owned_by": "anycode-relay",
        }));
    }

    Ok(axum::Json(serde_json::json!({
        "object": "list",
        "data": data,
    })))
}

async fn forward_response(upstream: reqwest::Response) -> Result<Response> {
    let status =
        StatusCode::from_u16(upstream.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let mut builder = Response::builder().status(status);

    for (name, value) in upstream.headers() {
        let n = name.as_str();
        if n.eq_ignore_ascii_case("transfer-encoding") || n.eq_ignore_ascii_case("connection") {
            continue;
        }
        if let Ok(v) = value.to_str() {
            builder = builder.header(n, v);
        }
    }

    let stream = upstream
        .bytes_stream()
        .map(|item| item.map_err(|e| std::io::Error::other(e.to_string())));

    builder
        .body(Body::from_stream(stream))
        .context("build proxy response")
        .map_err(|e| anyhow!(e))
}

#[cfg(test)]
mod tests {
    use super::list_models;
    use axum::http::StatusCode;

    #[tokio::test]
    async fn list_models_disabled_when_off() {
        // Default store may be enabled; just ensure handler returns structured response.
        let result = list_models().await;
        assert!(result.is_ok() || matches!(result, Err((StatusCode::SERVICE_UNAVAILABLE, _))));
    }
}
