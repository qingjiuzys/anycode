//! Loopback LAN handoff API (UI-facing).

use crate::api::state::AppState;
use crate::audit::{record_audit, AuditEventInput};
use crate::lan::{
    export_bundle, primary_lan_ip, save_instance, BundleExportOptions, HandoffApprovedNotice,
    HandoffDirection, HandoffKind, HandoffParty, HandoffRecord, HandoffState,
    IncomingHandoffRequest, LanSettings, OutgoingHandoffStatus,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

#[derive(Deserialize)]
pub struct HandoffRequestBody {
    pub peer_id: String,
    pub kind: HandoffKind,
    pub project_id: Option<String>,
    pub session_id: Option<String>,
    pub target_project_id: Option<String>,
}

#[derive(Deserialize)]
pub struct ApproveHandoffBody {
    pub target_root_path: Option<String>,
    pub target_project_id: Option<String>,
}

#[derive(Deserialize)]
pub struct LanSettingsPatch {
    pub discovery_enabled: Option<bool>,
    pub display_name: Option<String>,
    pub lan_port: Option<u16>,
    pub max_bundle_mb: Option<u64>,
}

fn memory_root() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".anycode")
        .join("memory")
}

pub async fn get_lan_peers(State(state): State<AppState>) -> impl IntoResponse {
    let Some(hub) = &state.lan_hub else {
        return Json(json!({ "peers": [], "enabled": false })).into_response();
    };
    hub.refresh_dev_peers();
    Json(json!({
        "peers": hub.list_peers(),
        "enabled": hub.settings_snapshot().await.discovery_enabled,
        "instance_id": hub.instance.instance_id,
        "display_name": hub.settings_snapshot().await.display_name,
    }))
    .into_response()
}

pub async fn get_lan_settings(State(state): State<AppState>) -> impl IntoResponse {
    let Some(hub) = &state.lan_hub else {
        return Json(json!({ "enabled": false })).into_response();
    };
    Json(json!({ "settings": hub.settings_snapshot().await })).into_response()
}

pub async fn patch_lan_settings(
    State(state): State<AppState>,
    Json(body): Json<LanSettingsPatch>,
) -> impl IntoResponse {
    let Some(hub) = &state.lan_hub else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "LAN hub not available" })),
        )
            .into_response();
    };
    let mut settings = hub.settings_snapshot().await;
    if let Some(v) = body.discovery_enabled {
        settings.discovery_enabled = v;
    }
    if let Some(v) = body.display_name.filter(|s| !s.trim().is_empty()) {
        settings.display_name = v;
    }
    if let Some(v) = body.lan_port {
        settings.lan_port = v;
    }
    if let Some(v) = body.max_bundle_mb {
        settings.max_bundle_mb = v;
    }
    if let Err(e) = settings.save(&hub.data_dir) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response();
    }
    {
        let mut hub_settings = hub.settings.write().await;
        *hub_settings = settings.clone();
    }
    let mut inst = hub.instance.clone();
    inst.device_name = settings.display_name.clone();
    let _ = save_instance(&hub.data_dir, &inst);
    Json(json!({ "ok": true, "settings": settings })).into_response()
}

