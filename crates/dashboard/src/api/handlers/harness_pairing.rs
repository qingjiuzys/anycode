//! Desktop pairing BFF. The product client secret never leaves the server.
//! Opaque device tokens are hashed at rest; Desktop may mirror the one-shot
//! token into the OS keychain after confirm.
use super::*;
use crate::api::auth::resolve_request_user_from_headers;
use axum::http::HeaderMap;
use axum::response::Response;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub const DEVICE_TOKEN_HEADER: &str = "x-anycode-device-token";
pub const KEYCHAIN_SERVICE: &str = "anycode.harness.device";

fn opaque43() -> String {
    let mut raw = [0u8; 32];
    raw[..16].copy_from_slice(Uuid::new_v4().as_bytes());
    raw[16..].copy_from_slice(Uuid::new_v4().as_bytes());
    URL_SAFE_NO_PAD.encode(raw)
}

pub fn token_hash(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn now_sqlite() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn expires_in(secs: i64) -> String {
    (chrono::Utc::now() + chrono::Duration::seconds(secs))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

async fn pairing_subject(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    if let Some(client) = &state.accounts_sso {
        let token = super::bearer_opaque_token(headers).ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "818cloud opaque bearer token required" })),
            )
                .into_response()
        })?;
        let identity = client.introspect(token, None).await.map_err(|e| {
            (
                StatusCode::FORBIDDEN,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response()
        })?;
        return Ok(identity.sub.to_string());
    }
    let user = resolve_request_user_from_headers(state, headers)
        .await
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "authenticated user required" })),
            )
                .into_response()
        })?;
    Ok(super::stable_uuid(&user.id).to_string())
}

pub async fn create_harness_pairing_challenge(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Response {
    match create_challenge_inner(&state, &headers, &body).await {
        Ok(v) => (StatusCode::OK, Json(v)).into_response(),
        Err(r) => r,
    }
}

async fn create_challenge_inner(
    state: &AppState,
    headers: &HeaderMap,
    body: &serde_json::Value,
) -> Result<serde_json::Value, Response> {
    let sub = pairing_subject(state, headers).await?;
    let label = body
        .get("label")
        .and_then(|v| v.as_str())
        .unwrap_or("desktop")
        .trim();
    if label.is_empty() || label.len() > 64 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "device label bounds" })),
        )
            .into_response());
    }
    let challenge = opaque43();
    let id = Uuid::new_v4().to_string();
    state
        .db
        .insert_harness_device_challenge(
            &id,
            &sub,
            label,
            &token_hash(&challenge),
            &expires_in(300),
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response()
        })?;
    Ok(json!({
        "device_id": id,
        "challenge": challenge,
        "expires_in": 300,
        "note": "one-use challenge; confirm locally. PRODUCT_SSO_CLIENT_SECRET is never issued."
    }))
}

pub async fn confirm_harness_pairing(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Response {
    match confirm_inner(&state, &headers, &body).await {
        Ok(v) => (StatusCode::OK, Json(v)).into_response(),
        Err(r) => r,
    }
}

async fn confirm_inner(
    state: &AppState,
    headers: &HeaderMap,
    body: &serde_json::Value,
) -> Result<serde_json::Value, Response> {
    let sub = pairing_subject(state, headers).await?;
    let challenge = body
        .get("challenge")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "challenge required" })),
            )
                .into_response()
        })?;
    let token = opaque43();
    let confirmed = state
        .db
        .confirm_harness_device(&token_hash(challenge), &token_hash(&token), &now_sqlite())
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response()
        })?
        .ok_or_else(|| {
            (
                StatusCode::FORBIDDEN,
                Json(json!({ "error": "challenge is invalid, used, or expired" })),
            )
                .into_response()
        })?;
    if confirmed.1 != sub {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "challenge belongs to another subject" })),
        )
            .into_response());
    }
    Ok(json!({
        "device_id": confirmed.0,
        "token": token,
        "keychain_service": KEYCHAIN_SERVICE,
        "keychain_account": confirmed.0,
        "note": "store token in OS keychain on Desktop; server kept only the hash"
    }))
}

