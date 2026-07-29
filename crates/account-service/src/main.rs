use anycode_account_service::admin;
use anycode_account_service::api::{self, AppState};
use anycode_account_service::{AccountDb, ServiceConfig};
use anyhow::Result;
use axum::http::{HeaderValue, Method};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "anycode_account_service=info,tower_http=info".into()),
        )
        .init();

    let config = ServiceConfig::from_env()?;
    let db = AccountDb::connect(&config.database_url).await?;
    db.migrate().await?;
    admin::bootstrap_admin_if_needed(
        &db,
        config.admin_bootstrap_email.as_deref(),
        config.admin_bootstrap_password.as_deref(),
    )
    .await?;
    anycode_account_service::store::bootstrap_portal_user_if_needed(
        &db,
        config.admin_bootstrap_email.as_deref(),
        config.admin_bootstrap_password.as_deref(),
    )
    .await?;

    let _ = anycode_account_service::plan::refresh_plan_cache(&db).await;

    let state = AppState {
        db,
        version: env!("CARGO_PKG_VERSION").into(),
        config: Arc::new(config.clone()),
        a2a_relay: Arc::new(anycode_account_service::a2a::StreamRelay::new()),
    };

    let mut cors = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
        ]);

    let origins: Vec<HeaderValue> = config
        .cors_origins
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();
    if !origins.is_empty() {
        cors = cors.allow_origin(AllowOrigin::list(origins));
    }

    let app = api::router(state).layer(cors);

    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    if config.portal_dir.is_some() {
        info!(%addr, "anycode-account serving API + Account Portal");
    } else {
        info!(%addr, "anycode-account API only (set ACCOUNT_PORTAL_DIR for portal UI)");
    }
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
