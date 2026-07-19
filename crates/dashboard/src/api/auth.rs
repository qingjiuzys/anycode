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

fn test_auth_bypass() -> bool {
    std::env::var("ANYCODE_DASHBOARD_TEST_AUTH_BYPASS")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

/// Test-only bypass (CI). Embedded desktop no longer grants full API trust —
/// a verified cloud session is required (see auth_middleware).
fn loopback_trusted_access(host: &str) -> bool {
    is_loopback_host(host) && test_auth_bypass()
}

fn cloud_session_present() -> bool {
    anycode_llm::read_cloud_access_token().is_some_and(|token| !token.trim().is_empty())
}

async fn identity_approved_for_token(token: &str) -> bool {
    let url = format!(
        "{}/api/v1/auth/me",
        anycode_llm::account_api_url().trim_end_matches('/')
    );
    let client = reqwest::Client::new();
    match client.get(&url).bearer_auth(token).send().await {
        Ok(resp) if resp.status().is_success() => {
            resp.json::<serde_json::Value>()
                .await
                .ok()
                .and_then(|value| value["identity_status"].as_str().map(str::to_string))
                .as_deref()
                == Some("approved")
        }
        _ => false,
    }
}

pub async fn cloud_identity_verified() -> bool {
    let token = match anycode_llm::read_cloud_access_token() {
        Some(token) if !token.trim().is_empty() => token,
        _ => return false,
    };
    if identity_approved_for_token(&token).await {
        return true;
    }
    if anycode_llm::refresh_cloud_access_token().await.is_err() {
        return false;
    }
    let refreshed = anycode_llm::read_cloud_access_token().unwrap_or_default();
    identity_approved_for_token(&refreshed).await
}

pub async fn resolve_request_user(
    state: &AppState,
    parts: &axum::http::request::Parts,
) -> Option<crate::auth_session::AuthUser> {
    if loopback_trusted_access(&state.host) {
        return auth_session::local_trusted_user(&state.db).await.ok();
    }
    if cloud_identity_verified().await {
        if let Some(session) = anycode_llm::read_cloud_session() {
            let email = session
                .user_email
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "cloud@anycode".to_string());
            return Some(auth_session::AuthUser {
                id: format!("cloud:{email}"),
                email,
                display_name: "Cloud User".into(),
                role: "owner".into(),
                organization_id: "cloud".into(),
                auth_method: "cloud_device".into(),
            });
        }
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
    let path = req.uri().path();
    if is_public_path(path) {
        return next.run(req).await;
    }
    if loopback_trusted_access(&state.host) {
        return next.run(req).await;
    }
    if cloud_session_present() && cloud_identity_verified().await {
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
            Json(json!({
                "error": "verified cloud account required",
                "hint": "Link and verify your anyCode cloud account before using the workbench API"
            })),
        )
            .into_response()
    }
}

fn is_public_path(path: &str) -> bool {
    matches!(
        path,
        "/health"
            | "/api/health"
            | "/auth/login"
            | "/api/auth/login"
            | "/auth/me"
            | "/api/auth/me"
            | "/auth/logout"
            | "/api/auth/logout"
            | "/cloud/session"
            | "/api/cloud/session"
            | "/cloud/link/start"
            | "/api/cloud/link/start"
            | "/cloud/link/poll"
            | "/api/cloud/link/poll"
            | "/cloud/unlink"
            | "/api/cloud/unlink"
    ) || path.starts_with("/setup/")
        || path.starts_with("/api/setup/")
        || path == "/bootstrap"
        || path == "/api/bootstrap"
}
