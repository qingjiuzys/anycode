//! A2A HTTP + WebSocket handlers.

use crate::a2a::models::{
    A2aJsonRpcRequest, A2aVersionNegotiation, AgentCard, HandoffKind, HandoffState,
    DEFAULT_A2A_VERSION,
};
use crate::a2a::relay::StreamRelay;
use crate::a2a::store::{self, CreateHandoffInput};
use crate::api::{json_error, AppState, AuthContext};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use sqlx::Row;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct DeviceScopeQuery {
    pub device_id: String,
}

#[derive(Deserialize)]
pub struct HeartbeatBody {
    pub agent_card: AgentCard,
}

pub async fn a2a_heartbeat(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Json(body): Json<HeartbeatBody>,
) -> impl IntoResponse {
    match store::upsert_presence(&state.db, &ctx.user, &body.agent_card).await {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, &e.to_string()).into_response(),
    }
}

pub async fn a2a_team_peers(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    match store::list_team_peers(&state.db, &ctx.user.organization_id).await {
        Ok(peers) => Json(serde_json::json!({ "peers": peers })).into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct HandoffRequestBody {
    pub kind: HandoffKind,
    pub sender_device_id: String,
    pub sender_instance_id: String,
    pub recipient_device_id: String,
    pub recipient_instance_id: String,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    pub session_id: Option<String>,
    pub session_title: Option<String>,
    pub target_project_id: Option<String>,
}

pub async fn a2a_handoff_request(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Json(body): Json<HandoffRequestBody>,
) -> impl IntoResponse {
    match store::create_handoff(
        &state.db,
        &ctx.user,
        CreateHandoffInput {
            kind: body.kind,
            sender_device_id: body.sender_device_id,
            sender_instance_id: body.sender_instance_id,
            recipient_device_id: body.recipient_device_id,
            recipient_instance_id: body.recipient_instance_id,
            project_id: body.project_id,
            project_name: body.project_name,
            session_id: body.session_id,
            session_title: body.session_title,
            target_project_id: body.target_project_id,
        },
    )
    .await
    {
        Ok(task) => Json(serde_json::json!({ "handoff": task })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, &e.to_string()).into_response(),
    }
}

pub async fn a2a_handoff_incoming(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Query(q): Query<DeviceScopeQuery>,
) -> impl IntoResponse {
    match store::list_incoming(&state.db, &q.device_id, &ctx.user.id).await {
        Ok(items) => Json(serde_json::json!({ "incoming": items })).into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

pub async fn a2a_handoff_outgoing(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Query(q): Query<DeviceScopeQuery>,
) -> impl IntoResponse {
    match store::list_outgoing(&state.db, &q.device_id, &ctx.user.id).await {
        Ok(items) => Json(serde_json::json!({ "outgoing": items })).into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct ApproveBody {
    pub recipient_device_id: String,
    pub target_project_id: Option<String>,
}

pub async fn a2a_handoff_approve(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path(id): Path<String>,
    Json(body): Json<ApproveBody>,
) -> impl IntoResponse {
    match store::approve_handoff(
        &state.db,
        &id,
        &ctx.user,
        &body.recipient_device_id,
        body.target_project_id,
    )
    .await
    {
        Ok(task) => {
            state.a2a_relay.open_session(&id).await;
            state
                .a2a_relay
                .store_stream_token(&id, task.stream_token.clone().unwrap_or_default())
                .await;
            Json(serde_json::json!({ "handoff": task })).into_response()
        }
        Err(e) => json_error(StatusCode::BAD_REQUEST, &e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct RejectBody {
    pub recipient_device_id: String,
}

pub async fn a2a_handoff_reject(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path(id): Path<String>,
    Json(body): Json<RejectBody>,
) -> impl IntoResponse {
    match store::reject_handoff(&state.db, &id, &ctx.user, &body.recipient_device_id).await {
        Ok(task) => Json(serde_json::json!({ "handoff": task })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, &e.to_string()).into_response(),
    }
}

pub async fn a2a_handoff_status(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path(id): Path<String>,
    Query(q): Query<DeviceScopeQuery>,
) -> impl IntoResponse {
    match store::verify_device_owner(&state.db, &ctx.user.id, &q.device_id).await {
        Ok(true) => {}
        Ok(false) => {
            return json_error(StatusCode::FORBIDDEN, "device not linked to user").into_response();
        }
        Err(e) => {
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response();
        }
    }
    match store::get_handoff(&state.db, &id).await {
        Ok(mut task) => {
            let is_party = (task.sender_device_id == q.device_id
                && task.sender_user_id == ctx.user.id)
                || (task.recipient_device_id == q.device_id
                    && task.recipient_user_id == ctx.user.id);
            if !is_party {
                return json_error(StatusCode::FORBIDDEN, "not a handoff party").into_response();
            }
            if let Ok(Some(token)) = store::peek_stream_token(&state.db, &id, &q.device_id).await {
                task.stream_token = Some(token);
            } else if let Some(token) = state.a2a_relay.get_stream_token(&id).await {
                if matches!(
                    task.state,
                    HandoffState::Approved | HandoffState::Uploading | HandoffState::Importing
                ) {
                    task.stream_token = Some(token);
                }
            }
            Json(serde_json::json!({ "handoff": task })).into_response()
        }
        Err(e) => json_error(StatusCode::NOT_FOUND, &e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct StreamQuery {
    pub role: String,
    pub token: String,
}

pub async fn a2a_handoff_stream_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<StreamQuery>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_stream_socket(state, id, q.role, q.token, socket))
}

async fn handle_stream_socket(
    state: AppState,
    handoff_id: String,
    role: String,
    token: String,
    socket: WebSocket,
) {
    let relay = state.a2a_relay.clone();
    let db = state.db.clone();

    let task = match store::verify_stream_token(&db, &handoff_id, &token).await {
        Ok(t) => t,
        Err(_) => return,
    };

    if !relay.session_exists(&handoff_id).await {
        relay.open_session(&handoff_id).await;
    }

    match role.as_str() {
        "sender" => {
            let _ =
                store::update_progress(&db, &handoff_id, HandoffState::Uploading, 0, None).await;
            run_sender_socket(relay, db, handoff_id, socket).await;
        }
        "receiver" => {
            let _ = store::update_progress(
                &db,
                &handoff_id,
                HandoffState::Importing,
                task.progress_pct,
                None,
            )
            .await;
            run_receiver_socket(relay, db, handoff_id, socket).await;
        }
        _ => {}
    }
}

async fn run_sender_socket(
    relay: Arc<StreamRelay>,
    db: crate::db::AccountDb,
    handoff_id: String,
    socket: WebSocket,
) {
    let (mut sink, mut stream) = socket.split();
    let mut failed = false;

    while let Some(msg) = stream.next().await {
        match msg {
            Ok(Message::Binary(data)) => {
                match relay.publish_chunk(&handoff_id, Bytes::from(data)).await {
                    Ok(n) => {
                        let cap = crate::a2a::models::MAX_RELAY_BUFFER_BYTES.max(1);
                        let pct = (n.min(cap) * 100 / cap) as u8;
                        let _ = store::update_progress(
                            &db,
                            &handoff_id,
                            HandoffState::Uploading,
                            pct,
                            None,
                        )
                        .await;
                    }
                    Err(e) => {
                        let _ = store::update_progress(
                            &db,
                            &handoff_id,
                            HandoffState::Failed,
                            0,
                            Some(&e.to_string()),
                        )
                        .await;
                        failed = true;
                        break;
                    }
                }
            }
            Ok(Message::Close(_)) | Err(_) => break,
            _ => {}
        }
    }

    // EOF so late/slow receivers can finish draining the buffer.
    // Sender does NOT mark completed — receiver owns terminal success.
    if !failed {
        let _ = relay.publish_eof(&handoff_id).await;
        let _ = store::update_progress(&db, &handoff_id, HandoffState::Importing, 95, None).await;
    }
    let _ = sink.send(Message::Close(None)).await;
}

async fn run_receiver_socket(
    relay: Arc<StreamRelay>,
    db: crate::db::AccountDb,
    handoff_id: String,
    socket: WebSocket,
) {
    let (mut sink, mut inbound) = socket.split();
    let mut rx = match relay.subscribe(&handoff_id).await {
        Ok(r) => r,
        Err(_) => return,
    };

    let mut ok = true;
    loop {
        tokio::select! {
            chunk = rx.next() => {
                match chunk {
                    Ok(Some(data)) => {
                        if sink.send(Message::Binary(data)).await.is_err() {
                            ok = false;
                            break;
                        }
                    }
                    Ok(None) => break, // EOF
                    Err(e) => {
                        let _ = store::update_progress(
                            &db,
                            &handoff_id,
                            HandoffState::Failed,
                            0,
                            Some(&e.to_string()),
                        )
                        .await;
                        ok = false;
                        break;
                    }
                }
            }
            msg = inbound.next() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => {
                        ok = false;
                        break;
                    }
                    Some(Err(_)) => {
                        ok = false;
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    if ok {
        let _ = store::update_progress(&db, &handoff_id, HandoffState::Completed, 100, None).await;
    }
    let _ = sink.send(Message::Close(None)).await;
    relay.clear_stream_token(&handoff_id).await;
    relay.close_session(&handoff_id).await;
}

/// P2 stub: JSON-RPC tasks/send — returns not implemented in P1.
pub async fn a2a_jsonrpc_stub(Json(body): Json<A2aJsonRpcRequest>) -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": body.id,
            "error": {
                "code": -32000,
                "message": "A2A JSON-RPC available in P2; use REST /a2a/handoff/* in P1"
            }
        })),
    )
}

pub async fn a2a_version_info() -> impl IntoResponse {
    Json(A2aVersionNegotiation {
        a2a_version: DEFAULT_A2A_VERSION.into(),
        supported_versions: vec![DEFAULT_A2A_VERSION.into()],
    })
}

pub async fn a2a_agent_card(
    State(state): State<AppState>,
    Extension(_ctx): Extension<AuthContext>,
    Path(instance_id): Path<String>,
) -> impl IntoResponse {
    let row =
        sqlx::query("SELECT agent_card_json FROM a2a_agent_presence WHERE instance_id = ? LIMIT 1")
            .bind(&instance_id)
            .fetch_optional(state.db.pool())
            .await;

    match row {
        Ok(Some(r)) => {
            let json: String = r.get("agent_card_json");
            (
                StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, "application/json")],
                json,
            )
                .into_response()
        }
        _ => json_error(StatusCode::NOT_FOUND, "agent not online").into_response(),
    }
}
