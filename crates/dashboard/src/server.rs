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

#[derive(Debug)]
pub struct DashboardConfig {
    pub host: String,
    pub port: u16,
    pub db_path: PathBuf,
    pub static_dir: Option<PathBuf>,
    /// When false, only `/api/*` (and WS/SSE) are served — no SPA at `/`.
    pub serve_ui: bool,
    pub version: String,
    /// Optional one-shot Desktop bootstrap token (embedded desktop only).
    pub desktop_bootstrap_token: Option<String>,
    /// Notified with the OS-assigned port when `port` is `0` (desktop ephemeral bind).
    pub bound_port_tx: Option<tokio::sync::oneshot::Sender<u16>>,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 43_180,
            db_path: default_db_path(),
            static_dir: None,
            serve_ui: true,
            version: env!("CARGO_PKG_VERSION").into(),
            desktop_bootstrap_token: None,
            bound_port_tx: None,
        }
    }
}

#[must_use]
pub fn default_db_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".anycode")
        .join("projects.db")
}

pub async fn run(config: DashboardConfig, workspace_paths: Vec<String>) -> Result<()> {
    let (_tx, rx) = tokio::sync::oneshot::channel::<()>();
    run_with_shutdown(config, workspace_paths, rx).await
}

/// Like [`run`], but also stops when `shutdown` is signaled (e.g. Tauri app exit).
pub async fn run_with_shutdown(
    config: DashboardConfig,
    workspace_paths: Vec<String>,
    shutdown: tokio::sync::oneshot::Receiver<()>,
) -> Result<()> {
    run_inner(config, workspace_paths, Some(shutdown)).await
}

