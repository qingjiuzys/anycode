use super::*;
use crate::schema::LOCAL_USER_ID;
use axum::extract::Query;
use axum::response::Redirect;

#[derive(Deserialize)]
pub struct LoginBody {
    pub email: String,
    #[serde(default)]
    pub password: String,
}

#[derive(Deserialize)]
pub struct DesktopBootstrapQuery {
    pub token: String,
}

pub async fn get_auth_me(
    State(state): State<AppState>,
    req: axum::extract::Request,
) -> impl IntoResponse {
    let (parts, _) = req.into_parts();
    match crate::api::auth::resolve_request_user(&state, &parts).await {
        Some(user) => Json(json!({ "user": user, "authenticated": true })).into_response(),
        None => (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "authenticated": false, "error": "not authenticated" })),
        )
            .into_response(),
    }
}

pub async fn post_auth_login(
    State(state): State<AppState>,
    Json(body): Json<LoginBody>,
) -> impl IntoResponse {
    match crate::auth_session::login(&state.db, &body.email, &body.password).await {
        Ok(Some(user)) => {
            let token = state.sessions.create(&user.id);
            let cookie = format!(
                "{}={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=604800",
                crate::auth_session::SESSION_COOKIE,
                token
            );
            let _ = crate::audit::record_audit(
                &state.db,
                crate::audit::AuditEventInput::low("user_login", json!({ "email": user.email })),
            )
            .await;
            let mut resp = Json(json!({ "user": user, "authenticated": true })).into_response();
            if let Ok(v) = cookie.parse() {
                resp.headers_mut().append(axum::http::header::SET_COOKIE, v);
            }
            resp
        }
        Ok(None) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "invalid credentials" })),
        )
            .into_response(),
        Err(e) => {
            tracing::warn!(error = %e, "dashboard login failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "login temporarily unavailable" })),
            )
                .into_response()
        }
    }
}

/// One-shot Desktop handshake: exchange an in-process bootstrap token for a
/// local `dw_session` cookie, then 303 to `/` so the token never stays in the URL.
pub async fn get_desktop_bootstrap(
    State(state): State<AppState>,
    Query(query): Query<DesktopBootstrapQuery>,
) -> impl IntoResponse {
    if !crate::api::auth::is_desktop_bootstrap_enabled(&state.host) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "desktop bootstrap unavailable" })),
        )
            .into_response();
    }
    let provided = query.token.trim();
    if provided.is_empty() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "bootstrap token required" })),
        )
            .into_response();
    }
    let expected = {
        let mut guard = state.desktop_bootstrap_token.lock().await;
        guard.take()
    };
    let Some(expected) = expected else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "bootstrap token already used or missing" })),
        )
            .into_response();
    };
    if provided != expected {
        // Do not restore a mismatched token — treat as consumed to avoid probing.
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "invalid bootstrap token" })),
        )
            .into_response();
    }

    let session = state.sessions.create(LOCAL_USER_ID);
    let cookie = format!(
        "{}={}; Path=/; HttpOnly; SameSite=Strict; Max-Age=604800",
        crate::auth_session::SESSION_COOKIE,
        session
    );
    let mut resp = Redirect::to("/").into_response();
    if let Ok(v) = cookie.parse() {
        resp.headers_mut().append(axum::http::header::SET_COOKIE, v);
    }
    resp
}

pub async fn post_auth_logout(
    State(state): State<AppState>,
    req: axum::extract::Request,
) -> impl IntoResponse {
    let (parts, _) = req.into_parts();
    if let Some(token) = crate::api::auth::cookie_from_request(&parts) {
        state.sessions.revoke(&token);
    }
    // Local Workbench logout must not clear cloud link / hosted models.
    let clear = format!(
        "{}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0",
        crate::auth_session::SESSION_COOKIE
    );
    let mut resp = Json(json!({ "ok": true })).into_response();
    if let Ok(v) = clear.parse() {
        resp.headers_mut().append(axum::http::header::SET_COOKIE, v);
    }
    resp
}
