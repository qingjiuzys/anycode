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
    // P1.6: refresh the config.json → settings table mirror at startup
    // (auto-backups config.json.bak before the first SQLite write).
    if let Err(e) = crate::config_patch::sync_settings_mirror(&db).await {
        tracing::warn!(error = %e, "settings mirror sync skipped");
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
        Some(Arc::new(
            crate::lan::LanHub::new(
                config.version.clone(),
                crate::lan::lan_data_dir(),
                Some(&db),
            )
            .await,
        ))
    } else {
        None
    };
    let features = match anycode_config::load_runtime_config(anycode_config::LoadOpts {
        config_file: None,
        ignore_approval: true,
        workspace_overlay: false,
        workspace_overlay_dir: None,
    })
    .await
    {
        Ok(cfg) => cfg.runtime.features,
        Err(_) => anycode_core::FeatureRegistry::default(),
    };
    let state = AppState {
        db,
        events: Arc::clone(&events),
        sessions: SessionStore::default(),
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
        desktop_bootstrap_token: Arc::new(tokio::sync::Mutex::new(
            config.desktop_bootstrap_token.clone(),
        )),
        test_auth_bypass: std::env::var("ANYCODE_DASHBOARD_TEST_AUTH_BYPASS")
            .ok()
            .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true")),
        embedded_desktop: crate::api::auth::embedded_desktop(),
        lan_hub: lan_hub.clone(),
        harness_graph: anycode_core::harness_graph_enabled(&features),
        harness_unified_kernel: anycode_core::harness_unified_kernel_enabled(&features),
        harness_gray_projects: anycode_core::harness_gray_projects_from_env(),
        accounts_sso: crate::api::state::accounts_client_if_server_side(
            crate::api::auth::embedded_desktop(),
        ),
        sso_redirect: std::env::var("ANYCODE_HARNESS_SSO_REDIRECT")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        sso_hops: std::sync::Arc::new(crate::api::state::SsoHopStore::default()),
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
    crate::remote_chat::spawn_remote_chat_worker(state.clone());
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
            sweep_uploads_once_per_day();
            maybe_generate_efficiency_report(&db_audit).await;
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

/// 每日一次清理闲置超过 7 天的 `uploads/<session_id>` 目录（best-effort）。
fn sweep_uploads_once_per_day() {
    static LAST_SWEEP_DAY: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let day = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() / 86_400)
        .unwrap_or(0);
    if LAST_SWEEP_DAY.swap(day, std::sync::atomic::Ordering::Relaxed) == day {
        return;
    }
    crate::control::text_upload::sweep_uploads_dir(std::time::Duration::from_secs(7 * 24 * 3600));
}

