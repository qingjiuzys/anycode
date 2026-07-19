use crate::api::{json_error, AppState};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde::Deserialize;
use sqlx::Row;

#[derive(Clone)]
pub struct AdminContext {
    pub user_id: String,
    pub email: String,
    pub role: String,
    pub token: String,
}

#[derive(Deserialize)]
pub struct AdminLoginBody {
    pub email: String,
    pub password: String,
}

pub async fn admin_login(
    State(state): State<AppState>,
    Json(body): Json<AdminLoginBody>,
) -> impl IntoResponse {
    match crate::admin::admin_login(&state.db, &body.email, &body.password).await {
        Ok(Some(token)) => Json(serde_json::json!({
            "token": token,
            "authenticated": true,
        }))
        .into_response(),
        Ok(None) => json_error(StatusCode::UNAUTHORIZED, "invalid credentials").into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

pub async fn admin_me(Extension(ctx): Extension<AdminContext>) -> impl IntoResponse {
    Json(serde_json::json!({
        "user": {
            "id": ctx.user_id,
            "email": ctx.email,
            "role": ctx.role,
        },
        "authenticated": true,
    }))
}

pub async fn admin_list_upstream_accounts(
    State(state): State<AppState>,
    Extension(ctx): Extension<AdminContext>,
) -> impl IntoResponse {
    let _ = ctx;
    match crate::upstream_pool::list_upstream_accounts(&state.db, None).await {
        Ok(accounts) => Json(serde_json::json!({ "accounts": accounts })).into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct CreateUpstreamAccountBody {
    pub provider_id: Option<String>,
    pub name: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub weight: Option<i32>,
}

pub async fn admin_create_upstream_account(
    State(state): State<AppState>,
    Extension(ctx): Extension<AdminContext>,
    Json(body): Json<CreateUpstreamAccountBody>,
) -> impl IntoResponse {
    let secret = match state.config.upstream_key_encryption_secret.as_deref() {
        Some(s) => s,
        None => {
            return json_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "UPSTREAM_KEY_ENCRYPTION_SECRET not configured",
            )
            .into_response()
        }
    };
    let provider = body.provider_id.as_deref().unwrap_or("agnes");
    match crate::upstream_pool::create_upstream_account(
        &state.db,
        secret,
        provider,
        &body.name,
        &body.api_key,
        body.base_url.as_deref(),
        body.weight.unwrap_or(100),
    )
    .await
    {
        Ok(account) => {
            let _ = crate::admin::write_audit_log(
                &state.db,
                &ctx.user_id,
                "create",
                "upstream_account",
                Some(&account.id),
                None,
            )
            .await;
            Json(serde_json::json!({ "account": account })).into_response()
        }
        Err(e) => json_error(StatusCode::BAD_REQUEST, &e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct PatchUpstreamAccountBody {
    pub status: Option<String>,
    pub weight: Option<i32>,
}

pub async fn admin_patch_upstream_account(
    State(state): State<AppState>,
    Extension(ctx): Extension<AdminContext>,
    axum::extract::Path(account_id): axum::extract::Path<String>,
    Json(body): Json<PatchUpstreamAccountBody>,
) -> impl IntoResponse {
    if let Some(status) = body.status.as_deref() {
        if let Err(e) =
            crate::upstream_pool::update_upstream_account_status(&state.db, &account_id, status)
                .await
        {
            return json_error(StatusCode::BAD_REQUEST, &e.to_string()).into_response();
        }
    }
    if let Some(weight) = body.weight {
        if let Err(e) =
            crate::upstream_pool::update_upstream_account_weight(&state.db, &account_id, weight)
                .await
        {
            return json_error(StatusCode::BAD_REQUEST, &e.to_string()).into_response();
        }
    }
    let _ = crate::admin::write_audit_log(
        &state.db,
        &ctx.user_id,
        "update",
        "upstream_account",
        Some(&account_id),
        Some(serde_json::json!({
            "status": body.status,
            "weight": body.weight,
        })),
    )
    .await;
    Json(serde_json::json!({ "ok": true })).into_response()
}

pub async fn admin_list_health_events(
    State(state): State<AppState>,
    Extension(_ctx): Extension<AdminContext>,
) -> impl IntoResponse {
    match crate::upstream_pool::list_health_events(&state.db, None, 100).await {
        Ok(events) => Json(serde_json::json!({ "events": events })).into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

pub async fn admin_list_models(
    State(state): State<AppState>,
    Extension(_ctx): Extension<AdminContext>,
) -> impl IntoResponse {
    match crate::models_catalog::list_models(&state.db, "team").await {
        Ok(models) => Json(serde_json::json!({ "models": models })).into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

pub async fn admin_usage_overview(
    State(state): State<AppState>,
    Extension(_ctx): Extension<AdminContext>,
) -> impl IntoResponse {
    let rows = match sqlx::query(
        r#"
        SELECT organization_id, CAST(SUM(total_tokens) AS SIGNED) AS total_tokens
        FROM usage_events
        WHERE created_at >= DATE_SUB(NOW(), INTERVAL 30 DAY)
        GROUP BY organization_id
        ORDER BY total_tokens DESC
        LIMIT 50
        "#,
    )
    .fetch_all(state.db.pool())
    .await
    {
        Ok(r) => r,
        Err(e) => {
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response()
        }
    };
    let usage: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "organization_id": r.get::<String, _>("organization_id"),
                "total_tokens": r.get::<i64, _>("total_tokens"),
            })
        })
        .collect();
    Json(serde_json::json!({ "usage": usage })).into_response()
}

pub async fn admin_list_identity_reviews(
    State(state): State<AppState>,
    Extension(_ctx): Extension<AdminContext>,
) -> impl IntoResponse {
    let rows = match sqlx::query(
        r#"
        SELECT r.id, r.user_id, u.email, r.status, r.id_number_last4,
               r.submitted_at, r.reviewed_at, r.rejection_reason
        FROM identity_reviews r JOIN users u ON u.id = r.user_id
        ORDER BY FIELD(r.status, 'pending', 'rejected', 'approved'), r.submitted_at ASC
        LIMIT 200
        "#,
    )
    .fetch_all(state.db.pool())
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response()
        }
    };
    let reviews: Vec<_> = rows
        .into_iter()
        .map(|r| {
            let submitted: chrono::DateTime<chrono::Utc> = r.get("submitted_at");
            let reviewed: Option<chrono::DateTime<chrono::Utc>> = r.get("reviewed_at");
            serde_json::json!({
                "id": r.get::<String, _>("id"),
                "user_id": r.get::<String, _>("user_id"),
                "email": r.get::<String, _>("email"),
                "status": r.get::<String, _>("status"),
                "id_number_masked": format!("**************{}", r.get::<String, _>("id_number_last4")),
                "submitted_at": submitted.to_rfc3339(),
                "reviewed_at": reviewed.map(|v| v.to_rfc3339()),
                "rejection_reason": r.get::<Option<String>, _>("rejection_reason"),
                "document_upload_supported": false
            })
        })
        .collect();
    Json(serde_json::json!({ "reviews": reviews })).into_response()
}

#[derive(Deserialize)]
pub struct RejectIdentityBody {
    pub reason: String,
}

#[derive(Deserialize)]
pub struct RevealIdentityBody {
    pub purpose: String,
}

pub async fn admin_reveal_identity(
    State(state): State<AppState>,
    Extension(ctx): Extension<AdminContext>,
    axum::extract::Path(review_id): axum::extract::Path<String>,
    Json(body): Json<RevealIdentityBody>,
) -> impl IntoResponse {
    if body.purpose.trim().len() < 4 {
        return json_error(StatusCode::BAD_REQUEST, "review purpose required").into_response();
    }
    let Some(secret) = state.config.identity_encryption_secret.as_deref() else {
        return json_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "identity service unavailable",
        )
        .into_response();
    };
    let row = match sqlx::query(
        "SELECT legal_name_ciphertext, legal_name_nonce, id_number_ciphertext, id_number_nonce FROM identity_reviews WHERE id = ?",
    )
    .bind(&review_id)
    .fetch_optional(state.db.pool())
    .await
    {
        Ok(Some(row)) => row,
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "review not found").into_response(),
        Err(e) => {
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response()
        }
    };
    let legal_name = match crate::crypto::decrypt_secret(
        row.get("legal_name_ciphertext"),
        row.get("legal_name_nonce"),
        secret,
    ) {
        Ok(value) => value,
        Err(_) => {
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "decrypt identity failed")
                .into_response()
        }
    };
    let id_number = match crate::crypto::decrypt_secret(
        row.get("id_number_ciphertext"),
        row.get("id_number_nonce"),
        secret,
    ) {
        Ok(value) => value,
        Err(_) => {
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "decrypt identity failed")
                .into_response()
        }
    };
    let _ = sqlx::query(
        "INSERT INTO audit_access_logs (id, admin_user_id, action, resource_type, resource_id, purpose) VALUES (?, ?, 'reveal', 'identity_review', ?, ?)",
    )
    .bind(format!("aacc_{}", uuid::Uuid::new_v4()))
    .bind(&ctx.user_id)
    .bind(&review_id)
    .bind(body.purpose.trim())
    .execute(state.db.pool())
    .await;
    Json(serde_json::json!({ "legal_name": legal_name, "id_number": id_number })).into_response()
}

