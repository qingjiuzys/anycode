use super::*;
use crate::workbench::{
    list_dir, read_file, read_raw_file, shared_manager, stat_path, terminal_shared_manager,
    BrowserSessionManager, CreateBrowserSessionBody, TerminalClientMessage, TerminalServerMessage,
    DEFAULT_MAX_RAW_BYTES, DEFAULT_MAX_READ_BYTES,
};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::Query;
use axum::http::{header, HeaderMap, HeaderValue};
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

#[derive(Deserialize)]
pub struct FsRawQuery {
    pub path: String,
    #[serde(default = "default_max_raw_bytes")]
    pub max_bytes: u64,
}

fn default_max_raw_bytes() -> u64 {
    DEFAULT_MAX_RAW_BYTES
}

/// Binary file preview/download for deliverable cards (images, PDF, video).
pub async fn raw_project_fs(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Query(q): Query<FsRawQuery>,
) -> impl IntoResponse {
    let root = match project_root_path(&state, &project_id).await {
        Ok(r) => r,
        Err(resp) => {
            return (resp.0, Json(json!({ "error": resp.1 }))).into_response();
        }
    };
    match read_raw_file(&root, &q.path, q.max_bytes) {
        Ok((bytes, mime, _rel)) => {
            let mut headers = HeaderMap::new();
            if let Ok(val) = HeaderValue::from_str(&mime) {
                headers.insert(header::CONTENT_TYPE, val);
            }
            headers.insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("private, max-age=60"),
            );
            (StatusCode::OK, headers, bytes).into_response()
        }
        Err(e) => {
            let msg = e.to_string();
            let code = if msg.contains("too large") {
                StatusCode::PAYLOAD_TOO_LARGE
            } else if msg.contains("not found") || msg.contains("No such") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::BAD_REQUEST
            };
            (code, Json(json!({ "error": msg }))).into_response()
        }
    }
}

/// Create a terminal tab inside a conversation's group.
#[derive(Deserialize)]
pub struct CreateTerminalSessionBody {
    pub project_id: String,
    pub conversation_id: String,
}

pub async fn create_terminal_session(
    State(state): State<AppState>,
    Json(body): Json<CreateTerminalSessionBody>,
) -> impl IntoResponse {
    let root = match project_root_path(&state, &body.project_id).await {
        Ok(r) => r,
        Err(resp) => {
            return (resp.0, Json(json!({ "error": resp.1 }))).into_response();
        }
    };
    let cwd = StdPath::new(&root);
    let mgr = terminal_shared_manager();
    match mgr.create(&body.project_id, &body.conversation_id, cwd) {
        Ok((session, _rx)) => Json(json!({ "session": session.info() })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct ListTerminalSessionsQuery {
    pub project_id: String,
    pub conversation_id: String,
}

pub async fn list_terminal_sessions(
    Query(q): Query<ListTerminalSessionsQuery>,
) -> impl IntoResponse {
    let mgr = terminal_shared_manager();
    let sessions = mgr.list(&q.project_id, &q.conversation_id);
    Json(json!({ "sessions": sessions })).into_response()
}

pub async fn delete_terminal_session(Path(session_id): Path<String>) -> impl IntoResponse {
    let mgr = terminal_shared_manager();
    if mgr.close(&session_id) {
        StatusCode::NO_CONTENT.into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "terminal session not found" })),
        )
            .into_response()
    }
}

/// Attach a WebSocket to an existing live terminal session. Multiple sockets
/// may attach concurrently; disconnecting one does not kill the shell.
pub async fn terminal_session_ws(
    Path(session_id): Path<String>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_terminal_ws(socket, session_id))
}

