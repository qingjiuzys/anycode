//! Cloud A2A handoff proxy — forwards to account-service with bearer token.

use super::*;
use crate::lan::{export_bundle, import_bundle, BundleExportOptions, HandoffKind, ImportOptions};
use anycode_llm::{account_api_url, read_cloud_access_token};
use anycode_setup::read_cloud_session;
use axum::body::Body;
use axum::response::Response;
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use std::path::PathBuf;
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const CHUNK_SIZE: usize = 64 * 1024;

/// Device id for status polls. Prefer session file; empty string means the
/// account-service status handler cannot authorize stream_token disclosure.
pub(crate) fn cloud_device_id_sync(_token: &str) -> String {
    read_cloud_session()
        .and_then(|s| s.device_id)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::cloud_device_id_sync;

    #[test]
    fn status_poll_device_id_helper_is_safe_without_session() {
        // No panic when cloud-session.json is absent in test env.
        let id = cloud_device_id_sync("tok");
        assert!(id.is_empty() || id.starts_with("ldev_"));
    }
}

fn memory_root() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".anycode")
        .join("memory")
}

fn lan_data_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".anycode")
        .join("lan")
}

async fn cloud_token() -> Result<String, (StatusCode, Json<serde_json::Value>)> {
    read_cloud_access_token().ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "cloud_session_required" })),
        )
    })
}

async fn cloud_device_id(token: &str) -> Result<String, (StatusCode, Json<serde_json::Value>)> {
    if let Some(id) = read_cloud_session().and_then(|s| s.device_id) {
        return Ok(id);
    }
    let url = format!("{}/api/v1/devices", account_api_url().trim_end_matches('/'));
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .bearer_auth(token)
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": e.to_string() })),
            )
        })?;
    let v: serde_json::Value = resp.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": e.to_string() })),
        )
    })?;
    let id = v["devices"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|d| d["id"].as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "no linked device" })),
            )
        })?;
    Ok(id)
}

async fn cloud_proxy_get(
    path: &str,
    token: &str,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let url = format!("{}{}", account_api_url().trim_end_matches('/'), path);
    let resp = reqwest::Client::new()
        .get(&url)
        .bearer_auth(token)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": e.to_string() })),
            )
        })?;
    let status = resp.status();
    let body = resp.bytes().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": e.to_string() })),
        )
    })?;
    Ok(Response::builder()
        .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap())
}

async fn cloud_proxy_post(
    path: &str,
    token: &str,
    body: serde_json::Value,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let url = format!("{}{}", account_api_url().trim_end_matches('/'), path);
    let resp = reqwest::Client::new()
        .post(&url)
        .bearer_auth(token)
        .json(&body)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": e.to_string() })),
            )
        })?;
    let status = resp.status();
    let bytes = resp.bytes().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": e.to_string() })),
        )
    })?;
    Ok(Response::builder()
        .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(bytes))
        .unwrap())
}

