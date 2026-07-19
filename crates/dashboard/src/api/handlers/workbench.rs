use super::*;
use crate::workbench::{
    list_dir, read_file, shared_manager, stat_path, BrowserSessionManager,
    CreateBrowserSessionBody, PtySession, TerminalClientMessage, TerminalServerMessage,
    DEFAULT_MAX_READ_BYTES,
};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::Query;
use futures::{SinkExt, StreamExt};
use std::path::Path as StdPath;

#[derive(Deserialize)]
pub struct FsPathQuery {
    #[serde(default)]
    pub path: String,
}

#[derive(Deserialize)]
pub struct FsReadQuery {
    pub path: String,
    #[serde(default = "default_max_bytes")]
    pub max_bytes: usize,
}

fn default_max_bytes() -> usize {
    DEFAULT_MAX_READ_BYTES
}

async fn project_root_path(
    state: &AppState,
    project_id: &str,
) -> Result<String, (StatusCode, String)> {
    match state.db.get_project(project_id).await {
        Ok(Some(p)) => Ok(p.root_path),
        Ok(None) => Err((StatusCode::NOT_FOUND, "project not found".into())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn list_project_fs(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Query(q): Query<FsPathQuery>,
) -> impl IntoResponse {
    let root = match project_root_path(&state, &project_id).await {
        Ok(r) => r,
        Err(resp) => {
            return (resp.0, Json(json!({ "error": resp.1 }))).into_response();
        }
    };
    match list_dir(&root, &q.path) {
        Ok(entries) => Json(json!({ "entries": entries })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn stat_project_fs(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Query(q): Query<FsPathQuery>,
) -> impl IntoResponse {
    let root = match project_root_path(&state, &project_id).await {
        Ok(r) => r,
        Err(resp) => {
            return (resp.0, Json(json!({ "error": resp.1 }))).into_response();
        }
    };
    match stat_path(&root, &q.path) {
        Ok(stat) => Json(json!({ "stat": stat })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn read_project_fs(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Query(q): Query<FsReadQuery>,
) -> impl IntoResponse {
    let root = match project_root_path(&state, &project_id).await {
        Ok(r) => r,
        Err(resp) => {
            return (resp.0, Json(json!({ "error": resp.1 }))).into_response();
        }
    };
    match read_file(&root, &q.path, q.max_bytes) {
        Ok(body) => Json(json!({ "file": body })).into_response(),
        Err(e) => {
            let msg = e.to_string();
            let code = if msg.contains("binary") || msg.contains("UTF-8") {
                StatusCode::UNSUPPORTED_MEDIA_TYPE
            } else {
                StatusCode::BAD_REQUEST
            };
            (code, Json(json!({ "error": msg }))).into_response()
        }
    }
}

pub async fn project_terminal_ws(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let root = match project_root_path(&state, &project_id).await {
        Ok(r) => r,
        Err(resp) => {
            return (resp.0, Json(json!({ "error": resp.1 }))).into_response();
        }
    };
    ws.on_upgrade(move |socket| handle_terminal_ws(socket, root))
}

async fn handle_terminal_ws(socket: WebSocket, root_path: String) {
    let cwd = StdPath::new(&root_path);
    let (pty, mut out_rx) = match PtySession::spawn(cwd) {
        Ok(v) => v,
        Err(e) => {
            let (mut socket, _) = socket.split();
            let msg = serde_json::to_string(&TerminalServerMessage::Error {
                message: e.to_string(),
            })
            .unwrap_or_default();
            let _ = socket.send(Message::Text(msg.into())).await;
            return;
        }
    };

    let (mut ws_tx, mut ws_rx) = socket.split();

    let pty_in = pty.clone();
    let read_task = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            let text = serde_json::to_string(&msg).unwrap_or_default();
            if ws_tx.send(Message::Text(text.into())).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(msg)) = ws_rx.next().await {
        match msg {
            Message::Text(text) => {
                if let Ok(client) = serde_json::from_str::<TerminalClientMessage>(&text) {
                    match client {
                        TerminalClientMessage::Input { data } => {
                            let _ = pty_in.write_input(&data);
                        }
                        TerminalClientMessage::Resize { cols, rows } => {
                            let _ = pty_in.resize(cols, rows);
                        }
                    }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    read_task.abort();
}

pub async fn get_workbench_browser_status() -> impl IntoResponse {
    let enabled = crate::config_patch::read_config_root()
        .ok()
        .map(|(_, cfg)| crate::browser_connector::read_browser_enabled(&cfg))
        .unwrap_or(false);
    let ready = crate::browser_connector::native_chromium_ready();
    let mut status = crate::browser_connector::browser_connector_status();
    if let Some(obj) = status.as_object_mut() {
        obj.insert("enabled".into(), json!(enabled));
        obj.insert("ready".into(), json!(ready));
        obj.insert(
            "doctor_message".into(),
            json!(BrowserSessionManager::doctor_message()),
        );
    }
    Json(status).into_response()
}

pub async fn create_browser_session(
    Json(body): Json<CreateBrowserSessionBody>,
) -> impl IntoResponse {
    if let Err(msg) = BrowserSessionManager::ensure_ready() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": msg, "code": "browser_unavailable" })),
        )
            .into_response();
    }
    let mgr = shared_manager();
    match mgr
        .create(&body.project_id, body.conversation_id.as_deref())
        .await
    {
        Ok(info) => Json(json!({ "session": info })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct NavigateBrowserBody {
    pub url: String,
}

pub async fn navigate_browser_session(
    Path(session_id): Path<String>,
    Json(body): Json<NavigateBrowserBody>,
) -> impl IntoResponse {
    let mgr = shared_manager();
    match mgr.navigate(&session_id, &body.url).await {
        Ok(state) => Json(json!({ "state": state })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn browser_session_state(Path(session_id): Path<String>) -> impl IntoResponse {
    let mgr = shared_manager();
    match mgr.state(&session_id).await {
        Ok(state) => Json(json!({ "state": state })).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn browser_session_screenshot(Path(session_id): Path<String>) -> impl IntoResponse {
    let mgr = shared_manager();
    match mgr.screenshot(&session_id).await {
        Ok(shot) => Json(json!({ "screenshot": shot })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn delete_browser_session(Path(session_id): Path<String>) -> impl IntoResponse {
    let mgr = shared_manager();
    match mgr.close(&session_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct BrowserLockBody {
    pub lock: String,
}

pub async fn browser_session_lock(
    Path(session_id): Path<String>,
    Json(body): Json<BrowserLockBody>,
) -> impl IntoResponse {
    let mgr = shared_manager();
    let lock = match body.lock.as_str() {
        "user" => anycode_browser::LockHolder::User,
        "agent" => anycode_browser::LockHolder::Agent,
        _ => anycode_browser::LockHolder::Idle,
    };
    match mgr.set_lock(&session_id, lock).await {
        Ok(current) => Json(json!({ "lock": current })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn browser_session_stream(
    Path(session_id): Path<String>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| browser_stream_socket(socket, session_id))
}

async fn browser_stream_socket(mut socket: WebSocket, session_id: String) {
    let mgr = shared_manager();
    let mut rx = match mgr.subscribe_screencast(&session_id).await {
        Ok(rx) => rx,
        Err(e) => {
            let _ = socket
                .send(Message::Text(
                    json!({ "error": e.to_string() }).to_string().into(),
                ))
                .await;
            return;
        }
    };
    loop {
        tokio::select! {
            frame = rx.recv() => {
                match frame {
                    Ok(f) => {
                        let payload = json!({
                            "image_base64": f.image_base64,
                            "format": "jpeg",
                            "metadata": f.metadata,
                        });
                        if socket.send(Message::Text(payload.to_string().into())).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}