pub async fn admin_approve_identity(
    State(state): State<AppState>,
    Extension(ctx): Extension<AdminContext>,
    axum::extract::Path(review_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    match crate::identity::review(&state.db, &ctx.user_id, &review_id, true, None).await {
        Ok(()) => {
            let _ = crate::admin::write_audit_log(
                &state.db,
                &ctx.user_id,
                "approve",
                "identity_review",
                Some(&review_id),
                None,
            )
            .await;
            Json(serde_json::json!({ "ok": true })).into_response()
        }
        Err(e) => json_error(StatusCode::BAD_REQUEST, &e.to_string()).into_response(),
    }
}

pub async fn admin_reject_identity(
    State(state): State<AppState>,
    Extension(ctx): Extension<AdminContext>,
    axum::extract::Path(review_id): axum::extract::Path<String>,
    Json(body): Json<RejectIdentityBody>,
) -> impl IntoResponse {
    match crate::identity::review(
        &state.db,
        &ctx.user_id,
        &review_id,
        false,
        Some(&body.reason),
    )
    .await
    {
        Ok(()) => {
            let _ = crate::admin::write_audit_log(
                &state.db,
                &ctx.user_id,
                "reject",
                "identity_review",
                Some(&review_id),
                Some(serde_json::json!({ "reason": body.reason })),
            )
            .await;
            Json(serde_json::json!({ "ok": true })).into_response()
        }
        Err(e) => json_error(StatusCode::BAD_REQUEST, &e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct CreateAuditRuleBody {
    pub name: String,
    pub keyword: String,
    pub severity: Option<String>,
}

pub async fn admin_list_audit_rules(
    State(state): State<AppState>,
    Extension(_ctx): Extension<AdminContext>,
) -> impl IntoResponse {
    match sqlx::query(
        "SELECT id, name, keyword, severity, enabled, created_at FROM audit_keyword_rules ORDER BY created_at DESC",
    )
    .fetch_all(state.db.pool())
    .await
    {
        Ok(rows) => {
            let rules: Vec<_> = rows
                .into_iter()
                .map(|row| {
                    serde_json::json!({
                        "id": row.get::<String, _>("id"),
                        "name": row.get::<String, _>("name"),
                        "keyword": row.get::<String, _>("keyword"),
                        "severity": row.get::<String, _>("severity"),
                        "enabled": row.get::<bool, _>("enabled")
                    })
                })
                .collect();
            Json(serde_json::json!({ "rules": rules })).into_response()
        }
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

pub async fn admin_create_audit_rule(
    State(state): State<AppState>,
    Extension(ctx): Extension<AdminContext>,
    Json(body): Json<CreateAuditRuleBody>,
) -> impl IntoResponse {
    if body.name.trim().is_empty() || body.keyword.trim().is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "name and keyword required").into_response();
    }
    let severity = body.severity.as_deref().unwrap_or("review");
    if !matches!(severity, "review" | "high" | "critical") {
        return json_error(StatusCode::BAD_REQUEST, "invalid severity").into_response();
    }
    let id = format!("arule_{}", uuid::Uuid::new_v4());
    match sqlx::query(
        "INSERT INTO audit_keyword_rules (id, name, keyword, severity, created_by_admin_id) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(body.name.trim())
    .bind(body.keyword.trim())
    .bind(severity)
    .bind(&ctx.user_id)
    .execute(state.db.pool())
    .await
    {
        Ok(_) => Json(serde_json::json!({ "ok": true, "id": id })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, &e.to_string()).into_response(),
    }
}

pub async fn admin_purge_audit(
    State(state): State<AppState>,
    Extension(ctx): Extension<AdminContext>,
) -> impl IntoResponse {
    match crate::conversation_audit::purge_expired(&state.db).await {
        Ok(deleted_conversations) => {
            let _ = crate::admin::write_audit_log(
                &state.db,
                &ctx.user_id,
                "purge_expired",
                "conversation_audit",
                None,
                Some(serde_json::json!({ "deleted_conversations": deleted_conversations })),
            )
            .await;
            Json(serde_json::json!({ "ok": true, "deleted_conversations": deleted_conversations }))
                .into_response()
        }
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

pub async fn admin_list_plans(
    State(state): State<AppState>,
    Extension(_ctx): Extension<AdminContext>,
) -> impl IntoResponse {
    match crate::plan::list_plans(&state.db, false).await {
        Ok(plans) => Json(serde_json::json!({ "plans": plans })).into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct AdminPatchPlanBody {
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub monthly_price_fen: Option<i32>,
    pub yearly_price_fen: Option<i32>,
    pub token_limit: Option<i64>,
    pub api_key_limit: Option<i32>,
    pub seat_limit: Option<i32>,
    pub quota_window_secs: Option<i32>,
    pub calls_per_window: Option<i32>,
    pub hosted_models_enabled: Option<bool>,
    pub promo_label: Option<Option<String>>,
    pub featured: Option<bool>,
    pub enabled: Option<bool>,
    pub sort_order: Option<i32>,
}

pub async fn admin_patch_plan(
    State(state): State<AppState>,
    Extension(ctx): Extension<AdminContext>,
    axum::extract::Path(plan_id): axum::extract::Path<String>,
    Json(body): Json<AdminPatchPlanBody>,
) -> impl IntoResponse {
    let patch = crate::plan::PatchPlanBody {
        display_name: body.display_name,
        description: body.description,
        monthly_price_fen: body.monthly_price_fen,
        yearly_price_fen: body.yearly_price_fen,
        token_limit: body.token_limit,
        api_key_limit: body.api_key_limit,
        seat_limit: body.seat_limit,
        quota_window_secs: body.quota_window_secs,
        calls_per_window: body.calls_per_window,
        hosted_models_enabled: body.hosted_models_enabled,
        promo_label: body.promo_label,
        featured: body.featured,
        enabled: body.enabled,
        sort_order: body.sort_order,
    };
    match crate::plan::patch_plan(&state.db, &plan_id, &patch).await {
        Ok(plan) => {
            let _ = crate::admin::write_audit_log(
                &state.db,
                &ctx.user_id,
                "update",
                "cloud_plan",
                Some(&plan_id),
                Some(serde_json::json!({ "plan_id": plan_id })),
            )
            .await;
            Json(serde_json::json!({ "plan": plan })).into_response()
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("not found") {
                json_error(StatusCode::NOT_FOUND, &msg).into_response()
            } else {
                json_error(StatusCode::INTERNAL_SERVER_ERROR, &msg).into_response()
            }
        }
    }
}
