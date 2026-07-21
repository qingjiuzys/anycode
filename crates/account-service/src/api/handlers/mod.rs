use crate::api::{json_error, AppState, AuthContext};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde::Deserialize;

pub mod admin;

pub async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let wechat_missing = crate::billing_wechat::wechat_pay_missing(&state.config);
    Json(serde_json::json!({
        "ok": true,
        "service": "anycode-account",
        "version": state.version,
        "portal_url": state.config.portal_url,
        "wechat_pay_configured": wechat_missing.is_empty(),
        "wechat_pay_missing": wechat_missing,
        "default_payment_provider": state.config.default_payment_provider,
    }))
}

#[derive(Deserialize)]
pub struct RegisterBody {
    pub email: String,
    pub password: String,
    pub display_name: String,
    pub verification_code: String,
    pub privacy_consent: bool,
    pub consent_version: String,
}

#[derive(Deserialize)]
pub struct SendEmailCodeBody {
    pub email: String,
}

pub async fn auth_send_email_code(
    State(state): State<AppState>,
    Json(body): Json<SendEmailCodeBody>,
) -> impl IntoResponse {
    match crate::email_verification::send_registration_code(&state.db, &state.config, &body.email)
        .await
    {
        Ok(()) => Json(serde_json::json!({ "ok": true, "expires_in": 600 })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, &e.to_string()).into_response(),
    }
}

