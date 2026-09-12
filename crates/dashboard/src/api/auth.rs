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

/// True when the request carries a Workbench session cookie or API Bearer token.
/// Used for sensitive fields (e.g. cloud `access_token`) that must not be handed
/// to every loopback process merely because the bind host is 127.0.0.1.
#[allow(dead_code)]
pub fn request_has_local_credential(state: &AppState, headers: &axum::http::HeaderMap) -> bool {
    if let Some(cookies) = headers.get(header::COOKIE).and_then(|v| v.to_str().ok()) {
        if cookies.split(';').any(|part| {
            part.trim()
                .split_once('=')
                .is_some_and(|(k, v)| k == SESSION_COOKIE && state.sessions.resolve(v).is_some())
        }) {
            return true;
        }
    }
    let auth = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    // validate_token is async — callers that need a sync check use cookie only
    // or the async helper below. Keep a cheap non-empty Bearer presence gate here
    // and let `request_may_read_cloud_token` do the real validation.
    !auth.is_empty() && auth.to_ascii_lowercase().starts_with("bearer ")
}

/// Async credential check for returning cloud JWT material.
pub async fn request_may_read_cloud_token(
    state: &AppState,
    headers: &axum::http::HeaderMap,
) -> bool {
    if let Some(cookies) = headers.get(header::COOKIE).and_then(|v| v.to_str().ok()) {
        if cookies.split(';').any(|part| {
            part.trim()
                .split_once('=')
                .is_some_and(|(k, v)| k == SESSION_COOKIE && state.sessions.resolve(v).is_some())
        }) {
            return true;
        }
    }
    let auth = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    crate::tokens::validate_token(&state.db, auth)
        .await
        .unwrap_or(false)
}

/// Loopback-trusted access for CI (`test_bypass`) only.
///
/// Packaged Desktop must use the `dw_session` cookie from
/// `/api/auth/desktop-bootstrap` — embedding alone is **not** enough to skip
/// auth (otherwise any process that can hit the ephemeral loopback port is the
/// workbench owner).
fn loopback_trusted_access(host: &str, test_bypass: bool, _embedded: bool) -> bool {
    is_loopback_host(host) && test_bypass
}

pub(crate) fn embedded_desktop() -> bool {
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

fn harness_pairing_bearer_request(path: &str, headers: &axum::http::HeaderMap) -> bool {
    let pairing = path.contains("/harness/pairing/");
    let graphs = path.contains("/harness/graphs/");
    if !pairing && !graphs {
        return false;
    }
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            let v = v.trim();
            v.len() > 7 && v[..7].eq_ignore_ascii_case("bearer ") && !v[7..].trim().is_empty()
        })
}

pub fn origin_allowed(origin: &str, embedded: bool) -> bool {
    if ALLOWED_BROWSER_ORIGINS.contains(&origin) {
        return true;
    }
    // Desktop embeds the SPA on an ephemeral loopback port (e.g. :53402).
    if embedded && origin_host(origin).is_some_and(is_loopback_host) {
        return true;
    }
    false
}

/// Same-origin exemption: the dashboard serves its own Workbench UI, so pages
/// it served must be able to mutate it on ANY configured port — the hardcoded
/// allowlist cannot enumerate user-chosen ports (default 43180 was missing,
/// 403-ing every mutation from the browser workbench). DNS-rebinding origins
/// never match: the origin authority must equal the Host header exactly and
/// its host must be loopback.
fn origin_matches_host_header(origin: &str, host_header: &str) -> bool {
    let Some(rest) = origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"))
    else {
        return false;
    };
    let authority = rest.split('/').next().unwrap_or("");
    let host = host_header.split(',').next().unwrap_or(host_header).trim();
    if authority.is_empty() || host.is_empty() {
        return false;
    }
    authority.eq_ignore_ascii_case(host) && origin_host(origin).is_some_and(is_loopback_host)
}

