use super::*;

#[derive(Deserialize)]
pub struct LoginBody {
    pub email: String,
    #[serde(default)]
    pub password: String,
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
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn post_auth_logout(
    State(state): State<AppState>,
    req: axum::extract::Request,
) -> impl IntoResponse {
    let (parts, _) = req.into_parts();
    if let Some(token) = crate::api::auth::cookie_from_request(&parts) {
        state.sessions.revoke(&token);
    }
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
