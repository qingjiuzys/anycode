use crate::api::{self, state::AppState};
use crate::auth_session::SessionStore;
use crate::db::DashboardDb;
use crate::events::EventBus;
use crate::ingest::ingest_recent_disk_tasks;
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
            if let Ok(n) = ingest_recent_disk_tasks(&db, &tasks_root, &workspace_paths).await {
                if n > 0 {
                    info!(count = n, "auto-ingested recent task logs on first boot");
                }
            }
            let _ = sync_skills_to_db(&db, &workspace_paths).await;
        }
    }
    match sync_skills_to_db(&db, &workspace_paths).await {
        Ok(n) if n > 0 => info!(count = n, "synced local skills"),
        Ok(_) => {}
        Err(e) => tracing::warn!(error = %e, "skills scan skipped"),
    }
    if tasks_root.is_dir() {
        match ingest_recent_disk_tasks(&db, &tasks_root, &workspace_paths).await {
            Ok(n) if n > 0 => info!(count = n, "ingested recent task logs"),
            Ok(_) => {}
            Err(e) => tracing::warn!(error = %e, "task log ingest skipped"),
        }
    }
    let events = Arc::new(EventBus::new());
    db.upsert_local_service(
        "dashboard",
        &config.host,
        config.port,
        "running",
        "local",
        Some(std::process::id()),
    )
    .await?;

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
    let state = AppState {
        db,
        events,
        sessions: SessionStore::default(),
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
    let app = api::router(state);
    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .context("parse listen address")?;
    let listener = TcpListener::bind(addr)
        .await
        .context("bind dashboard port")?;
    info!(
        url = %format!("http://{}:{}/", config.host, config.port),
        db = %config.db_path.display(),
        "digital workbench listening"
    );
    axum::serve(listener, app)
        .await
        .context("dashboard server stopped")
}

pub async fn app_for_test(db_path: &Path) -> Result<Router> {
    let db = DashboardDb::open(db_path).await?;
    let state = AppState {
        db,
        events: Arc::new(EventBus::new()),
        sessions: SessionStore::default(),
        version: "test".into(),
        static_dir: None,
        workspace_paths: vec![],
        tasks_root: PathBuf::from(".anycode/tasks"),
        host: "127.0.0.1".into(),
        port: 43180,
        started_at: chrono::Utc::now().to_rfc3339(),
        pid: std::process::id(),
    };
    Ok(api::router(state))
}
