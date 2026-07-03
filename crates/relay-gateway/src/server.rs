use crate::proxy::{chat_completions, list_models, ProxyState};
use anyhow::{Context, Result};
use axum::{
    extract::DefaultBodyLimit,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Router,
};
use bytes::Bytes;
use reqwest::Client;
use std::net::SocketAddr;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

#[derive(Debug, Clone)]
pub struct GatewayConfig {
    pub host: String,
    pub port: u16,
    pub gateway_token: Option<String>,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 43_210,
            gateway_token: std::env::var("ANYCODE_RELAY_GATEWAY_TOKEN")
                .ok()
                .filter(|s| !s.trim().is_empty()),
        }
    }
}

pub fn gateway_addr() -> SocketAddr {
    let cfg = GatewayConfig::default();
    format!("{}:{}", cfg.host, cfg.port)
        .parse()
        .expect("valid gateway addr")
}

fn build_router(state: ProxyState) -> Router {
    Router::new()
        .route("/health", get(|| async { axum::Json(serde_json::json!({ "ok": true })) }))
        .route(
            "/v1/chat/completions",
            post(
                |state: axum::extract::State<ProxyState>, headers: HeaderMap, body: Bytes| async move {
                    match chat_completions(state.0.clone(), headers, body).await {
                        Ok(resp) => resp.into_response(),
                        Err((status, msg)) => (status, msg).into_response(),
                    }
                },
            ),
        )
        .route("/v1/models", get(|| async move {
            match list_models().await {
                Ok(json) => json.into_response(),
                Err((status, msg)) => (status, msg).into_response(),
            }
        }))
        .with_state(state)
        .layer(DefaultBodyLimit::max(32 * 1024 * 1024))
}

use axum::response::IntoResponse;

pub async fn run_gateway(config: GatewayConfig) -> Result<()> {
    let state = ProxyState {
        client: Client::builder()
            .build()
            .context("build relay http client")?,
        rr: Arc::new(AtomicUsize::new(0)),
        gateway_token: config.gateway_token.clone(),
    };
    let app = build_router(state);
    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .context("parse gateway listen addr")?;
    let listener = TcpListener::bind(addr)
        .await
        .context("bind relay gateway")?;
    info!(%addr, "relay gateway listening");
    axum::serve(listener, app)
        .await
        .context("relay gateway serve")
}

/// Spawn gateway in background; returns join handle.
pub fn spawn_gateway(config: GatewayConfig) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if let Err(e) = run_gateway(config).await {
            tracing::error!(error = %e, "relay gateway exited");
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{save_relay_store, RelayAccount, RelayConfig, RelayStore};
    use reqwest::Client;
    use std::sync::atomic::AtomicUsize;

    #[tokio::test]
    async fn health_endpoint() {
        let state = ProxyState {
            client: Client::new(),
            rr: Arc::new(AtomicUsize::new(0)),
            gateway_token: None,
        };
        let app = build_router(state);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let resp = Client::new()
            .get(format!("http://{addr}/health"))
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success());
    }
}