/// 每周一次效能报告（确定性聚合；文件已存在则跳过，错误仅 log）。
async fn maybe_generate_efficiency_report(db: &crate::db::DashboardDb) {
    static LAST_REPORT_WEEK: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    use chrono::Datelike;
    let now = chrono::Utc::now();
    let week = now.iso_week();
    let key = week.year() as u64 * 100 + week.week() as u64;
    if LAST_REPORT_WEEK.swap(key, std::sync::atomic::Ordering::Relaxed) == key {
        return;
    }
    match crate::observability::efficiency_report::generate_weekly_report(db).await {
        Ok(Some(path)) => {
            tracing::info!(path = %path.display(), "weekly efficiency report generated")
        }
        Ok(None) => {}
        Err(e) => tracing::warn!(error = %e, "weekly efficiency report failed"),
    }
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

/// Scripted Host for `app_for_test*`. Fixture chat must not load the developer
/// `~/.anycode/config.json` (anycode_cloud pairing) or mutate process env.
pub fn scripted_fixture_runtime() -> Arc<anycode_agent::AgentRuntime> {
    struct RepeatingScriptedLlm;

    #[async_trait::async_trait]
    impl anycode_core::LLMClient for RepeatingScriptedLlm {
        async fn chat(
            &self,
            _messages: Vec<anycode_core::Message>,
            _tools: Vec<anycode_core::ToolSchema>,
            _config: &anycode_core::ModelConfig,
        ) -> Result<anycode_core::LLMResponse, anycode_core::CoreError> {
            Ok(anycode_core::LLMResponse {
                message: anycode_core::Message {
                    id: uuid::Uuid::new_v4(),
                    role: anycode_core::MessageRole::Assistant,
                    content: anycode_core::MessageContent::Text("fixture-ok".into()),
                    timestamp: chrono::Utc::now(),
                    metadata: Default::default(),
                },
                tool_calls: vec![],
                usage: anycode_core::Usage {
                    input_tokens: 2,
                    output_tokens: 1,
                    cache_creation_tokens: None,
                    cache_read_tokens: None,
                },
            })
        }

        async fn chat_stream(
            &self,
            _messages: Vec<anycode_core::Message>,
            _tools: Vec<anycode_core::ToolSchema>,
            _config: &anycode_core::ModelConfig,
        ) -> Result<tokio::sync::mpsc::Receiver<anycode_core::StreamEvent>, anycode_core::CoreError>
        {
            let (tx, rx) = tokio::sync::mpsc::channel(8);
            tokio::spawn(async move {
                let _ = tx
                    .send(anycode_core::StreamEvent::Delta("fixture-ok".into()))
                    .await;
                let _ = tx
                    .send(anycode_core::StreamEvent::Usage(anycode_core::Usage {
                        input_tokens: 2,
                        output_tokens: 1,
                        cache_creation_tokens: None,
                        cache_read_tokens: None,
                    }))
                    .await;
                let _ = tx.send(anycode_core::StreamEvent::Done).await;
            });
            Ok(rx)
        }
    }

    anycode_agent::AgentRuntime::sandboxed_scripted(
        Arc::new(RepeatingScriptedLlm),
        std::collections::HashMap::new(),
    )
}

pub struct TestAppOptions {
    pub host: String,
    pub serve_ui: bool,
    pub auth_bypass: bool,
    pub embedded_desktop: bool,
    pub desktop_bootstrap_token: Option<String>,
    pub harness_graph: bool,
    pub harness_unified_kernel: bool,
    pub harness_gray_projects: Vec<String>,
    pub tasks_root: Option<PathBuf>,
    pub accounts_sso: Option<std::sync::Arc<anycode_harness_cloud818::identity::AccountsClient>>,
    pub seeded_runtime: Option<std::sync::Arc<anycode_agent::AgentRuntime>>,
    pub sso_redirect: Option<String>,
}

impl Default for TestAppOptions {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            serve_ui: true,
            auth_bypass: true,
            embedded_desktop: crate::api::auth::embedded_desktop(),
            desktop_bootstrap_token: None,
            harness_graph: false,
            harness_unified_kernel: false,
            harness_gray_projects: Vec::new(),
            tasks_root: None,
            accounts_sso: None,
            seeded_runtime: None,
            sso_redirect: None,
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
            seeded_runtime: Some(scripted_fixture_runtime()),
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
    let mut chat_runtime = crate::control::chat_runtime::ChatRuntimeHost::new()
        .with_session_stores(db_for_state.clone(), Arc::clone(&events));
    if let Some(runtime) = opts.seeded_runtime {
        chat_runtime = chat_runtime.with_seeded_runtime(runtime);
    }
    let state = AppState {
        db,
        events: Arc::clone(&events),
        sessions: SessionStore::default(),
        web_chat_tail: crate::control::web_chat_tail::WebChatTailHub::default(),
        chat_runtime,
        version: "test".into(),
        static_dir: None,
        serve_ui: opts.serve_ui,
        workspace_paths: vec![],
        tasks_root: opts
            .tasks_root
            .unwrap_or_else(|| PathBuf::from(".anycode/tasks")),
        host: opts.host,
        port: 43180,
        started_at: chrono::Utc::now().to_rfc3339(),
        pid: std::process::id(),
        desktop_bootstrap_token: Arc::new(tokio::sync::Mutex::new(opts.desktop_bootstrap_token)),
        test_auth_bypass: opts.auth_bypass,
        embedded_desktop: opts.embedded_desktop,
        lan_hub: None,
        harness_graph: opts.harness_graph,
        harness_unified_kernel: opts.harness_unified_kernel,
        harness_gray_projects: opts.harness_gray_projects,
        accounts_sso: opts.accounts_sso,
        sso_redirect: opts.sso_redirect,
        sso_hops: std::sync::Arc::new(crate::api::state::SsoHopStore::default()),
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
    async fn desktop_bootstrap_mints_local_session() {
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

        // Packaged Desktop no longer trusts loopback alone — API requires the
        // dw_session cookie minted by /api/auth/desktop-bootstrap.
        let unauthenticated = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/projects")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            unauthenticated.status(),
            axum::http::StatusCode::UNAUTHORIZED
        );

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

        // WKWebView may retry the handshake URL; the process token stays valid.
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
        assert_eq!(replay.status(), axum::http::StatusCode::SEE_OTHER);
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
    async fn harness_pairing_with_bearer_is_not_origin_blocked() {
        let dir = tempfile::tempdir().unwrap();
        let app = app_for_test(&dir.path().join("pair-origin.db"))
            .await
            .unwrap();
        let res = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/harness/pairing/challenges")
                    .header("origin", "http://127.0.0.1:5175")
                    .header("host", "127.0.0.1:43180")
                    .header("content-type", "application/json")
                    .header("authorization", "Bearer test-sso")
                    .body(Body::from(r#"{"label":"desktop"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_ne!(res.status(), axum::http::StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn desktop_pairing_client_posts_to_bound_loopback_bff() {
        let dir = tempfile::tempdir().unwrap();
        let app = app_for_test(&dir.path().join("pair-tcp.db")).await.unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
        tokio::spawn(async move {
            axum::serve(listener, app).await.ok();
        });
        let challenge = anycode_harness_cloud818::desktop_pairing::post_pairing(
            &origin,
            "/api/harness/pairing/challenges",
            Some("test-sso"),
            serde_json::json!({"label":"desktop"}),
        )
        .await
        .expect("desktop client challenge");
        let challenge_s = challenge["challenge"].as_str().expect("challenge");
        assert_eq!(challenge["expires_in"], 300);
        let confirm = anycode_harness_cloud818::desktop_pairing::post_pairing(
            &origin,
            "/api/harness/pairing/confirm",
            Some("test-sso"),
            serde_json::json!({ "challenge": challenge_s }),
        )
        .await
        .expect("desktop client confirm");
        let device_id = confirm["device_id"].as_str().expect("device_id");
        let token = confirm["token"].as_str().expect("token");
        assert_eq!(device_id.len(), 36);
        assert_eq!(token.len(), 43);
        assert_eq!(confirm["keychain_service"], "anycode.harness.device");
    }

    #[tokio::test]
    async fn pairing_sso_begin_requires_server_accounts_client() {
        let dir = tempfile::tempdir().unwrap();
        let app = app_for_test(&dir.path().join("sso-begin.db"))
            .await
            .unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
        tokio::spawn(async move {
            axum::serve(listener, app).await.ok();
        });
        let err = anycode_harness_cloud818::desktop_pairing::post_pairing(
            &origin,
            "/api/harness/pairing/sso/begin",
            None,
            serde_json::json!({}),
        )
        .await
        .expect_err("embedded/local BFF must not mint SSO hops");
        assert!(err.contains("sso hop is not configured"), "{err}");
    }

    #[tokio::test]
    async fn pairing_sso_begin_returns_authorize_url_without_secret() {
        let dir = tempfile::tempdir().unwrap();
        let client = anycode_harness_cloud818::identity::AccountsClient::new(
            "http://127.0.0.1:18780",
            "a".repeat(32),
            true,
        )
        .unwrap();
        let app = app_for_test_custom(
            &dir.path().join("sso-begin-ok.db"),
            TestAppOptions {
                accounts_sso: Some(std::sync::Arc::new(client)),
                sso_redirect: Some("http://127.0.0.1:18781/api/auth/hop/v2/callback".into()),
                ..TestAppOptions::default()
            },
        )
        .await
        .unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
        tokio::spawn(async move {
            axum::serve(listener, app).await.ok();
        });
        let begin = anycode_harness_cloud818::desktop_pairing::post_pairing(
            &origin,
            "/api/harness/pairing/sso/begin",
            None,
            serde_json::json!({}),
        )
        .await
        .expect("begin");
        let url = begin["authorize_url"].as_str().expect("authorize_url");
        assert!(url.starts_with("http://127.0.0.1:18780/api/v2/sso/authorize"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("redirect_uri=http"));
        assert_eq!(begin["poll_token"].as_str().unwrap().len(), 43);
        assert!(!serde_json::to_string(&begin).unwrap().contains("secret"));
        let poll = anycode_harness_cloud818::desktop_pairing::post_pairing(
            &origin,
            "/api/harness/pairing/sso/poll",
            None,
            serde_json::json!({ "poll_token": begin["poll_token"] }),
        )
        .await
        .expect("poll pending");
        assert_eq!(poll["pending"], true);
    }

    #[tokio::test]
    async fn pairing_challenges_accept_sso_bearer_without_local_session() {
        let dir = tempfile::tempdir().unwrap();
        let client = anycode_harness_cloud818::identity::AccountsClient::new(
            "http://127.0.0.1:18780",
            "a".repeat(32),
            true,
        )
        .unwrap();
        let app = app_for_test_custom(
            &dir.path().join("sso-chal.db"),
            TestAppOptions {
                auth_bypass: false,
                accounts_sso: Some(std::sync::Arc::new(client)),
                sso_redirect: Some(
                    "http://127.0.0.1:18782/api/harness/pairing/sso/callback".into(),
                ),
                ..TestAppOptions::default()
            },
        )
        .await
        .unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
        tokio::spawn(async move {
            axum::serve(listener, app).await.ok();
        });
        let denied = anycode_harness_cloud818::desktop_pairing::post_pairing(
            &origin,
            "/api/harness/pairing/challenges",
            None,
            serde_json::json!({"label":"x"}),
        )
        .await
        .expect_err("anonymous challenge must stay closed");
        assert!(
            denied.contains("401") || denied.contains("local session"),
            "{denied}"
        );
        let err = anycode_harness_cloud818::desktop_pairing::post_pairing(
            &origin,
            "/api/harness/pairing/challenges",
            Some(&"A".repeat(43)),
            serde_json::json!({"label":"x"}),
        )
        .await
        .expect_err("opaque bearer reaches introspect, not session middleware");
        assert!(
            !err.contains("local session or API token required"),
            "{err}"
        );
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