pub async fn post_cloud_a2a_heartbeat(State(state): State<AppState>) -> impl IntoResponse {
    let token = match cloud_token().await {
        Ok(t) => t,
        Err(e) => return e.into_response(),
    };
    let device_id = match cloud_device_id(&token).await {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };
    let hub = state.lan_hub.as_ref();
    let data_dir = lan_data_dir();
    let instance_id = hub
        .map(|h| h.instance.instance_id.clone())
        .unwrap_or_else(|| crate::lan::load_or_create_instance(&data_dir).instance_id);
    let display_name = if let Some(h) = hub {
        h.settings_snapshot().await.display_name
    } else {
        crate::lan::load_or_create_instance(&data_dir).device_name
    };
    let me: serde_json::Value = match cloud_proxy_get("/api/v1/auth/me", &token).await {
        Ok(r) => {
            let bytes = axum::body::to_bytes(r.into_body(), 1024 * 1024)
                .await
                .unwrap_or_default();
            serde_json::from_slice(&bytes).unwrap_or(json!({}))
        }
        Err(e) => return e.into_response(),
    };
    let org_id = me["user"]["organization_id"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let user_id = me["user"]["id"].as_str().unwrap_or_default().to_string();
    let card = json!({
        "schema_version": "anycode_agent_card_v1",
        "instance_id": instance_id,
        "device_id": device_id,
        "organization_id": org_id,
        "user_id": user_id,
        "name": display_name,
        "transport": "cloud",
        "version": state.version,
        "capabilities": ["handoff.project", "handoff.session", "streaming.relay"],
    });
    cloud_proxy_post(
        "/api/v1/a2a/presence/heartbeat",
        &token,
        json!({ "agent_card": card }),
    )
    .await
    .map(|r| r.into_response())
    .unwrap_or_else(|e| e.into_response())
}

pub async fn get_cloud_a2a_team_peers() -> impl IntoResponse {
    let token = match cloud_token().await {
        Ok(t) => t,
        Err(e) => return e.into_response(),
    };
    cloud_proxy_get("/api/v1/a2a/team/peers", &token)
        .await
        .map(|r| r.into_response())
        .unwrap_or_else(|e| e.into_response())
}

#[derive(Deserialize)]
pub struct CloudHandoffRequestBody {
    pub recipient_device_id: String,
    pub recipient_instance_id: String,
    pub kind: HandoffKind,
    pub project_id: Option<String>,
    pub session_id: Option<String>,
    pub target_project_id: Option<String>,
}

pub async fn post_cloud_a2a_handoff_request(
    State(state): State<AppState>,
    Json(body): Json<CloudHandoffRequestBody>,
) -> impl IntoResponse {
    let token = match cloud_token().await {
        Ok(t) => t,
        Err(e) => return e.into_response(),
    };
    let sender_device_id = match cloud_device_id(&token).await {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };
    let data_dir = lan_data_dir();
    let instance_id = state
        .lan_hub
        .as_ref()
        .map(|h| h.instance.instance_id.clone())
        .unwrap_or_else(|| crate::lan::load_or_create_instance(&data_dir).instance_id);

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

    let payload = json!({
        "kind": body.kind,
        "sender_device_id": sender_device_id,
        "sender_instance_id": instance_id,
        "recipient_device_id": body.recipient_device_id,
        "recipient_instance_id": body.recipient_instance_id,
        "project_id": project_id,
        "project_name": project.name,
        "session_id": body.session_id,
        "session_title": session_title,
        "target_project_id": body.target_project_id,
    });

    let resp = match cloud_proxy_post("/api/v1/a2a/handoff/request", &token, payload).await {
        Ok(r) => r,
        Err(e) => return e.into_response(),
    };
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap_or_default();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or(json!({}));
    let handoff_id = v["handoff"]["id"].as_str().unwrap_or_default().to_string();
    if !handoff_id.is_empty() {
        let st = state.clone();
        tokio::spawn(run_cloud_outgoing_upload(
            st,
            handoff_id.clone(),
            body.kind,
            project_id,
            body.session_id.clone(),
        ));
    }
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(bytes))
        .unwrap()
        .into_response()
}

pub async fn get_cloud_a2a_handoff_incoming() -> impl IntoResponse {
    let token = match cloud_token().await {
        Ok(t) => t,
        Err(e) => return e.into_response(),
    };
    let device_id = match cloud_device_id(&token).await {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };
    cloud_proxy_get(
        &format!("/api/v1/a2a/handoff/incoming?device_id={device_id}"),
        &token,
    )
    .await
    .map(|r| r.into_response())
    .unwrap_or_else(|e| e.into_response())
}

pub async fn get_cloud_a2a_handoff_outgoing() -> impl IntoResponse {
    let token = match cloud_token().await {
        Ok(t) => t,
        Err(e) => return e.into_response(),
    };
    let device_id = match cloud_device_id(&token).await {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };
    cloud_proxy_get(
        &format!("/api/v1/a2a/handoff/outgoing?device_id={device_id}"),
        &token,
    )
    .await
    .map(|r| r.into_response())
    .unwrap_or_else(|e| e.into_response())
}

#[derive(Deserialize)]
pub struct CloudApproveBody {
    pub target_project_id: Option<String>,
}

pub async fn post_cloud_a2a_handoff_approve(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<CloudApproveBody>,
) -> impl IntoResponse {
    let token = match cloud_token().await {
        Ok(t) => t,
        Err(e) => return e.into_response(),
    };
    let device_id = match cloud_device_id(&token).await {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };
    let resp = match cloud_proxy_post(
        &format!("/api/v1/a2a/handoff/{id}/approve"),
        &token,
        json!({
            "recipient_device_id": device_id,
            "target_project_id": body.target_project_id,
        }),
    )
    .await
    {
        Ok(r) => r,
        Err(e) => return e.into_response(),
    };
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap_or_default();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or(json!({}));
    if let (Some(hid), Some(stream_token)) = (
        v["handoff"]["id"].as_str(),
        v["handoff"]["stream_token"].as_str(),
    ) {
        let kind = match v["handoff"]["kind"].as_str() {
            Some("session") => HandoffKind::Session,
            _ => HandoffKind::Project,
        };
        let st = state.clone();
        let target = body.target_project_id.clone();
        tokio::spawn(run_cloud_incoming_receive(
            st,
            hid.to_string(),
            stream_token.to_string(),
            kind,
            target,
        ));
    }
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(bytes))
        .unwrap()
        .into_response()
}

pub async fn post_cloud_a2a_handoff_reject(Path(id): Path<String>) -> impl IntoResponse {
    let token = match cloud_token().await {
        Ok(t) => t,
        Err(e) => return e.into_response(),
    };
    let device_id = match cloud_device_id(&token).await {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };
    cloud_proxy_post(
        &format!("/api/v1/a2a/handoff/{id}/reject"),
        &token,
        json!({ "recipient_device_id": device_id }),
    )
    .await
    .map(|r| r.into_response())
    .unwrap_or_else(|e| e.into_response())
}