async fn run_inner(
    config: DashboardConfig,
    workspace_paths: Vec<String>,
    shutdown: Option<tokio::sync::oneshot::Receiver<()>>,
) -> Result<()> {
    let _ = anycode_setup::ensure_layout();
    if let Err(e) = crate::media_defaults::ensure_default_local_stt() {
        tracing::warn!(error = %e, "default local STT bootstrap skipped");
    }
    let db = DashboardDb::open(&config.db_path)
        .await
        .context("open dashboard database")?;
    let tasks_root = dirs::home_dir()
        .map(|h| h.join(".anycode").join("tasks"))
        .unwrap_or_else(|| std::path::PathBuf::from(".anycode/tasks"));
    if !workspace_paths.is_empty() {
        let n = db.sync_workspace_paths(&workspace_paths).await?;
        info!(count = n, "synced workspace projects");
    }
    if let Ok(stats) = db.overview_stats().await {
        if stats.projects_count == 0 && !workspace_paths.is_empty() {
            info!("empty database — auto-scanning workspace projects");
            let _ = db.sync_workspace_paths(&workspace_paths).await;
        }
    }
    // Skill discovery walks every workspace root; keep it off the startup
    // critical path so HTTP binds while the catalog populates in background.
    // Read handlers that need fresh skills call sync_skills_to_db themselves
    // (TTL-cached), so nothing blocks on this spawn.
    let db_skills = db.clone();
    let paths_skills = workspace_paths.clone();
    tokio::spawn(async move {
        match sync_skills_to_db(&db_skills, &paths_skills).await {
            Ok(n) if n > 0 => info!(count = n, "synced local skills"),
            Ok(_) => {}
            Err(e) => tracing::warn!(error = %e, "skills scan skipped"),
        }
    });
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
        let mut reconciled = 0usize;
        for session in running {
            if !crate::cancel_ipc::is_active(&session.id)
                && db
                    .cancel_running_session(&session.id)
                    .await
                    .unwrap_or(false)
            {
                reconciled += 1;
            }
        }
        if reconciled > 0 {
            info!(
                count = reconciled,
                "reconciled orphan running sessions after dashboard restart"
            );
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
                "non-loopback dashboard requires at least one API token; create one in Settings → API tokens (or set ANYCODE_DASHBOARD_ALLOW_UNAUTH=1 for local dev)"
            );
        }
    }

    let static_dir = if config.serve_ui {
        config
            .static_dir
            .or_else(crate::static_ui::discover_ui_dist)
    } else {
        None
    };
    if static_dir.is_some() {
        info!("serving dashboard UI static files");
    } else if !config.serve_ui {
        info!("API-only mode (no Workbench SPA at /)");
    }
    let events = Arc::new(EventBus::new());
    crate::notify::register_inprocess_bus(Arc::clone(&events));
    let db_for_state = db.clone();
    let lan_hub = if crate::lan::lan_enabled() {
        Some(Arc::new(crate::lan::LanHub::new(
            config.version.clone(),
            crate::lan::lan_data_dir(),
        )))
    } else {
        None
    };
    let state = AppState {
        db,
        events: Arc::clone(&events),
        sessions: SessionStore::default(),
        web_chat: crate::control::web_chat::WebChatHub,
        web_chat_tail: crate::control::web_chat_tail::WebChatTailHub::default(),
        chat_runtime: crate::control::chat_runtime::ChatRuntimeHost::new()
            .with_session_stores(db_for_state.clone(), Arc::clone(&events)),
        version: config.version.clone(),
        static_dir,
        serve_ui: config.serve_ui,
        workspace_paths: workspace_paths.clone(),
        tasks_root: tasks_root.clone(),
        host: config.host.clone(),
        port: config.port,
        started_at: started_at.clone(),
        pid: std::process::id(),
        managed_local_llm: crate::managed_local_llm::ManagedLocalLlm::new(),
        desktop_bootstrap_token: Arc::new(tokio::sync::Mutex::new(
            config.desktop_bootstrap_token.clone(),
        )),
        test_auth_bypass: std::env::var("ANYCODE_DASHBOARD_TEST_AUTH_BYPASS")
            .ok()
            .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true")),
        embedded_desktop: crate::api::auth::embedded_desktop(),
        lan_hub: lan_hub.clone(),
    };
    crate::control::question_notify::install(events, db_for_state.clone());
    crate::control::approval_notify::install(Arc::clone(&state.events), db_for_state);
    if let Some(hub) = lan_hub {
        crate::lan::spawn_discovery(Arc::clone(&hub));
        crate::lan::spawn_lan_listener(crate::lan::LanListenerState {
            hub: Arc::clone(&hub),
            db: state.db.clone(),
            events: Arc::clone(&state.events),
            memory_root: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".anycode")
                .join("memory"),
        });
    }
    crate::api::spawn_cloud_a2a_heartbeat(state.clone());
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
    let db_usage = state.db.clone();
    let tasks_root_usage = state.tasks_root.clone();
    tokio::spawn(async move {
        match crate::observability::usage_backfill::backfill_llm_usage(&db_usage, &tasks_root_usage)
            .await
        {
            Ok(n) => tracing::debug!(count = n, "llm usage events backfilled"),
            Err(e) => tracing::warn!(error = %e, "llm usage backfill failed"),
        }
    });
    let db_audit = state.db.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            match crate::compliance_audit_upload::flush_pending(&db_audit).await {
                Ok(n) if n > 0 => tracing::info!(count = n, "compliance audit batch uploaded"),
                Ok(_) => {}
                Err(e) => tracing::warn!(error = %e, "compliance audit upload failed"),
            }
        }
    });
    if crate::service_governance::is_loopback_host(&config.host) {
        let spawn_gateway = std::env::var("ANYCODE_RELAY_GATEWAY").ok().as_deref() == Some("1");
        if spawn_gateway {
            let gw_cfg = anycode_relay_gateway::GatewayConfig::default();
            info!(
                port = gw_cfg.port,
                "spawning dev-only local relay gateway (ANYCODE_RELAY_GATEWAY=1)"
            );
            let _relay_handle = anycode_relay_gateway::spawn_gateway(gw_cfg);
        }
    }
    let app = api::router(state.clone());
    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .context("parse listen address")?;
    let listener = TcpListener::bind(addr)
        .await
        .context("bind dashboard port")?;
    let bound_port = listener.local_addr().context("read bound port")?.port();
    if let Some(tx) = config.bound_port_tx {
        let _ = tx.send(bound_port);
    }

    state
        .db
        .upsert_local_service(
            "dashboard",
            &config.host,
            bound_port,
            "running",
            "local",
            Some(std::process::id()),
        )
        .await?;

    let db_shutdown = state.db.clone();
    let shutdown_host = config.host.clone();
    let shutdown_port = bound_port;

    info!(
        url = %format!("http://{}:{}/", config.host, bound_port),
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

            if let Some(mut shutdown) = shutdown {
                tokio::select! {
                    _ = ctrl_c => {},
                    _ = terminate => {},
                    _ = &mut shutdown => {},
                }
            } else {
                tokio::select! {
                    _ = ctrl_c => {},
                    _ = terminate => {},
                }
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
    app_for_test_with_options(db_path, host, true).await
}

pub async fn app_for_test_api_only(db_path: &Path) -> Result<Router> {
    app_for_test_with_options(db_path, "127.0.0.1", false).await
}

pub struct TestAppOptions {
    pub host: String,
    pub serve_ui: bool,
    pub auth_bypass: bool,
    pub embedded_desktop: bool,
    pub desktop_bootstrap_token: Option<String>,
}

impl Default for TestAppOptions {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            serve_ui: true,
            auth_bypass: true,
            embedded_desktop: crate::api::auth::embedded_desktop(),
            desktop_bootstrap_token: None,
        }
    }
}

