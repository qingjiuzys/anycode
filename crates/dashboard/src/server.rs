use crate::api::{self, state::AppState};
use crate::auth_session::SessionStore;
use crate::db::DashboardDb;
use crate::events::EventBus;
use crate::skills_scan::sync_skills_to_db;
use anyhow::{Context, Result};
use axum::Router;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

#[derive(Debug, Clone)]
pub struct DashboardConfig {
    pub host: String,
    pub port: u16,
    pub db_path: PathBuf,
    pub static_dir: Option<PathBuf>,
    pub version: String,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 43_180,
            db_path: default_db_path(),
            static_dir: None,
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }
}

#[must_use]
pub fn default_db_path() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".anycode")
        .join("projects.db")
}

pub async fn run(config: DashboardConfig, workspace_paths: Vec<String>) -> Result<()> {
    let _ = anycode_setup::ensure_layout();
    if let Err(e) = crate::media_defaults::ensure_default_local_stt() {
        tracing::warn!(error = %e, "default local STT bootstrap skipped");
    }
    crate::control::task_trigger::init_default_anycode_bin();
    let db = DashboardDb::open(&config.db_path)
        .await
        .context("open dashboard database")?;
    let tasks_root = std::env::var("HOME")
        .map(|h| std::path::PathBuf::from(h).join(".anycode").join("tasks"))
        .unwrap_or_else(|_| std::path::PathBuf::from(".anycode/tasks"));
    if !workspace_paths.is_empty() {
        let n = db.sync_workspace_paths(&workspace_paths).await?;
        info!(count = n, "synced workspace projects");
    }
    if let Ok(stats) = db.overview_stats().await {
        if stats.projects_count == 0 && !workspace_paths.is_empty() {
            info!("empty database — auto-scanning workspace projects");
            let _ = db.sync_workspace_paths(&workspace_paths).await;
            let _ = sync_skills_to_db(&db, &workspace_paths).await;
        }
    }
    match sync_skills_to_db(&db, &workspace_paths).await {
        Ok(n) if n > 0 => info!(count = n, "synced local skills"),
        Ok(_) => {}
        Err(e) => tracing::warn!(error = %e, "skills scan skipped"),
    }
    let swept =
        crate::approval_ipc::sweep_stale_pending(crate::approval_ipc::STALE_PENDING_MAX_AGE_SECS);
    if swept > 0 {
        info!(count = swept, "swept stale pending tool approval files");
    }
    let swept_active = crate::cancel_ipc::sweep_stale_active();
    if swept_active > 0 {
        info!(
            count = swept_active,
            "swept stale active session registrations"
        );
    }
    if let Ok(running) = db.list_running_sessions(500).await {
        for session in running {
            if !crate::cancel_ipc::is_active(&session.id) {
                let _ = db.cancel_running_session(&session.id).await;
            }
        }
    }
    let _ = db.reconcile_local_services("dashboard").await;
    crate::local_service::terminate_live_dashboard_peers(&db, &config.host, config.port).await?;

    let started_at = chrono::Utc::now().to_rfc3339();
    if !crate::service_governance::is_loopback_host(&config.host) {
        let n = crate::tokens::token_count_active(&db).await.unwrap_or(0);
        let allow = std::env::var("ANYCODE_DASHBOARD_ALLOW_UNAUTH")
            .ok()
            .as_deref()
            == Some("1");
        if n == 0 && !allow {
            anyhow::bail!(
                "non-loopback dashboard requires at least one API token; run: anycode dashboard token create (or set ANYCODE_DASHBOARD_ALLOW_UNAUTH=1 for local dev)"
            );
        }
    }

    let static_dir = config
        .static_dir
        .or_else(crate::static_ui::discover_ui_dist);
    if static_dir.is_some() {
        info!("serving dashboard UI static files");
    }
    let events = Arc::new(EventBus::new());
    let state = AppState {
        db,
        events,
        sessions: SessionStore::default(),
        web_chat: crate::control::web_chat::WebChatHub::default(),
        version: config.version.clone(),
        static_dir,
        workspace_paths: workspace_paths.clone(),
        tasks_root: tasks_root.clone(),
        host: config.host.clone(),
        port: config.port,
        started_at: started_at.clone(),
        pid: std::process::id(),
    };
    let _ = crate::audit::record_audit(
        &state.db,
        crate::audit::AuditEventInput::low(
            "dashboard_started",
            serde_json::json!({ "host": config.host, "port": config.port }),
        ),
    )
    .await;
    if let Err(e) = crate::metrics::maybe_emit_blocked_threshold_alert(&state.db).await {
        tracing::warn!(error = %e, "blocked threshold alert skipped");
    }
    if let Ok(n) = state.db.sweep_stale_pending_sessions(5).await {
        if n > 0 {
            tracing::info!(count = n, "swept stale pending sessions");
        }
    }
    let db_backfill = state.db.clone();
    tokio::spawn(async move {
        match db_backfill.refresh_all_project_trust_scores().await {
            Ok(n) => tracing::debug!(count = n, "project trust scores backfilled"),
            Err(e) => tracing::warn!(error = %e, "project trust score backfill failed"),
        }
    });
    let app = api::router(state.clone());
    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .context("parse listen address")?;
    let listener = TcpListener::bind(addr)
        .await
        .context("bind dashboard port")?;

    state
        .db
        .upsert_local_service(
            "dashboard",
            &config.host,
            config.port,
            "running",
            "local",
            Some(std::process::id()),
        )
        .await?;

    let db_shutdown = state.db.clone();
    let shutdown_host = config.host.clone();
    let shutdown_port = config.port;

    info!(
        url = %format!("http://{}:{}/", config.host, config.port),
        db = %config.db_path.display(),
        "digital workbench listening"
    );
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let ctrl_c = async {
                tokio::signal::ctrl_c()
                    .await
                    .expect("failed to install Ctrl+C handler");
            };
            #[cfg(unix)]
            let terminate = async {
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .expect("failed to install signal handler")
                    .recv()
                    .await;
            };
            #[cfg(not(unix))]
            let terminate = std::future::pending::<()>();

            tokio::select! {
                _ = ctrl_c => {},
                _ = terminate => {},
            }
            crate::local_service::mark_self_stopped(&db_shutdown, &shutdown_host, shutdown_port)
                .await;
        })
        .await
        .context("dashboard server stopped")
}

pub async fn app_for_test(db_path: &Path) -> Result<Router> {
    app_for_test_with_host(db_path, "127.0.0.1").await
}

pub async fn app_for_test_with_host(db_path: &Path, host: &str) -> Result<Router> {
    let db = DashboardDb::open(db_path).await?;
    let state = AppState {
        db,
        events: Arc::new(EventBus::new()),
        sessions: SessionStore::default(),
        web_chat: crate::control::web_chat::WebChatHub::default(),
        version: "test".into(),
        static_dir: None,
        workspace_paths: vec![],
        tasks_root: PathBuf::from(".anycode/tasks"),
        host: host.into(),
        port: 43180,
        started_at: chrono::Utc::now().to_rfc3339(),
        pid: std::process::id(),
    };
    Ok(api::router(state))
}