pub async fn auth_register(
    State(state): State<AppState>,
    Json(body): Json<RegisterBody>,
) -> impl IntoResponse {
    match crate::store::register(
        &state.db,
        &state.config,
        crate::store::RegisterInput {
            email: body.email,
            password: body.password,
            display_name: body.display_name,
            verification_code: body.verification_code,
            privacy_consent: body.privacy_consent,
            consent_version: body.consent_version,
        },
    )
    .await
    {
        Ok((user, token)) => Json(serde_json::json!({
            "user": user,
            "token": token,
            "authenticated": true,
        }))
        .into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, &e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct LoginBody {
    pub email: String,
    pub password: String,
}

pub async fn auth_login(
    State(state): State<AppState>,
    Json(body): Json<LoginBody>,
) -> impl IntoResponse {
    match crate::store::login(&state.db, &body.email, &body.password).await {
        Ok(Some((user, token))) => Json(serde_json::json!({
            "user": user,
            "token": token,
            "authenticated": true,
        }))
        .into_response(),
        Ok(None) => json_error(StatusCode::UNAUTHORIZED, "invalid credentials").into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

pub async fn auth_logout(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let _ = crate::store::logout(&state.db, &ctx.token).await;
    Json(serde_json::json!({ "ok": true }))
}

pub async fn auth_me(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let identity_status: Option<String> =
        sqlx::query_scalar("SELECT identity_status FROM users WHERE id = ?")
            .bind(&ctx.user.id)
            .fetch_optional(state.db.pool())
            .await
            .ok()
            .flatten();
    Json(serde_json::json!({
        "user": ctx.user,
        "identity_status": identity_status.unwrap_or_else(|| "identity_pending".into()),
        "authenticated": true
    }))
}

#[derive(Deserialize)]
pub struct ProfilePatchBody {
    pub display_name: String,
}

pub async fn patch_account_profile(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Json(body): Json<ProfilePatchBody>,
) -> impl IntoResponse {
    match crate::store::update_display_name(&state.db, &ctx.user.id, &body.display_name).await {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, &e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct IdentitySubmitBody {
    pub legal_name: String,
    pub id_number: String,
}

pub async fn identity_submit(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Json(body): Json<IdentitySubmitBody>,
) -> impl IntoResponse {
    let Some(secret) = state.config.identity_encryption_secret.as_deref() else {
        return json_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "IDENTITY_ENCRYPTION_SECRET is not configured",
        )
        .into_response();
    };
    match crate::identity::submit(
        &state.db,
        &ctx.user.id,
        &body.legal_name,
        &body.id_number,
        secret,
    )
    .await
    {
        Ok(()) => Json(serde_json::json!({
            "ok": true,
            "status": "approved",
            "document_upload_supported": false
        }))
        .into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, &e.to_string()).into_response(),
    }
}

pub async fn identity_status(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let Some(secret) = state.config.identity_encryption_secret.as_deref() else {
        return json_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "identity service unavailable",
        )
        .into_response();
    };
    match crate::identity::status(&state.db, &ctx.user.id, secret).await {
        Ok(identity) => Json(serde_json::json!({ "identity": identity })).into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

pub async fn audit_ingest(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Json(body): Json<crate::conversation_audit::AuditIngest>,
) -> impl IntoResponse {
    let Some(secret) = state.config.audit_encryption_secret.as_deref() else {
        return json_error(StatusCode::SERVICE_UNAVAILABLE, "audit service unavailable")
            .into_response();
    };
    match crate::conversation_audit::ingest(
        &state.db,
        &ctx.user.organization_id,
        &ctx.user.id,
        &body,
        secret,
    )
    .await
    {
        Ok(keyword_hits) => {
            Json(serde_json::json!({ "ok": true, "keyword_hits": keyword_hits })).into_response()
        }
        Err(e) => json_error(StatusCode::BAD_REQUEST, &e.to_string()).into_response(),
    }
}

pub async fn get_account_bundle(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    match crate::store::get_account_bundle(&state.db, &ctx.user.organization_id, &ctx.user).await {
        Ok(bundle) => Json(serde_json::json!({ "account": bundle })).into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

pub async fn get_subscription(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    match crate::store::get_subscription(&state.db, &ctx.user.organization_id).await {
        Ok(sub) => Json(serde_json::json!({ "subscription": sub })).into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct UpgradeBody {
    pub plan: String,
}

pub async fn upgrade_subscription(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Json(body): Json<UpgradeBody>,
) -> impl IntoResponse {
    match crate::store::upgrade_plan(&state.db, &ctx.user.organization_id, &body.plan).await {
        Ok(sub) => Json(serde_json::json!({ "subscription": sub })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, &e.to_string()).into_response(),
    }
}

pub async fn get_entitlements(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    match crate::store::get_entitlements(&state.db, &ctx.user.organization_id).await {
        Ok(ent) => Json(serde_json::json!({ "entitlements": ent })).into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

pub async fn get_billing_contact(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    match crate::store::get_account_bundle(&state.db, &ctx.user.organization_id, &ctx.user).await {
        Ok(bundle) => {
            Json(serde_json::json!({ "contact": bundle.billing_contact })).into_response()
        }
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct BillingContactPatch {
    pub email: Option<String>,
    pub company_name: Option<String>,
    pub tax_id: Option<String>,
}

pub async fn patch_billing_contact(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Json(body): Json<BillingContactPatch>,
) -> impl IntoResponse {
    let current =
        match crate::store::get_account_bundle(&state.db, &ctx.user.organization_id, &ctx.user)
            .await
        {
            Ok(b) => b.billing_contact,
            Err(e) => {
                return json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string())
                    .into_response()
            }
        };
    match crate::store::update_billing_contact(
        &state.db,
        &ctx.user.organization_id,
        body.email.as_deref().unwrap_or(&current.email),
        body.company_name
            .as_deref()
            .unwrap_or(&current.company_name),
        body.tax_id.as_deref().unwrap_or(&current.tax_id),
    )
    .await
    {
        Ok(contact) => Json(serde_json::json!({ "contact": contact })).into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

pub async fn list_invoices(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    match crate::store::get_account_bundle(&state.db, &ctx.user.organization_id, &ctx.user).await {
        Ok(bundle) => Json(serde_json::json!({ "invoices": bundle.invoices })).into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

pub async fn get_org(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    match crate::store::get_account_bundle(&state.db, &ctx.user.organization_id, &ctx.user).await {
        Ok(bundle) => {
            Json(serde_json::json!({ "organization": bundle.organization })).into_response()
        }
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

pub async fn list_members(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    match crate::store::list_org_members(&state.db, &ctx.user.organization_id).await {
        Ok(members) => Json(serde_json::json!({ "members": members })).into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

pub async fn list_api_keys(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    match crate::store::list_api_keys(&state.db, &ctx.user.organization_id).await {
        Ok(keys) => Json(serde_json::json!({ "keys": keys })).into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct CreateKeyBody {
    pub name: String,
    pub expires_days: Option<i64>,
}

pub async fn create_api_key(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Json(body): Json<CreateKeyBody>,
) -> impl IntoResponse {
    match crate::store::create_api_key(
        &state.db,
        &ctx.user.organization_id,
        &body.name,
        body.expires_days,
    )
    .await
    {
        Ok((key, plaintext)) => Json(serde_json::json!({
            "key": key,
            "plaintext": plaintext,
        }))
        .into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, &e.to_string()).into_response(),
    }
}

pub async fn revoke_api_key(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path(key_id): Path<String>,
) -> impl IntoResponse {
    match crate::store::revoke_api_key(&state.db, &ctx.user.organization_id, &key_id).await {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct DeviceLinkStartBody {
    pub device_name: Option<String>,
}

pub async fn device_link_start(
    State(state): State<AppState>,
    Json(body): Json<DeviceLinkStartBody>,
) -> impl IntoResponse {
    match crate::devices::start_device_link(
        &state.db,
        body.device_name.as_deref(),
        &state.config.portal_url,
    )
    .await
    {
        Ok(link) => Json(serde_json::json!({
            "device_code": link.device_code,
            "user_code": link.user_code,
            "verification_uri": link.verification_uri,
            "expires_in": link.expires_in,
            "interval": link.interval,
            "deep_link": format!("anycode://link?code={}", link.device_code),
        }))
        .into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, &e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct DeviceLinkApproveBody {
    pub device_code: String,
}

pub async fn device_link_approve(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Json(body): Json<DeviceLinkApproveBody>,
) -> impl IntoResponse {
    let identity_status: Option<String> =
        sqlx::query_scalar("SELECT identity_status FROM users WHERE id = ?")
            .bind(&ctx.user.id)
            .fetch_optional(state.db.pool())
            .await
            .ok()
            .flatten();
    if identity_status.as_deref() != Some("approved") {
        return json_error(StatusCode::FORBIDDEN, "verified cloud identity required")
            .into_response();
    }
    match crate::devices::approve_device_link(&state.db, &ctx.user.id, &body.device_code).await {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, &e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct DeviceLinkPollBody {
    pub device_code: String,
}

pub async fn device_link_poll(
    State(state): State<AppState>,
    Json(body): Json<DeviceLinkPollBody>,
) -> impl IntoResponse {
    match crate::devices::poll_device_link(&state.db, &body.device_code).await {
        Ok(Some(tokens)) => Json(serde_json::json!({
            "access_token": tokens.access_token,
            "refresh_token": tokens.refresh_token,
            "user": tokens.user,
            "entitlements": tokens.entitlements,
            "gateway_url": state.config.model_gateway_url,
            "authenticated": true,
        }))
        .into_response(),
        Ok(None) => (
            StatusCode::ACCEPTED,
            Json(serde_json::json!({ "pending": true })),
        )
            .into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, &e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct DeviceRefreshBody {
    pub refresh_token: String,
}

pub async fn device_refresh(
    State(state): State<AppState>,
    Json(body): Json<DeviceRefreshBody>,
) -> impl IntoResponse {
    match crate::devices::refresh_device_session(&state.db, &body.refresh_token).await {
        Ok(Some(tokens)) => Json(serde_json::json!({
            "access_token": tokens.access_token,
            "refresh_token": tokens.refresh_token,
            "user": tokens.user,
            "entitlements": tokens.entitlements,
            "gateway_url": state.config.model_gateway_url,
        }))
        .into_response(),
        Ok(None) => json_error(StatusCode::UNAUTHORIZED, "invalid refresh token").into_response(),
        Err(e) if e.to_string().contains("refresh token reuse detected") => json_error(
            StatusCode::UNAUTHORIZED,
            "refresh token reuse detected; device session revoked",
        )
        .into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

pub async fn list_devices(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    match crate::devices::list_linked_devices(&state.db, &ctx.user.id).await {
        Ok(devices) => Json(serde_json::json!({ "devices": devices })).into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

pub async fn revoke_device(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path(device_id): Path<String>,
) -> impl IntoResponse {
    match crate::devices::revoke_linked_device(&state.db, &ctx.user.id, &device_id).await {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct CheckoutBody {
    pub plan: String,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub cycle: Option<String>,
}

pub async fn billing_checkout(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Json(body): Json<CheckoutBody>,
) -> impl IntoResponse {
    let provider = body
        .provider
        .as_deref()
        .unwrap_or(state.config.default_payment_provider.as_str());
    let cycle = if body.plan == "cloud_5h" {
        "monthly"
    } else {
        body.cycle.as_deref().unwrap_or("monthly")
    };

    if body.plan == "cloud_5h" && provider == "stripe" {
        return json_error(
            StatusCode::BAD_REQUEST,
            "cloud_5h pass is only available via WeChat Pay",
        )
        .into_response();
    }

    match provider {
        "wechat" => {
            match crate::billing_wechat::create_native_order(
                &state.config,
                &state.db,
                &ctx.user.organization_id,
                &body.plan,
                cycle,
            )
            .await
            {
                Ok(order) => Json(serde_json::json!({
                    "provider": "wechat",
                    "order": order,
                }))
                .into_response(),
                Err(e) => json_error(StatusCode::BAD_REQUEST, &e.to_string()).into_response(),
            }
        }
        "stripe" => match crate::billing_stripe::create_checkout_session(
            &state.config,
            &state.db,
            &ctx.user.organization_id,
            &body.plan,
            &ctx.user.email,
        )
        .await
        {
            Ok(url) => Json(serde_json::json!({
                "provider": "stripe",
                "checkout_url": url,
            }))
            .into_response(),
            Err(e) => json_error(StatusCode::BAD_REQUEST, &e.to_string()).into_response(),
        },
        _ => json_error(StatusCode::BAD_REQUEST, "unsupported payment provider").into_response(),
    }
}

pub async fn get_payment_order(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path(order_id): Path<String>,
) -> impl IntoResponse {
    match crate::billing::get_payment_order(&state.db, &ctx.user.organization_id, &order_id).await {
        Ok(Some(order)) => Json(serde_json::json!({ "order": order })).into_response(),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "order not found").into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

/// Ask WeChat for payment status and fulfill if already paid (callback miss recovery).
pub async fn sync_payment_order(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path(order_id): Path<String>,
) -> impl IntoResponse {
    let order = match crate::billing::get_payment_order(
        &state.db,
        &ctx.user.organization_id,
        &order_id,
    )
    .await
    {
        Ok(Some(o)) => o,
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "order not found").into_response(),
        Err(e) => {
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response()
        }
    };
    if order.status == "paid" {
        return Json(serde_json::json!({ "order": order, "synced": false })).into_response();
    }
    if order.provider != "wechat" {
        return json_error(StatusCode::BAD_REQUEST, "only wechat orders can sync").into_response();
    }
    let Some(out_trade_no) = order.out_trade_no.as_deref() else {
        return json_error(StatusCode::BAD_REQUEST, "order missing out_trade_no").into_response();
    };
    match crate::billing_wechat::sync_order_by_out_trade_no(
        &state.config,
        &state.db,
        out_trade_no,
    )
    .await
    {
        Ok(synced) => {
            let refreshed = crate::billing::get_payment_order(
                &state.db,
                &ctx.user.organization_id,
                &order_id,
            )
            .await
            .ok()
            .flatten()
            .unwrap_or(order);
            Json(serde_json::json!({ "order": refreshed, "synced": synced })).into_response()
        }
        Err(e) => json_error(StatusCode::BAD_REQUEST, &e.to_string()).into_response(),
    }
}

pub async fn wechat_webhook(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let payload = String::from_utf8_lossy(&body);
    match crate::billing_wechat::handle_wechat_notify(&state.config, &state.db, &headers, &payload)
        .await
    {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({ "code": "SUCCESS", "message": "成功" })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("wechat notify error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "code": "FAIL", "message": e.to_string() })),
            )
                .into_response()
        }
    }
}

pub async fn stripe_webhook(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let payload = String::from_utf8_lossy(&body);
    let sig = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    match crate::billing_stripe::handle_stripe_webhook(&state.config, &state.db, &payload, sig)
        .await
    {
        Ok(()) => Json(serde_json::json!({ "received": true })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, &e.to_string()).into_response(),
    }
}

pub async fn models_catalog(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let org = match crate::store::get_subscription(&state.db, &ctx.user.organization_id).await {
        Ok(s) => s.plan,
        Err(e) => {
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response()
        }
    };
    match crate::models_catalog::list_models(&state.db, &org).await {
        Ok(models) => Json(serde_json::json!({ "models": models })).into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

pub async fn usage_summary(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    match crate::usage::usage_summary(&state.db, &ctx.user.organization_id).await {
        Ok(summary) => Json(serde_json::json!({ "usage": summary })).into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

pub async fn plans_catalog(State(state): State<AppState>) -> impl IntoResponse {
    match crate::plan::list_plans(&state.db, true).await {
        Ok(plans) => {
            let _ = crate::plan::refresh_plan_cache(&state.db).await;
            Json(serde_json::json!({ "plans": plans })).into_response()
        }
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct GatewayAuthBody {
    pub api_key: Option<String>,
    pub access_token: Option<String>,
    pub model_id: String,
    #[serde(default)]
    pub exclude_account_ids: Vec<String>,
}

pub async fn gateway_authorize(
    State(state): State<AppState>,
    Json(body): Json<GatewayAuthBody>,
) -> impl IntoResponse {
    let org_id = if let Some(key) = body.api_key.as_deref() {
        crate::usage::resolve_org_by_api_key(&state.db, key)
            .await
            .ok()
            .flatten()
    } else if let Some(tok) = body.access_token.as_deref() {
        crate::usage::resolve_org_by_session(&state.db, tok)
            .await
            .ok()
            .flatten()
    } else {
        None
    };
    let Some(org_id) = org_id else {
        return json_error(StatusCode::UNAUTHORIZED, "invalid credentials").into_response();
    };
    if !crate::store::organization_has_verified_identity(&state.db, &org_id)
        .await
        .unwrap_or(false)
    {
        return json_error(StatusCode::FORBIDDEN, "verified cloud identity required")
            .into_response();
    }
    let sub = match crate::store::get_subscription(&state.db, &org_id).await {
        Ok(s) => s,
        Err(e) => {
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response()
        }
    };
    let ent = match crate::store::get_entitlements(&state.db, &org_id).await {
        Ok(e) => e,
        Err(e) => {
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response()
        }
    };
    if !ent.hosted_models_enabled {
        return json_error(StatusCode::FORBIDDEN, "hosted models not enabled for plan")
            .into_response();
    }
    if body.model_id != "auto" && !crate::usage::is_allowed_hosted_model(&body.model_id) {
        return json_error(StatusCode::BAD_REQUEST, "model not supported").into_response();
    }
    if let Err(e) = crate::quota::check_call_quota(&state.db, &org_id).await {
        return json_error(StatusCode::TOO_MANY_REQUESTS, &e.to_string()).into_response();
    }
    let models = match crate::models_catalog::list_models(&state.db, &sub.plan).await {
        Ok(m) => m,
        Err(e) => {
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response()
        }
    };
    let allowed = if body.model_id == "auto" {
        models.iter().any(|m| m.id == "auto" && m.available)
    } else {
        models.iter().any(|m| m.id == body.model_id && m.available)
    };
    if !allowed {
        return json_error(StatusCode::FORBIDDEN, "model not available for plan").into_response();
    }
    let resolved_model_id =
        match crate::usage::resolve_model_id(&state.db, &sub.plan, &body.model_id).await {
            Ok(id) => id,
            Err(e) => return json_error(StatusCode::FORBIDDEN, &e.to_string()).into_response(),
        };
    let upstream = match crate::models_catalog::get_model_upstream(&state.db, &resolved_model_id)
        .await
    {
        Ok(u) => u,
        Err(e) => {
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response()
        }
    };
    let Some((provider_id, upstream_model)) = upstream else {
        return json_error(StatusCode::NOT_FOUND, "model not found").into_response();
    };

    let mut auth_json = serde_json::json!({
        "organization_id": org_id,
        "provider_id": provider_id,
        "upstream_model": upstream_model,
        "requested_model_id": body.model_id,
        "resolved_model_id": resolved_model_id,
        "token_limit": ent.token_limit,
        "tokens_used": ent.tokens_used,
        "calls_remaining": ent.calls_remaining,
        "quota_resets_at": ent.quota_resets_at,
    });

    if let Some(secret) = state.config.upstream_key_encryption_secret.as_deref() {
        match crate::upstream_pool::select_upstream_credential(
            &state.db,
            secret,
            &provider_id,
            &body.exclude_account_ids,
        )
        .await
        {
            Ok(Some(cred)) => {
                auth_json["upstream_account_id"] = cred.account_id.into();
                auth_json["upstream_api_key"] = cred.api_key.into();
                auth_json["upstream_base_url"] = cred.base_url.into();
            }
            Ok(None) => {
                return json_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "no available upstream account in pool",
                )
                .into_response();
            }
            Err(e) => {
                return json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string())
                    .into_response()
            }
        }
    }

    Json(auth_json).into_response()
}

#[derive(Deserialize)]
pub struct GatewayUsageBody {
    pub api_key: Option<String>,
    pub access_token: Option<String>,
    pub model_id: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub upstream_account_id: Option<String>,
}

pub async fn gateway_usage(
    State(state): State<AppState>,
    Json(body): Json<GatewayUsageBody>,
) -> impl IntoResponse {
    let org_id = if let Some(key) = body.api_key.as_deref() {
        crate::usage::resolve_org_by_api_key(&state.db, key)
            .await
            .ok()
            .flatten()
    } else if let Some(tok) = body.access_token.as_deref() {
        crate::usage::resolve_org_by_session(&state.db, tok)
            .await
            .ok()
            .flatten()
    } else {
        None
    };
    let Some(org_id) = org_id else {
        return json_error(StatusCode::UNAUTHORIZED, "invalid credentials").into_response();
    };
    match crate::usage::record_usage(
        &state.db,
        &org_id,
        &body.model_id,
        body.prompt_tokens,
        body.completion_tokens,
        body.upstream_account_id.as_deref(),
    )
    .await
    {
        Ok(()) => {
            if let Some(account_id) = body.upstream_account_id.as_deref() {
                let _ = crate::upstream_pool::record_upstream_success(
                    &state.db,
                    account_id,
                    body.prompt_tokens,
                    body.completion_tokens,
                )
                .await;
            }
            Json(serde_json::json!({ "ok": true })).into_response()
        }
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct GatewayUpstreamFailureBody {
    pub upstream_account_id: String,
    pub status_code: Option<i32>,
    pub message: Option<String>,
}

pub async fn gateway_upstream_failure(
    State(state): State<AppState>,
    Json(body): Json<GatewayUpstreamFailureBody>,
) -> impl IntoResponse {
    match crate::upstream_pool::record_upstream_failure(
        &state.db,
        &body.upstream_account_id,
        body.status_code,
        body.message.as_deref().unwrap_or("upstream error"),
    )
    .await
    {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}