pub async fn post_lan_handoff_request(
    State(state): State<AppState>,
    Json(body): Json<HandoffRequestBody>,
) -> impl IntoResponse {
    let Some(hub) = state.lan_hub.clone() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "LAN hub not available" })),
        )
            .into_response();
    };
    let settings = hub.settings_snapshot().await;
    let peer = hub
        .list_peers()
        .into_iter()
        .find(|p| p.instance_id == body.peer_id);
    let Some(peer) = peer else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "peer not found" })),
        )
            .into_response();
    };

    let project_id = match body.project_id.as_deref() {
        Some(id) => id.to_string(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "project_id required" })),
            )
                .into_response();
        }
    };

    let project = match state.db.get_project(&project_id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "project not found" })),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };

    let session_title = if let Some(sid) = body.session_id.as_deref() {
        state
            .db
            .get_session(sid)
            .await
            .ok()
            .flatten()
            .map(|s| s.title)
    } else {
        None
    };

    let sender = local_party(&hub, &settings).await;
    let recipient = HandoffParty {
        instance_id: peer.instance_id.clone(),
        device_name: peer.device_name.clone(),
        host: peer.host.clone(),
        lan_port: peer.lan_port,
    };

    let mut record = HandoffRecord::new_outgoing(
        body.kind,
        sender.clone(),
        recipient.clone(),
        Some(project_id.clone()),
        Some(project.name.clone()),
        body.session_id.clone(),
        session_title,
    );
    record.target_project_id = body.target_project_id.clone();

    let incoming = IncomingHandoffRequest {
        id: record.id.clone(),
        kind: body.kind,
        sender,
        recipient,
        project_id: Some(project_id.clone()),
        project_name: Some(project.name.clone()),
        session_id: body.session_id.clone(),
        session_title: record.session_title.clone(),
    };

    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap_or_default();
    let url = format!("{}/api/lan/handoff/request", peer.base_url());
    if let Err(e) = client.post(&url).json(&incoming).send().await {
        return (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": format!("failed to reach peer: {e}") })),
        )
            .into_response();
    }

    let id = record.id.clone();
    hub.handoffs.write().await.insert(id.clone(), record);

    let _ = record_audit(
        &state.db,
        AuditEventInput::low(
            "lan_handoff_requested",
            json!({ "handoff_id": id, "peer_id": body.peer_id, "kind": body.kind }),
        ),
    )
    .await;

    tokio::spawn(run_outgoing_upload(
        state.clone(),
        hub,
        id.clone(),
        body.kind,
        project_id.clone(),
        body.session_id.clone(),
    ));

    Json(json!({ "ok": true, "handoff_id": id })).into_response()
}

async fn run_outgoing_upload(
    state: AppState,
    hub: Arc<crate::lan::LanHub>,
    handoff_id: String,
    kind: HandoffKind,
    project_id: String,
    session_id: Option<String>,
) {
    let settings = hub.settings_snapshot().await;
    let max_bytes = settings.max_bundle_mb * 1024 * 1024;
    for _ in 0..120 {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let record = hub.handoffs.read().await.get(&handoff_id).cloned();
        let Some(record) = record else { break };
        if record.state == HandoffState::Rejected || record.state == HandoffState::Failed {
            break;
        }
        if record.state != HandoffState::Approved {
            continue;
        }
        let token = record.upload_token.clone().unwrap_or_default();
        let peer_url = format!(
            "http://{}:{}/api/lan/handoff/{}/upload",
            record.recipient.host, record.recipient.lan_port, handoff_id
        );
        let bundle = match export_bundle(
            &state.db,
            &memory_root(),
            BundleExportOptions {
                kind,
                project_id: project_id.clone(),
                session_id: session_id.clone(),
                source_instance_id: hub.instance.instance_id.clone(),
                source_device_name: settings.display_name.clone(),
                max_bytes,
            },
        )
        .await
        {
            Ok(p) => p,
            Err(e) => {
                let mut map = hub.handoffs.write().await;
                if let Some(r) = map.get_mut(&handoff_id) {
                    r.state = HandoffState::Failed;
                    r.error = Some(e.to_string());
                }
                break;
            }
        };
        let bytes = match std::fs::read(&bundle) {
            Ok(b) => b,
            Err(e) => {
                let mut map = hub.handoffs.write().await;
                if let Some(r) = map.get_mut(&handoff_id) {
                    r.state = HandoffState::Failed;
                    r.error = Some(e.to_string());
                }
                break;
            }
        };
        {
            let mut map = hub.handoffs.write().await;
            if let Some(r) = map.get_mut(&handoff_id) {
                r.state = HandoffState::Uploading;
                r.progress_pct = 30;
            }
        }
        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()
            .unwrap_or_default();
        let resp = client
            .post(&peer_url)
            .header("x-handoff-token", &token)
            .header("content-type", "application/octet-stream")
            .body(bytes)
            .send()
            .await;
        let _ = std::fs::remove_file(&bundle);
        match resp {
            Ok(r) if r.status().is_success() => {
                let mut map = hub.handoffs.write().await;
                if let Some(rec) = map.get_mut(&handoff_id) {
                    rec.state = HandoffState::Completed;
                    rec.progress_pct = 100;
                }
                let _ = record_audit(
                    &state.db,
                    AuditEventInput::low(
                        "lan_handoff_completed",
                        json!({ "handoff_id": handoff_id }),
                    ),
                )
                .await;
            }
            Ok(r) => {
                let err = r.text().await.unwrap_or_default();
                let mut map = hub.handoffs.write().await;
                if let Some(rec) = map.get_mut(&handoff_id) {
                    rec.state = HandoffState::Failed;
                    rec.error = Some(err);
                }
            }
            Err(e) => {
                let mut map = hub.handoffs.write().await;
                if let Some(rec) = map.get_mut(&handoff_id) {
                    rec.state = HandoffState::Failed;
                    rec.error = Some(e.to_string());
                }
            }
        }
        break;
    }
}

