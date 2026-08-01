use crate::api::state::AppState;
use crate::auth_session::{self, SESSION_COOKIE};
use crate::service_governance::is_loopback_host;
use axum::{
    body::Body,
    extract::State,
    http::{header, Method, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Browser origins allowed for CORS and mutating-request Origin checks.
pub const ALLOWED_BROWSER_ORIGINS: &[&str] = &[
    "http://127.0.0.1:43180",
    "http://localhost:43180",
    "http://127.0.0.1:43199",
    "http://localhost:43199",
    "tauri://localhost",
    "https://tauri.localhost",
];

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

/// Test-only bypass (CI). Product paths use local session / API token.
fn loopback_trusted_access(host: &str, test_bypass: bool) -> bool {
    is_loopback_host(host) && test_bypass
}

fn embedded_desktop() -> bool {
    std::env::var("ANYCODE_DASHBOARD_EMBEDDED_DESKTOP")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

/// Generate a process-only, high-entropy desktop bootstrap token (≥256 bits).
#[must_use]
pub fn generate_desktop_bootstrap_token() -> String {
    format!(
        "dbt_{}_{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

pub fn origin_allowed(origin: &str) -> bool {
    ALLOWED_BROWSER_ORIGINS.contains(&origin)
}

fn host_header_loopback(host_header: &str) -> bool {
    let host = host_header.split(',').next().unwrap_or(host_header).trim();
    let host_only = if let Some(stripped) = host.strip_prefix('[') {
        stripped.split(']').next().unwrap_or(host)
    } else {
        host.rsplit_once(':')
            .and_then(|(h, port)| {
                if port.chars().all(|c| c.is_ascii_digit()) {
                    Some(h)
                } else {
                    None
                }
            })
            .unwrap_or(host)
    };
    is_loopback_host(host_only)
}

/// Reject mutating browser requests with a disallowed Origin, and reject
/// non-loopback Host when the dashboard itself is bound to loopback.
pub async fn mutate_origin_guard(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let method = req.method().clone();
    if matches!(
        method,
        Method::GET | Method::HEAD | Method::OPTIONS | Method::TRACE
    ) {
        return next.run(req).await;
    }

    if let Some(origin) = req
        .headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
    {
        if !origin_allowed(origin) {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({ "error": "origin not allowed" })),
            )
                .into_response();
        }
    }

    if is_loopback_host(&state.host) {
        if let Some(host) = req
            .headers()
            .get(header::HOST)
            .and_then(|v| v.to_str().ok())
        {
            if !host_header_loopback(host) {
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({ "error": "host not allowed" })),
                )
                    .into_response();
            }
        }
    }

    next.run(req).await
}

pub async fn resolve_request_user(
    state: &AppState,
    parts: &axum::http::request::Parts,
) -> Option<crate::auth_session::AuthUser> {
    if loopback_trusted_access(&state.host, state.test_auth_bypass) {
        return auth_session::local_trusted_user(&state.db).await.ok();
    }
    // Local Workbench identity is always org_local / user_local.
    // Cloud credentials never substitute for the local session user.
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
    // Setup endpoints are only public on loopback — on a non-loopback bind they
    // would let anyone write first-run configuration anonymously.
    let setup_path = path.starts_with("/setup/") || path.starts_with("/api/setup/");
    if is_public_path(path) && (!setup_path || is_loopback_host(&state.host)) {
        return next.run(req).await;
    }
    if loopback_trusted_access(&state.host, state.test_auth_bypass) {
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
                "error": "local session or API token required",
                "hint": "Open anyCode Desktop for a local workbench session, or provide an API Bearer token"
            })),
        )
            .into_response()
    }
}

pub fn is_desktop_bootstrap_enabled(host: &str) -> bool {
    embedded_desktop() && is_loopback_host(host)
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
            | "/auth/desktop-bootstrap"
            | "/api/auth/desktop-bootstrap"
            | "/cloud/session"
            | "/api/cloud/session"
            | "/cloud/link/start"
            | "/api/cloud/link/start"
            | "/cloud/link/poll"
            | "/api/cloud/link/poll"
            | "/cloud/unlink"
            | "/api/cloud/unlink"
    ) || path.starts_with("/cloud/upstream/")
        || path.starts_with("/api/cloud/upstream/")
        || path.starts_with("/setup/")
        || path.starts_with("/api/setup/")
        || path == "/bootstrap"
        || path == "/api/bootstrap"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_paths_are_still_public_on_loopback() {
        assert!(is_public_path("/api/setup/status"));
        assert!(is_public_path("/setup/workspace"));
    }

    #[test]
    fn cloud_upstream_proxy_is_public() {
        assert!(is_public_path("/api/cloud/upstream/api/v1/account/bundle"));
        assert!(is_public_path("/cloud/upstream/api/v1/auth/me"));
        assert!(!is_public_path("/api/cloud/sync-models"));
    }

    #[test]
    fn bootstrap_token_is_high_entropy() {
        let a = generate_desktop_bootstrap_token();
        let b = generate_desktop_bootstrap_token();
        assert!(a.starts_with("dbt_"));
        assert_ne!(a, b);
        // Two UUID simple hex strings = 64 hex chars + separators.
        assert!(a.len() >= 64);
    }

    #[test]
    fn origin_allowlist_covers_workbench_and_tauri() {
        assert!(origin_allowed("http://127.0.0.1:43180"));
        assert!(origin_allowed("tauri://localhost"));
        assert!(!origin_allowed("https://evil.example"));
    }

    #[test]
    fn host_header_strips_port_for_loopback() {
        assert!(host_header_loopback("127.0.0.1:43180"));
        assert!(host_header_loopback("localhost"));
        assert!(!host_header_loopback("evil.example"));
        assert!(!host_header_loopback("evil.example:443"));
    }
}
