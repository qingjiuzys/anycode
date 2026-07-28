//! LAN-facing axum listener (port 43181) for peer handoff.

use crate::db::DashboardDb;
use crate::events::EventBus;
use crate::lan::security::{is_private_ip, peer_addr_from_headers};
use crate::lan::LanHub;
use crate::lan::{
    import_bundle, HandoffApprovedNotice, HandoffDirection, HandoffRecord, HandoffState,
    ImportOptions, IncomingHandoffRequest,
};
use crate::schema::ProjectEvent;
use anyhow::Context;
use axum::{
    body::Bytes,
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, warn};

#[derive(Clone)]
pub struct LanListenerState {
    pub hub: Arc<LanHub>,
    pub db: DashboardDb,
    pub events: Arc<EventBus>,
    pub memory_root: std::path::PathBuf,
}

pub fn spawn_lan_listener(state: LanListenerState) {
    tokio::spawn(async move {
        if let Err(e) = run_listener(state).await {
            warn!(error = %e, "LAN handoff listener stopped");
        }
    });
}

async fn run_listener(state: LanListenerState) -> anyhow::Result<()> {
    let settings = state.hub.settings_snapshot().await;
    if !settings.discovery_enabled {
        return Ok(());
    }
    let port = settings.lan_port;
    let app = lan_router(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("bind LAN handoff port {port}"))?;
    info!(port, "LAN handoff listener started");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

fn lan_router(state: LanListenerState) -> Router {
    Router::new()
        .route("/api/lan/health", get(lan_health))
        .route("/api/lan/handoff/request", post(lan_handoff_request))
        .route("/api/lan/handoff/{id}/approved", post(lan_handoff_approved))
        .route("/api/lan/handoff/{id}/upload", post(lan_handoff_upload))
        .route("/api/lan/handoff/{id}/status", get(lan_handoff_status))
        .with_state(state)
}

async fn lan_health(State(state): State<LanListenerState>) -> impl IntoResponse {
    Json(json!({
        "ok": true,
        "instance_id": state.hub.instance.instance_id,
        "device_name": state.hub.settings_snapshot().await.display_name,
        "version": state.hub.version,
    }))
}

fn reject_private(
    headers: &HeaderMap,
    connect: Option<SocketAddr>,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    let ip = peer_addr_from_headers(
        headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()),
        headers.get("x-real-ip").and_then(|v| v.to_str().ok()),
        connect,
    );
    match ip {
        Some(ip) if is_private_ip(&ip) => Ok(()),
        _ => Err((
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "remote host not allowed" })),
        )),
    }
}

async fn lan_handoff_request(
    State(state): State<LanListenerState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<IncomingHandoffRequest>,
) -> impl IntoResponse {
    if let Err(e) = reject_private(&headers, Some(addr)) {
        return e.into_response();
    }
    let mut record = HandoffRecord::new_incoming(body);
    record.expire_if_stale();
    let id = record.id.clone();
    {
        let mut map = state.hub.handoffs.write().await;
        map.insert(id.clone(), record);
    }
    state.events.publish(ProjectEvent {
        id: format!("lan-handoff-{id}"),
        project_id: String::new(),
        session_id: None,
        task_id: None,
        agent_id: None,
        event_type: "lan_handoff_requested".into(),
        severity: "info".into(),
        title: "LAN handoff request".into(),
        body: id.clone(),
        payload: json!({ "handoff_id": id }),
        occurred_at: chrono::Utc::now().to_rfc3339(),
    });
    Json(json!({ "ok": true, "handoff_id": id })).into_response()
}

async fn lan_handoff_approved(
    State(state): State<LanListenerState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<HandoffApprovedNotice>,
) -> impl IntoResponse {
    if let Err(e) = reject_private(&headers, Some(addr)) {
        return e.into_response();
    }
    let mut map = state.hub.handoffs.write().await;
    let Some(record) = map.get_mut(&id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "handoff not found" })),
        )
            .into_response();
    };
    if record.direction != HandoffDirection::Outgoing {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "not an outgoing handoff" })),
        )
            .into_response();
    }
    record.state = HandoffState::Approved;
    record.upload_token = Some(body.upload_token);
    record.target_root_path = body.target_root_path;
    record.target_project_id = body.target_project_id;
    record.updated_at = chrono::Utc::now();
    Json(json!({ "ok": true })).into_response()
}

async fn lan_handoff_upload(
    State(state): State<LanListenerState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    if let Err(e) = reject_private(&headers, Some(addr)) {
        return e.into_response();
    }
    let token = headers
        .get("x-handoff-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let mut map = state.hub.handoffs.write().await;
    let Some(record) = map.get_mut(&id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "handoff not found" })),
        )
            .into_response();
    };
    if !record.is_token_valid(token) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "invalid or expired token" })),
        )
            .into_response();
    }
    record.state = HandoffState::Importing;
    record.progress_pct = 50;
    record.updated_at = chrono::Utc::now();

    let bundle_path = state
        .hub
        .data_dir
        .join("bundles")
        .join(format!("{id}.tar.gz"));
    if let Some(parent) = bundle_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(e) = std::fs::write(&bundle_path, &body) {
        record.state = HandoffState::Failed;
        record.error = Some(e.to_string());
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response();
    }
    record.bundle_path = Some(bundle_path.display().to_string());

    let import_opts = ImportOptions {
        kind: record.kind,
        target_root_path: record.target_root_path.clone(),
        target_project_id: record.target_project_id.clone(),
    };
    let db = state.db.clone();
    let memory_root = state.memory_root.clone();
    let bundle = bundle_path.clone();
    drop(map);

    match import_bundle(&db, &memory_root, &bundle, import_opts).await {
        Ok(result) => {
            let mut map = state.hub.handoffs.write().await;
            if let Some(record) = map.get_mut(&id) {
                record.state = HandoffState::Completed;
                record.progress_pct = 100;
                record.updated_at = chrono::Utc::now();
            }
            Json(json!({
                "ok": true,
                "project_id": result.project_id,
                "root_path": result.root_path,
                "sessions_imported": result.sessions_imported,
            }))
            .into_response()
        }
        Err(e) => {
            let mut map = state.hub.handoffs.write().await;
            if let Some(record) = map.get_mut(&id) {
                record.state = HandoffState::Failed;
                record.error = Some(e.to_string());
            }
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

async fn lan_handoff_status(
    State(state): State<LanListenerState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let map = state.hub.handoffs.read().await;
    match map.get(&id) {
        Some(r) => Json(json!({
            "id": r.id,
            "state": r.state,
            "progress_pct": r.progress_pct,
            "error": r.error,
        }))
        .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "handoff not found" })),
        )
            .into_response(),
    }
}