pub async fn revoke_harness_pairing(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Response {
    match revoke_inner(&state, &headers, &body).await {
        Ok(v) => (StatusCode::OK, Json(v)).into_response(),
        Err(r) => r,
    }
}

async fn revoke_inner(
    state: &AppState,
    headers: &HeaderMap,
    body: &serde_json::Value,
) -> Result<serde_json::Value, Response> {
    let sub = pairing_subject(state, headers).await?;
    let device_id = body
        .get("device_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "device_id required" })),
            )
                .into_response()
        })?;
    let ok = state
        .db
        .revoke_harness_device(device_id, &sub, &now_sqlite())
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response()
        })?;
    if !ok {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "device is not an active pairing for this subject" })),
        )
            .into_response());
    }
    Ok(json!({ "ok": true, "device_id": device_id, "revoked": true }))
}

pub async fn resolve_harness_device(
    state: &AppState,
    headers: &HeaderMap,
    subject: &str,
) -> Result<Option<Uuid>, Response> {
    let Some(raw) = headers
        .get(DEVICE_TOKEN_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return Ok(None);
    };
    let row = state
        .db
        .harness_device_by_token_hash(&token_hash(raw))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response()
        })?;
    let Some((id, sub, status)) = row else {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "unknown device token" })),
        )
            .into_response());
    };
    if status != "active" || sub != subject {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "device revoked or subject mismatch" })),
        )
            .into_response());
    }
    Uuid::parse_str(&id).map(Some).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "device id" })),
        )
            .into_response()
    })
}

fn pairing_err(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "error": message }))).into_response()
}

fn sso_ready(
    state: &AppState,
) -> Result<
    (
        std::sync::Arc<anycode_harness_cloud818::identity::AccountsClient>,
        String,
    ),
    Response,
> {
    let client = state
        .accounts_sso
        .clone()
        .ok_or_else(|| pairing_err(StatusCode::SERVICE_UNAVAILABLE, "sso hop is not configured"))?;
    let redirect = state
        .sso_redirect
        .clone()
        .ok_or_else(|| pairing_err(StatusCode::SERVICE_UNAVAILABLE, "sso hop is not configured"))?;
    Ok((client, redirect))
}

pub async fn begin_harness_pairing_sso(State(state): State<AppState>) -> Response {
    match begin_sso_inner(&state).await {
        Ok(v) => (StatusCode::OK, Json(v)).into_response(),
        Err(r) => r,
    }
}

async fn begin_sso_inner(state: &AppState) -> Result<serde_json::Value, Response> {
    let (client, redirect) = sso_ready(state)?;
    let poll_token = opaque43();
    let verifier = opaque43();
    let state_token = opaque43();
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let mut authorize =
        reqwest::Url::parse(&format!("{}/api/v2/sso/authorize", client.issuer()))
            .map_err(|_| pairing_err(StatusCode::INTERNAL_SERVER_ERROR, "sso authorize url"))?;
    authorize
        .query_pairs_mut()
        .append_pair("client_id", "anycode")
        .append_pair("redirect_uri", &redirect)
        .append_pair("state", &state_token)
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256");
    state
        .sso_hops
        .insert(
            poll_token.clone(),
            crate::api::state::SsoHop {
                verifier,
                state: state_token,
                redirect_uri: redirect,
                access_token: None,
                created: std::time::Instant::now(),
            },
        )
        .await;
    Ok(json!({
        "authorize_url": authorize.as_str(),
        "poll_token": poll_token,
        "expires_in": 300,
        "note": "open authorize_url; PRODUCT_SSO_CLIENT_SECRET stays on this server"
    }))
}

pub async fn complete_harness_pairing_sso(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    match complete_sso_inner(&state, &body).await {
        Ok(v) => (StatusCode::OK, Json(v)).into_response(),
        Err(r) => r,
    }
}

