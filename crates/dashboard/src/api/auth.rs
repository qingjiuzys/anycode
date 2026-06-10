use crate::api::state::AppState;
use crate::auth_session::{self, SESSION_COOKIE};
use crate::service_governance::is_loopback_host;
use axum::{
    body::Body,
    extract::State,
    http::{header, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

fn cookie_value(parts: &axum::http::request::Parts, name: &str) -> Option<String> {
    parts
        .headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|part| {
                let (k, v) = part.trim().split_once('=')?;
                if k == name {
                    Some(v.to_string())
                } else {
                    None
                }
            })
        })
}

pub fn cookie_from_request(parts: &axum::http::request::Parts) -> Option<String> {
    cookie_value(parts, SESSION_COOKIE)
}

pub async fn resolve_request_user(
    state: &AppState,
    parts: &axum::http::request::Parts,
) -> Option<crate::auth_session::AuthUser> {
    if is_loopback_host(&state.host) {
        return auth_session::local_trusted_user(&state.db).await.ok();
    }
    if let Some(token) = cookie_value(parts, SESSION_COOKIE) {
        if let Some(uid) = state.sessions.resolve(&token) {
            return auth_session::get_user_by_id(&state.db, &uid)
                .await
                .ok()
                .flatten();
        }
    }
    let auth = parts
        .headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if crate::tokens::validate_token(&state.db, auth)
        .await
        .unwrap_or(false)
    {
        return auth_session::local_trusted_user(&state.db).await.ok();
    }
    None
}

pub async fn auth_middleware(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    if is_loopback_host(&state.host) {
        return next.run(req).await;
    }
    let path = req.uri().path();
    if is_public_path(path) {
        return next.run(req).await;
    }
    let auth = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let token_ok = crate::tokens::validate_token(&state.db, auth)
        .await
        .unwrap_or(false);
    let session_ok = req
        .headers()
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|part| {
                let (k, v) = part.trim().split_once('=')?;
                if k == SESSION_COOKIE {
                    Some(v.to_string())
                } else {
                    None
                }
            })
        })
        .and_then(|t| state.sessions.resolve(&t))
        .is_some();
    if token_ok || session_ok {
        next.run(req).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "login or Bearer token required for non-loopback dashboard" })),
        )
            .into_response()
    }
}

fn is_public_path(path: &str) -> bool {
    matches!(
        path,
        "/health"
            | "/api/health"
            | "/settings/doctor"
            | "/api/settings/doctor"
            | "/auth/login"
            | "/api/auth/login"
            | "/auth/me"
            | "/api/auth/me"
    )
}