pub async fn app_for_test_with_options(
    db_path: &Path,
    host: &str,
    serve_ui: bool,
) -> Result<Router> {
    app_for_test_custom(
        db_path,
        TestAppOptions {
            host: host.into(),
            serve_ui,
            ..TestAppOptions::default()
        },
    )
    .await
}

pub async fn app_for_test_custom(db_path: &Path, opts: TestAppOptions) -> Result<Router> {
    let db = DashboardDb::open(db_path).await?;
    let events = Arc::new(EventBus::new());
    crate::notify::register_inprocess_bus(Arc::clone(&events));
    let db_for_state = db.clone();
    let state = AppState {
        db,
        events: Arc::clone(&events),
        sessions: SessionStore::default(),
        web_chat: crate::control::web_chat::WebChatHub,
        web_chat_tail: crate::control::web_chat_tail::WebChatTailHub::default(),
        chat_runtime: crate::control::chat_runtime::ChatRuntimeHost::new()
            .with_session_stores(db_for_state.clone(), Arc::clone(&events)),
        version: "test".into(),
        static_dir: None,
        serve_ui: opts.serve_ui,
        workspace_paths: vec![],
        tasks_root: PathBuf::from(".anycode/tasks"),
        host: opts.host,
        port: 43180,
        started_at: chrono::Utc::now().to_rfc3339(),
        pid: std::process::id(),
        managed_local_llm: crate::managed_local_llm::ManagedLocalLlm::new(),
        desktop_bootstrap_token: Arc::new(tokio::sync::Mutex::new(opts.desktop_bootstrap_token)),
        test_auth_bypass: opts.auth_bypass,
        embedded_desktop: opts.embedded_desktop,
        lan_hub: None,
    };
    crate::control::question_notify::install(events, db_for_state.clone());
    crate::control::approval_notify::install(Arc::clone(&state.events), db_for_state);
    Ok(api::router(state))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn api_only_root_is_not_spa() {
        let dir = tempfile::tempdir().unwrap();
        let app = app_for_test_api_only(&dir.path().join("projects.db"))
            .await
            .unwrap();
        let res = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(res.status().is_success());
        let body = res.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"], "api_only");
    }

    #[tokio::test]
    async fn local_models_api_lists_descriptor_status() {
        let dir = tempfile::tempdir().unwrap();
        let app = app_for_test(&dir.path().join("local-models.db"))
            .await
            .unwrap();
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/local-models")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["models"][0]["id"], "managed-minicpm5-1b");
        assert_eq!(json["models"][0]["sha256"].as_str().unwrap().len(), 64);
    }

    #[tokio::test]
    async fn desktop_bootstrap_mints_one_shot_local_session() {
        let token = crate::api::auth::generate_desktop_bootstrap_token();
        let dir = tempfile::tempdir().unwrap();
        let app = app_for_test_custom(
            &dir.path().join("bootstrap.db"),
            TestAppOptions {
                auth_bypass: false,
                embedded_desktop: true,
                desktop_bootstrap_token: Some(token.clone()),
                ..TestAppOptions::default()
            },
        )
        .await
        .unwrap();

        // Embedded desktop trusts the loopback API without a cookie.
        let loopback_trusted = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/projects")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(loopback_trusted.status(), axum::http::StatusCode::OK);

        let boot = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri(format!("/api/auth/desktop-bootstrap?token={token}"))
                    .header("host", "127.0.0.1:43180")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(boot.status(), axum::http::StatusCode::SEE_OTHER);
        let set_cookie = boot
            .headers()
            .get_all(axum::http::header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .find(|v| v.starts_with("dw_session="))
            .expect("dw_session cookie")
            .to_string();
        assert!(set_cookie.contains("HttpOnly"));
        assert!(set_cookie.contains("SameSite=Strict"));
        let session_cookie = set_cookie.split(';').next().unwrap().to_string();

        let allowed = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/projects")
                    .header(axum::http::header::COOKIE, &session_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(allowed.status(), axum::http::StatusCode::OK);

        // The token is one-shot: replaying it must not mint another session.
        let replay = app
            .oneshot(
                axum::http::Request::builder()
                    .uri(format!("/api/auth/desktop-bootstrap?token={token}"))
                    .header("host", "127.0.0.1:43180")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(replay.status(), axum::http::StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn desktop_bootstrap_rejects_without_embedded_flag() {
        let token = crate::api::auth::generate_desktop_bootstrap_token();
        let dir = tempfile::tempdir().unwrap();
        let app = app_for_test_custom(
            &dir.path().join("bootstrap-noembed.db"),
            TestAppOptions {
                auth_bypass: false,
                embedded_desktop: false,
                desktop_bootstrap_token: Some(token.clone()),
                ..TestAppOptions::default()
            },
        )
        .await
        .unwrap();
        let res = app
            .oneshot(
                axum::http::Request::builder()
                    .uri(format!("/api/auth/desktop-bootstrap?token={token}"))
                    .header("host", "127.0.0.1:43180")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), axum::http::StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn mutating_api_rejects_disallowed_origin() {
        let dir = tempfile::tempdir().unwrap();
        let app = app_for_test(&dir.path().join("origin.db")).await.unwrap();
        let res = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/auth/logout")
                    .header("origin", "https://evil.example")
                    .header("host", "127.0.0.1:43180")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), axum::http::StatusCode::FORBIDDEN);
        let body = res.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"], "origin not allowed");
    }

    #[tokio::test]
    async fn mutating_api_rejects_non_loopback_host_on_loopback_bind() {
        let dir = tempfile::tempdir().unwrap();
        let app = app_for_test_custom(
            &dir.path().join("host.db"),
            TestAppOptions {
                embedded_desktop: true,
                ..TestAppOptions::default()
            },
        )
        .await
        .unwrap();
        let res = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/auth/logout")
                    .header("origin", "http://127.0.0.1:43180")
                    .header("host", "evil.example")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), axum::http::StatusCode::FORBIDDEN);
        let body = res.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"], "host not allowed");
    }
}
