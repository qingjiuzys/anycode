use crate::config::ServiceConfig;
use crate::db::AccountDb;
use crate::models::AuthUser;
use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
    Json, Router,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: AccountDb,
    pub version: String,
    pub config: Arc<ServiceConfig>,
}

pub mod handlers;

use handlers::admin::AdminContext;

#[derive(Clone)]
pub struct AuthContext {
    pub user: AuthUser,
    pub token: String,
}

pub fn router(state: AppState) -> Router {
    let public = Router::new()
        .route(
            "/auth/email/send-code",
            post(handlers::auth_send_email_code),
        )
        .route("/auth/register", post(handlers::auth_register))
        .route("/auth/login", post(handlers::auth_login))
        .route("/devices/link/start", post(handlers::device_link_start))
        .route("/devices/link/poll", post(handlers::device_link_poll))
        .route("/devices/refresh", post(handlers::device_refresh))
        .route("/billing/webhooks/stripe", post(handlers::stripe_webhook))
        .route("/billing/webhooks/wechat", post(handlers::wechat_webhook))
        .route("/plans/catalog", get(handlers::plans_catalog))
        .route("/gateway/authorize", post(handlers::gateway_authorize))
        .route("/gateway/usage", post(handlers::gateway_usage))
        .route(
            "/gateway/upstream-failure",
            post(handlers::gateway_upstream_failure),
        );

    let admin_public = Router::new().route("/admin/login", post(handlers::admin::admin_login));

    let admin_authed = Router::new()
        .route("/admin/me", get(handlers::admin::admin_me))
        .route(
            "/admin/upstream-accounts",
            get(handlers::admin::admin_list_upstream_accounts)
                .post(handlers::admin::admin_create_upstream_account),
        )
        .route(
            "/admin/upstream-accounts/{account_id}",
            axum::routing::patch(handlers::admin::admin_patch_upstream_account),
        )
        .route(
            "/admin/health-events",
            get(handlers::admin::admin_list_health_events),
        )
        .route("/admin/models", get(handlers::admin::admin_list_models))
        .route(
            "/admin/usage-overview",
            get(handlers::admin::admin_usage_overview),
        )
        .route(
            "/admin/identity-reviews",
            get(handlers::admin::admin_list_identity_reviews),
        )
        .route(
            "/admin/identity-reviews/{review_id}/approve",
            post(handlers::admin::admin_approve_identity),
        )
        .route(
            "/admin/identity-reviews/{review_id}/reject",
            post(handlers::admin::admin_reject_identity),
        )
        .route(
            "/admin/identity-reviews/{review_id}/reveal",
            post(handlers::admin::admin_reveal_identity),
        )
        .route(
            "/admin/audit/rules",
            get(handlers::admin::admin_list_audit_rules)
                .post(handlers::admin::admin_create_audit_rule),
        )
        .route(
            "/admin/audit/purge",
            post(handlers::admin::admin_purge_audit),
        )
        .route("/admin/plans", get(handlers::admin::admin_list_plans))
        .route(
            "/admin/plans/{plan_id}",
            axum::routing::patch(handlers::admin::admin_patch_plan),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_admin_auth,
        ));

    let authed = Router::new()
        .route("/auth/me", get(handlers::auth_me))
        .route("/auth/logout", post(handlers::auth_logout))
        .route("/account/identity", get(handlers::identity_status))
        .route("/account/identity/submit", post(handlers::identity_submit))
        .route("/audit/messages", post(handlers::audit_ingest))
        .route("/account/subscription", get(handlers::get_subscription))
        .route(
            "/account/subscription/upgrade",
            post(handlers::upgrade_subscription),
        )
        .route("/account/entitlements", get(handlers::get_entitlements))
        .route("/account/bundle", get(handlers::get_account_bundle))
        .route("/account/profile", patch(handlers::patch_account_profile))
        .route(
            "/account/billing/contact",
            get(handlers::get_billing_contact).patch(handlers::patch_billing_contact),
        )
        .route("/account/invoices", get(handlers::list_invoices))
        .route(
            "/account/api-keys",
            get(handlers::list_api_keys).post(handlers::create_api_key),
        )
        .route(
            "/account/api-keys/{key_id}",
            delete(handlers::revoke_api_key),
        )
        .route("/org", get(handlers::get_org))
        .route("/org/members", get(handlers::list_members))
        .route("/devices/link/approve", post(handlers::device_link_approve))
        .route("/devices", get(handlers::list_devices))
        .route("/devices/{device_id}", delete(handlers::revoke_device))
        .route("/billing/checkout", post(handlers::billing_checkout))
        .route(
            "/billing/orders/{order_id}",
            get(handlers::get_payment_order),
        )
        .route("/models/catalog", get(handlers::models_catalog))
        .route("/usage/summary", get(handlers::usage_summary))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_auth));

    let api = Router::new()
        .route("/health", get(handlers::health))
        .nest(
            "/api/v1",
            public.merge(authed).merge(admin_public).merge(admin_authed),
        )
        .with_state(state.clone());

    let mut app = api;
    if let Some(portal_dir) = state.config.portal_dir.clone() {
        app = app.merge(crate::portal::portal_router(portal_dir));
    }
    if let Some(ops_dir) = state.config.ops_portal_dir.clone() {
        app = app.nest("/ops", crate::portal::portal_router(ops_dir));
    }
    app
}

async fn require_admin_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = bearer_from_request(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    let user = crate::admin::resolve_admin_session(&state.db, &token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    req.extensions_mut().insert(AdminContext {
        user_id: user.id,
        email: user.email,
        role: user.role,
        token,
    });
    Ok(next.run(req).await)
}

async fn require_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = bearer_from_request(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    let user = crate::store::resolve_session(&state.db, &token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    req.extensions_mut().insert(AuthContext { user, token });
    Ok(next.run(req).await)
}

pub fn bearer_from_request(req: &Request) -> Option<String> {
    if let Some(h) = req.headers().get(header::AUTHORIZATION) {
        let s = h.to_str().ok()?;
        if let Some(rest) = s.strip_prefix("Bearer ") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

pub fn json_error(status: StatusCode, msg: &str) -> impl IntoResponse {
    (status, Json(serde_json::json!({ "error": msg })))
}