pub fn spawn_cloud_a2a_heartbeat(state: AppState) {
    tokio::spawn(async move {
        loop {
            let _ = post_cloud_a2a_heartbeat(State(state.clone())).await;
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    });
}

async fn run_cloud_outgoing_upload(
    state: AppState,
    handoff_id: String,
    kind: HandoffKind,
    project_id: String,
    session_id: Option<String>,
) {
    let token = match read_cloud_access_token() {
        Some(t) => t,
        None => return,
    };
    for _ in 0..120 {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let url = format!(
            "{}/api/v1/a2a/handoff/{handoff_id}/status?device_id={}",
            account_api_url().trim_end_matches('/'),
            cloud_device_id_sync(&token)
        );
        let Ok(resp) = reqwest::Client::new()
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
        else {
            continue;
        };
        let Ok(v) = resp.json::<serde_json::Value>().await else {
            continue;
        };
        let state_str = v["handoff"]["state"].as_str().unwrap_or("");
        if matches!(state_str, "rejected" | "failed" | "expired" | "completed") {
            break;
        }
        // Accept approved or uploading — stream_token may appear after approve
        // while a concurrent poll already moved state to uploading.
        if !matches!(state_str, "approved" | "uploading") {
            continue;
        }
        let stream_token = match v["handoff"]["stream_token"].as_str() {
            Some(t) if !t.is_empty() => t.to_string(),
            _ => continue,
        };
        let data_dir = lan_data_dir();
        let instance_id = state
            .lan_hub
            .as_ref()
            .map(|h| h.instance.instance_id.clone())
            .unwrap_or_else(|| crate::lan::load_or_create_instance(&data_dir).instance_id);
        let display_name = if let Some(h) = state.lan_hub.as_ref() {
            h.settings_snapshot().await.display_name.clone()
        } else {
            crate::lan::load_or_create_instance(&data_dir).device_name
        };
        let bundle_path = match export_bundle(
            &state.db,
            &memory_root(),
            BundleExportOptions {
                kind,
                project_id: project_id.clone(),
                session_id: session_id.clone(),
                source_instance_id: instance_id,
                source_device_name: display_name,
                // Must stay ≤ account-service MAX_RELAY_BUFFER_BYTES (64 MiB).
                max_bytes: 64 * 1024 * 1024,
            },
        )
        .await
        {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(error = %e, "cloud handoff export failed");
                break;
            }
        };
        let ws_base = account_api_url()
            .trim_end_matches('/')
            .replace("https://", "wss://")
            .replace("http://", "ws://");
        let ws_url = format!(
            "{ws_base}/api/v1/a2a/handoff/{handoff_id}/stream?role=sender&token={stream_token}"
        );
        let Ok((mut ws, _)) = connect_async(&ws_url).await else {
            break;
        };
        let data = match tokio::fs::read(&bundle_path).await {
            Ok(d) => d,
            Err(_) => break,
        };
        for chunk in data.chunks(CHUNK_SIZE) {
            if ws.send(Message::Binary(chunk.to_vec())).await.is_err() {
                break;
            }
        }
        let _ = ws.close(None).await;
        let _ = tokio::fs::remove_file(&bundle_path).await;
        break;
    }
}

async fn run_cloud_incoming_receive(
    state: AppState,
    handoff_id: String,
    stream_token: String,
    kind: HandoffKind,
    target_project_id: Option<String>,
) {
    let ws_base = account_api_url()
        .trim_end_matches('/')
        .replace("https://", "wss://")
        .replace("http://", "ws://");
    let ws_url = format!(
        "{ws_base}/api/v1/a2a/handoff/{handoff_id}/stream?role=receiver&token={stream_token}"
    );
    let Ok((mut ws, _)) = connect_async(&ws_url).await else {
        return;
    };
    let tmp = std::env::temp_dir().join(format!("anycode-cloud-handoff-{handoff_id}.tar.gz"));
    let mut file = match tokio::fs::File::create(&tmp).await {
        Ok(f) => f,
        Err(_) => return,
    };
    while let Some(msg) = ws.next().await {
        match msg {
            Ok(Message::Binary(data)) => {
                if tokio::io::AsyncWriteExt::write_all(&mut file, &data)
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Ok(Message::Close(_)) | Err(_) => break,
            _ => {}
        }
    }
    drop(file);
    let _ = import_bundle(
        &state.db,
        &memory_root(),
        &tmp,
        ImportOptions {
            kind,
            target_project_id,
            target_root_path: None,
        },
    )
    .await;
    let _ = tokio::fs::remove_file(&tmp).await;
}