fn origin_host(origin: &str) -> Option<&str> {
    let rest = origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"))?;
    let authority = rest.split('/').next()?;
    Some(authority.rsplit_once(':').map_or(authority, |(h, _)| h))
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
        let same_origin = req
            .headers()
            .get(header::HOST)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|host| origin_matches_host_header(origin, host));
        let harness_bearer = harness_pairing_bearer_request(req.uri().path(), req.headers());
        if !origin_allowed(origin, state.embedded_desktop) && !same_origin && !harness_bearer {
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

pub async fn resolve_request_user_from_headers(
    state: &AppState,
    headers: &axum::http::HeaderMap,
) -> Option<crate::auth_session::AuthUser> {
    if loopback_trusted_access(&state.host, state.test_auth_bypass, state.embedded_desktop) {
        return auth_session::local_trusted_user(&state.db).await.ok();
    }
    if let Some(token) = headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|part| {
                let (k, v) = part.trim().split_once('=')?;
                (k == SESSION_COOKIE).then_some(v.to_string())
            })
        })
    {
        if let Some(uid) = state.sessions.resolve(&token) {
            return auth_session::get_user_by_id(&state.db, &uid)
                .await
                .ok()
                .flatten();
        }
    }
    let auth = headers
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

pub async fn resolve_request_user(
    state: &AppState,
    parts: &axum::http::request::Parts,
) -> Option<crate::auth_session::AuthUser> {
    if loopback_trusted_access(&state.host, state.test_auth_bypass, state.embedded_desktop) {
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
    if loopback_trusted_access(&state.host, state.test_auth_bypass, state.embedded_desktop) {
        return next.run(req).await;
    }
    // Desktop pairing talks to a server-side BFF that has no dw_session cookie.
    // The handler still introspects the opaque 818cloud bearer.
    if state.accounts_sso.is_some() && harness_pairing_bearer_request(path, req.headers()) {
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

pub fn is_desktop_bootstrap_enabled(host: &str, embedded: bool) -> bool {
    embedded && is_loopback_host(host)
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
            | "/api/cloud/unlink" // `/api/cloud/upstream/*` requires a local session / API token (not public).
            | "/api/harness/pairing/sso/begin"
            | "/api/harness/pairing/sso/complete"
            | "/api/harness/pairing/sso/poll"
            | "/api/harness/pairing/sso/callback"
            | "/harness/pairing/sso/begin"
            | "/harness/pairing/sso/complete"
            | "/harness/pairing/sso/poll"
            | "/harness/pairing/sso/callback"
    ) || path.starts_with("/setup/")
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
    fn cloud_upstream_proxy_requires_auth() {
        assert!(!is_public_path("/api/cloud/upstream/api/v1/account/bundle"));
        assert!(!is_public_path("/cloud/upstream/api/v1/auth/me"));
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
        assert!(origin_allowed("tauri://localhost", false));
        assert!(origin_allowed("http://127.0.0.1:43199", false));
        assert!(!origin_allowed("https://evil.example", false));
    }

    #[test]
    fn embedded_desktop_allows_ephemeral_loopback_origin() {
        assert!(origin_allowed("http://127.0.0.1:53402", true));
        assert!(origin_allowed("http://localhost:61234", true));
        assert!(!origin_allowed("http://evil.example:53402", true));
    }

    #[test]
    fn host_header_strips_port_for_loopback() {
        assert!(host_header_loopback("127.0.0.1:43180"));
        assert!(host_header_loopback("localhost"));
        assert!(!host_header_loopback("evil.example"));
        assert!(!host_header_loopback("evil.example:443"));
    }

    #[test]
    fn same_origin_loopback_matches_on_any_port() {
        // 默认工作台端口 43180:自服务的页面必须能改自己。
        assert!(origin_matches_host_header(
            "http://127.0.0.1:43180",
            "127.0.0.1:43180"
        ));
        assert!(origin_matches_host_header(
            "http://localhost:43180",
            "localhost:43180"
        ));
        // 端口不同 = 跨源,不放行(本机其它端口可能挂着恶意页面)。
        assert!(!origin_matches_host_header(
            "http://127.0.0.1:9999",
            "127.0.0.1:43180"
        ));
        // DNS rebinding:origin 非 loopback 一律不放行。
        assert!(!origin_matches_host_header(
            "http://evil.example:43180",
            "127.0.0.1:43180"
        ));
        assert!(!origin_matches_host_header(
            "http://127.0.0.1:43180",
            "evil.example"
        ));
        assert!(!origin_matches_host_header(
            "ftp://127.0.0.1:43180",
            "127.0.0.1:43180"
        ));
    }
}