async fn complete_sso_inner(
    state: &AppState,
    body: &serde_json::Value,
) -> Result<serde_json::Value, Response> {
    let (client, _) = sso_ready(state)?;
    let poll_token = body
        .get("poll_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| pairing_err(StatusCode::BAD_REQUEST, "poll_token required"))?;
    let code = body
        .get("code")
        .and_then(|v| v.as_str())
        .ok_or_else(|| pairing_err(StatusCode::BAD_REQUEST, "code required"))?;
    let state_token = body
        .get("state")
        .and_then(|v| v.as_str())
        .ok_or_else(|| pairing_err(StatusCode::BAD_REQUEST, "state required"))?;
    let hop = state
        .sso_hops
        .by_poll(poll_token)
        .await
        .ok_or_else(|| pairing_err(StatusCode::FORBIDDEN, "unknown or expired SSO hop"))?;
    if hop.state != state_token {
        return Err(pairing_err(StatusCode::FORBIDDEN, "SSO hop state mismatch"));
    }
    let token = client
        .exchange_code(code, &hop.redirect_uri, &hop.verifier)
        .await
        .map_err(|e| pairing_err(StatusCode::FORBIDDEN, &e.to_string()))?;
    if !state.sso_hops.set_access_token(poll_token, token).await {
        return Err(pairing_err(
            StatusCode::FORBIDDEN,
            "unknown or expired SSO hop",
        ));
    }
    Ok(json!({ "ok": true, "pending": false }))
}

pub async fn poll_harness_pairing_sso(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    match poll_sso_inner(&state, &body).await {
        Ok(v) => (StatusCode::OK, Json(v)).into_response(),
        Err(r) => r,
    }
}

async fn poll_sso_inner(
    state: &AppState,
    body: &serde_json::Value,
) -> Result<serde_json::Value, Response> {
    let _ = sso_ready(state)?;
    let poll_token = body
        .get("poll_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| pairing_err(StatusCode::BAD_REQUEST, "poll_token required"))?;
    match state.sso_hops.consume_if_ready(poll_token).await {
        crate::api::state::SsoHopPoll::Unknown => Err(pairing_err(
            StatusCode::FORBIDDEN,
            "unknown or expired SSO hop",
        )),
        crate::api::state::SsoHopPoll::Expired => Err(pairing_err(
            StatusCode::FORBIDDEN,
            "unknown or expired SSO hop",
        )),
        crate::api::state::SsoHopPoll::Pending => Ok(json!({ "pending": true })),
        crate::api::state::SsoHopPoll::Ready(access_token) => Ok(json!({
            "access_token": access_token,
            "token_type": "Bearer",
            "note": "memory only; Desktop must not persist this SSO token"
        })),
    }
}

#[derive(Deserialize)]
pub struct SsoCallbackQuery {
    pub code: String,
    pub state: String,
    pub iss: Option<String>,
}

pub async fn callback_harness_pairing_sso(
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<SsoCallbackQuery>,
) -> Response {
    match callback_sso_inner(&state, &query).await {
        Ok(()) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            "<!doctype html><title>anyCode pairing</title><p>Desktop may close this window. The access token is not in this page.</p>",
        )
            .into_response(),
        Err(r) => r,
    }
}

async fn callback_sso_inner(state: &AppState, query: &SsoCallbackQuery) -> Result<(), Response> {
    let (client, _) = sso_ready(state)?;
    if let Some(iss) = query.iss.as_deref() {
        if iss != client.issuer() {
            return Err(pairing_err(StatusCode::FORBIDDEN, "SSO issuer mismatch"));
        }
    }
    let (poll_token, hop) = state
        .sso_hops
        .by_state(&query.state)
        .await
        .ok_or_else(|| pairing_err(StatusCode::FORBIDDEN, "unknown or expired SSO hop"))?;
    let token = client
        .exchange_code(&query.code, &hop.redirect_uri, &hop.verifier)
        .await
        .map_err(|e| pairing_err(StatusCode::FORBIDDEN, &e.to_string()))?;
    if !state.sso_hops.set_access_token(&poll_token, token).await {
        return Err(pairing_err(
            StatusCode::FORBIDDEN,
            "unknown or expired SSO hop",
        ));
    }
    Ok(())
}