async fn local_party(hub: &crate::lan::LanHub, settings: &LanSettings) -> HandoffParty {
    let host = primary_lan_ip()
        .map(|ip| ip.to_string())
        .unwrap_or_else(|| "127.0.0.1".into());
    HandoffParty {
        instance_id: hub.instance.instance_id.clone(),
        device_name: settings.display_name.clone(),
        host,
        lan_port: settings.lan_port,
    }
}

pub async fn get_lan_handoff_incoming(State(state): State<AppState>) -> impl IntoResponse {
    let Some(hub) = &state.lan_hub else {
        return Json(json!({ "requests": [] })).into_response();
    };
    let mut map = hub.handoffs.write().await;
    for r in map.values_mut() {
        r.expire_if_stale();
    }
    let requests: Vec<_> = map
        .values()
        .filter(|r| {
            r.direction == HandoffDirection::Incoming && r.state == HandoffState::PendingApproval
        })
        .cloned()
        .collect();
    Json(json!({ "requests": requests })).into_response()
}

pub async fn get_lan_handoff_outgoing(State(state): State<AppState>) -> impl IntoResponse {
    let Some(hub) = &state.lan_hub else {
        return Json(json!({ "requests": [] })).into_response();
    };
    let requests: Vec<OutgoingHandoffStatus> = hub
        .handoffs
        .read()
        .await
        .values()
        .filter(|r| r.direction == HandoffDirection::Outgoing)
        .map(OutgoingHandoffStatus::from)
        .collect();
    Json(json!({ "requests": requests })).into_response()
}

pub async fn post_lan_handoff_approve(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<ApproveHandoffBody>,
) -> impl IntoResponse {
    let Some(hub) = state.lan_hub.clone() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "LAN hub not available" })),
        )
            .into_response();
    };
    let notice = {
        let mut map = hub.handoffs.write().await;
        let Some(record) = map.get_mut(&id) else {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "handoff not found" })),
            )
                .into_response();
        };
        if record.direction != HandoffDirection::Incoming {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "not an incoming handoff" })),
            )
                .into_response();
        }
        record.approve(
            body.target_root_path.clone(),
            body.target_project_id.clone(),
        );
        HandoffApprovedNotice {
            id: id.clone(),
            upload_token: record.upload_token.clone().unwrap_or_default(),
            target_root_path: body.target_root_path,
            target_project_id: body.target_project_id,
        }
    };

    let sender_url = {
        let map = hub.handoffs.read().await;
        map.get(&id).map(|r| {
            format!(
                "http://{}:{}/api/lan/handoff/{}/approved",
                r.sender.host, r.sender.lan_port, id
            )
        })
    };
    if let Some(url) = sender_url {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        let _ = client.post(&url).json(&notice).send().await;
    }

    let _ = record_audit(
        &state.db,
        AuditEventInput::low("lan_handoff_approved", json!({ "handoff_id": id })),
    )
    .await;

    Json(json!({ "ok": true })).into_response()
}

pub async fn post_lan_handoff_reject(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let Some(hub) = &state.lan_hub else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "LAN hub not available" })),
        )
            .into_response();
    };
    let mut map = hub.handoffs.write().await;
    let Some(record) = map.get_mut(&id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "handoff not found" })),
        )
            .into_response();
    };
    record.reject();
    let _ = record_audit(
        &state.db,
        AuditEventInput::low("lan_handoff_rejected", json!({ "handoff_id": id })),
    )
    .await;
    Json(json!({ "ok": true })).into_response()
}