async fn handle_terminal_ws(socket: WebSocket, session_id: String) {
    let mgr = terminal_shared_manager();
    let pty = match mgr.get(&session_id) {
        Some(p) => p,
        None => {
            let (mut socket, _) = socket.split();
            let msg = serde_json::to_string(&TerminalServerMessage::Error {
                message: "terminal session not found".into(),
            })
            .unwrap_or_default();
            let _ = socket.send(Message::Text(msg.into())).await;
            return;
        }
    };

    let (mut ws_tx, mut ws_rx) = socket.split();
    let mut out_rx = pty.subscribe();
    let pty_in = pty.clone();

    // Replay recent output (e.g. the shell prompt) to a newly attached
    // subscriber; a fresh `broadcast` receiver otherwise misses anything
    // produced before it subscribed.
    for msg in pty.replay_history() {
        let text = serde_json::to_string(&msg).unwrap_or_default();
        if ws_tx.send(Message::Text(text.into())).await.is_err() {
            return;
        }
    }

    let read_task = tokio::spawn(async move {
        loop {
            match out_rx.recv().await {
                Ok(msg) => {
                    let text = serde_json::to_string(&msg).unwrap_or_default();
                    if ws_tx.send(Message::Text(text.into())).await.is_err() {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
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

pub async fn get_project_git_status(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    let root = match project_root_path(&state, &project_id).await {
        Ok(r) => r,
        Err(resp) => {
            return (resp.0, Json(json!({ "error": resp.1 }))).into_response();
        }
    };
    match crate::workbench::git_status(StdPath::new(&root)) {
        Ok(status) => Json(json!({ "git": status })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct GitCommitBody {
    #[serde(default)]
    pub message: Option<String>,
}

pub async fn get_project_git_changes(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    let root = match project_root_path(&state, &project_id).await {
        Ok(r) => r,
        Err(resp) => {
            return (resp.0, Json(json!({ "error": resp.1 }))).into_response();
        }
    };
    match crate::workbench::git_changes(StdPath::new(&root)) {
        Ok(changes) => Json(json!({ "changes": changes })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_project_git_file_diff(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Query(query): Query<GitFileDiffQuery>,
) -> impl IntoResponse {
    let root = match project_root_path(&state, &project_id).await {
        Ok(r) => r,
        Err(resp) => {
            return (resp.0, Json(json!({ "error": resp.1 }))).into_response();
        }
    };
    let path = query.path.as_deref().unwrap_or("");
    if path.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "missing path" })),
        )
            .into_response();
    }
    let kind = match query.kind.as_deref() {
        Some("added") => crate::workbench::GitChangeKind::Added,
        Some("deleted") => crate::workbench::GitChangeKind::Deleted,
        Some("renamed") => crate::workbench::GitChangeKind::Renamed,
        Some("type_changed") => crate::workbench::GitChangeKind::TypeChanged,
        Some("untracked") => crate::workbench::GitChangeKind::Untracked,
        _ => crate::workbench::GitChangeKind::Modified,
    };
    match crate::workbench::git_file_diff(StdPath::new(&root), path, kind) {
        Ok(diff) => Json(json!({ "diff": diff })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct GitFileDiffQuery {
    pub path: Option<String>,
    pub kind: Option<String>,
}

pub async fn post_project_git_commit(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(body): Json<GitCommitBody>,
) -> impl IntoResponse {
    let root = match project_root_path(&state, &project_id).await {
        Ok(r) => r,
        Err(resp) => {
            return (resp.0, Json(json!({ "error": resp.1 }))).into_response();
        }
    };
    let root_path = StdPath::new(&root);
    if !crate::workbench::is_git_repo(root_path) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "not a git repository" })),
        )
            .into_response();
    }
    let message = body
        .message
        .filter(|m| !m.trim().is_empty())
        .unwrap_or_else(|| "Update from anyCode".to_string());
    match crate::workbench::git_commit_all(root_path, &message) {
        Ok(()) => Json(json!({ "ok": true })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn post_project_git_push(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    let root = match project_root_path(&state, &project_id).await {
        Ok(r) => r,
        Err(resp) => {
            return (resp.0, Json(json!({ "error": resp.1 }))).into_response();
        }
    };
    match crate::workbench::git_push(StdPath::new(&root)) {
        Ok(detail) => Json(json!({ "ok": true, "detail": detail })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
