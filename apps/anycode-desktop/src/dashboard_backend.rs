//! In-process Digital Workbench server (replaces `anycode dashboard` sidecar).

use anycode_dashboard::generate_desktop_bootstrap_token;
use anycode_dashboard::load_workspace_paths;
use anycode_dashboard::server::{default_db_path, run_with_shutdown, DashboardConfig};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager};

const DASHBOARD_HOST: &str = "127.0.0.1";

/// OS-assigned loopback port for the in-process API (set after bind; `0` until ready).
static DESKTOP_API_PORT: AtomicU16 = AtomicU16::new(0);

pub fn desktop_api_port() -> u16 {
    DESKTOP_API_PORT.load(Ordering::SeqCst)
}

pub fn desktop_api_base() -> Option<String> {
    let port = desktop_api_port();
    (port > 0).then(|| format!("http://{DASHBOARD_HOST}:{port}"))
}

/// True when loopback `/api/health` returns `{"ok":true}`.
pub fn dashboard_http_ready() -> bool {
    dashboard_health_body().is_some_and(|body| body.contains("\"ok\":true"))
}

/// GET /api/health response when something HTTP answers on the dashboard port.
fn dashboard_health_body() -> Option<String> {
    let port = desktop_api_port();
    if port == 0 {
        return None;
    }
    use std::io::{Read, Write};
    use std::net::TcpStream;
    let mut stream = TcpStream::connect((DASHBOARD_HOST, port)).ok()?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    let req = format!(
        "GET /api/health HTTP/1.1\r\nHost: {DASHBOARD_HOST}:{port}\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(req.as_bytes()).ok()?;
    let mut buf = [0u8; 2048];
    let n = stream.read(&mut buf).ok()?;
    let resp = String::from_utf8_lossy(&buf[..n]).to_string();
    resp.contains("200").then_some(resp)
}

pub struct DashboardServerState {
    shutdown_tx: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    /// One-shot copy for the first Workbench navigation; server holds the
    /// authoritative token in AppState and consumes it on bootstrap.
    bootstrap_token: Mutex<Option<String>>,
}

impl DashboardServerState {
    pub fn new() -> Self {
        Self {
            shutdown_tx: Mutex::new(None),
            bootstrap_token: Mutex::new(None),
        }
    }

    pub fn stop(&self) {
        if let Ok(mut guard) = self.shutdown_tx.lock() {
            if let Some(tx) = guard.take() {
                let _ = tx.send(());
            }
        }
        DESKTOP_API_PORT.store(0, Ordering::SeqCst);
    }

    /// Take the one-shot desktop bootstrap token (first navigation only).
    pub fn take_bootstrap_token(&self) -> Option<String> {
        self.bootstrap_token.lock().ok()?.take()
    }
}

pub fn apply_dashboard_env(app: &AppHandle) {
    apply_account_env_if_unset(app);
    if let Some(tpl) = resolve_resource_path(
        app,
        &[
            "resources/project-templates",
            "project-templates",
            "_up_/resources/project-templates",
        ],
    ) {
        std::env::set_var("ANYCODE_PROJECT_TEMPLATES", tpl);
    }
    if let Some(ui) = resolve_resource_path(
        app,
        &[
            "resources/dashboard-ui",
            "dashboard-ui",
            "_up_/resources/dashboard-ui",
        ],
    ) {
        if ui.join("index.html").is_file() {
            std::env::set_var("ANYCODE_DASHBOARD_STATIC", ui);
        }
    }
    std::env::set_var("ANYCODE_DASHBOARD_EMBEDDED_DESKTOP", "1");
    std::env::set_var("ANYCODE_DASHBOARD_INPROCESS_EVENTS", "1");
    if let Some(browser) = resolve_resource_path(
        app,
        &["resources/browser", "browser", "_up_/resources/browser"],
    ) {
        if browser.join("run.sh").is_file() {
            std::env::set_var("ANYCODE_BROWSER_MCP_ROOT", &browser);
        }
        let chromium_path_file = browser.join(".chromium-path");
        if chromium_path_file.is_file() {
            if let Ok(raw) = std::fs::read_to_string(&chromium_path_file) {
                let p = raw.trim();
                if !p.is_empty() && std::path::Path::new(p).is_file() {
                    std::env::set_var("ANYCODE_CHROMIUM_PATH", p);
                }
            }
        }
    }
    if let Some(starter) = resolve_resource_path(
        app,
        &[
            "resources/skills-starter",
            "skills-starter",
            "_up_/resources/skills-starter",
        ],
    ) {
        if starter.is_dir() {
            std::env::set_var("ANYCODE_SKILLS_STARTER", starter);
        }
    }
}

pub fn start_in_process(app: AppHandle) {
    apply_dashboard_env(&app);
    DESKTOP_API_PORT.store(0, Ordering::SeqCst);

    let bootstrap_token = generate_desktop_bootstrap_token();
    if let Some(state) = app.try_state::<DashboardServerState>() {
        if let Ok(mut guard) = state.bootstrap_token.lock() {
            *guard = Some(bootstrap_token.clone());
        }
    }
    let (bound_port_tx, bound_port_rx) = tokio::sync::oneshot::channel();
    let static_dir = resolve_resource_path(
        &app,
        &[
            "resources/dashboard-ui",
            "dashboard-ui",
            "_up_/resources/dashboard-ui",
        ],
    )
    .filter(|p| p.join("index.html").is_file());
    let serve_ui = static_dir.is_some();
    if !serve_ui {
        eprintln!(
            "anycode-desktop: bundled dashboard-ui not found in app resources — Workbench UI unavailable"
        );
    }
    let config = DashboardConfig {
        host: DASHBOARD_HOST.into(),
        port: 0,
        db_path: default_db_path(),
        static_dir,
        serve_ui,
        version: env!("CARGO_PKG_VERSION").into(),
        desktop_bootstrap_token: Some(bootstrap_token),
        bound_port_tx: Some(bound_port_tx),
    };
    let paths = load_workspace_paths();
    let (tx, rx) = tokio::sync::oneshot::channel();
    if let Some(state) = app.try_state::<DashboardServerState>() {
        if let Ok(mut guard) = state.shutdown_tx.lock() {
            *guard = Some(tx);
        }
    }
    tauri::async_runtime::spawn(async move {
        let server_task = tauri::async_runtime::spawn(async move {
            run_with_shutdown(config, paths, rx).await
        });
        if let Ok(port) = bound_port_rx.await {
            DESKTOP_API_PORT.store(port, Ordering::SeqCst);
            eprintln!(
                "anycode-desktop: workbench API listening on http://{DASHBOARD_HOST}:{port}/"
            );
        }
        match server_task.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => eprintln!("anycode-desktop: dashboard server exited: {e:#}"),
            Err(e) => eprintln!("anycode-desktop: dashboard server task failed: {e}"),
        }
        DESKTOP_API_PORT.store(0, Ordering::SeqCst);
    });
}

fn resolve_resource_path(app: &AppHandle, candidates: &[&str]) -> Option<PathBuf> {
    use tauri::path::BaseDirectory;
    for rel in candidates {
        if let Ok(p) = app.path().resolve(rel, BaseDirectory::Resource) {
            if p.is_file() || p.is_dir() {
                return Some(p);
            }
        }
    }
    None
}

/// When launched from Finder/DMG, shell env is empty. Prefer a live loopback
/// account-service, then the build-time `account-endpoints.json` manifest.
fn apply_account_env_if_unset(app: &AppHandle) {
    if std::env::var("ANYCODE_ACCOUNT_API_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .is_some()
    {
        return;
    }

    if let Some((api, portal, gateway)) = probe_live_local_account() {
        std::env::set_var("ANYCODE_ACCOUNT_API_URL", &api);
        std::env::set_var("ANYCODE_ACCOUNT_PORTAL_URL", &portal);
        if let Some(gateway) = gateway {
            std::env::set_var("ANYCODE_MODEL_GATEWAY_URL", gateway);
        }
        return;
    }

    if let Some(manifest) = read_bundled_account_endpoints(app) {
        std::env::set_var("ANYCODE_ACCOUNT_API_URL", &manifest.api_url);
        std::env::set_var("ANYCODE_ACCOUNT_PORTAL_URL", &manifest.portal_url);
        if let Some(gateway) = manifest.gateway_url {
            std::env::set_var("ANYCODE_MODEL_GATEWAY_URL", gateway);
        }
    }
}

#[derive(Debug)]
struct BundledAccountEndpoints {
    api_url: String,
    portal_url: String,
    gateway_url: Option<String>,
}

fn read_bundled_account_endpoints(app: &AppHandle) -> Option<BundledAccountEndpoints> {
    let path = resolve_resource_path(
        app,
        &[
            "resources/account-endpoints.json",
            "account-endpoints.json",
            "_up_/resources/account-endpoints.json",
        ],
    )?;
    let raw = std::fs::read_to_string(&path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let api_url = value
        .get("account_api_url")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())?
        .to_string();
    let portal_url = value
        .get("account_portal_url")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(api_url.as_str())
        .to_string();
    let gateway_url = value
        .get("model_gateway_url")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    Some(BundledAccountEndpoints {
        api_url,
        portal_url,
        gateway_url,
    })
}

fn probe_live_local_account() -> Option<(String, String, Option<String>)> {
    let health_body = http_get_body("127.0.0.1", 43200, "/health")?;
    let health: serde_json::Value = serde_json::from_str(&health_body).ok()?;
    if health.get("ok").and_then(|v| v.as_bool()) != Some(true) {
        return None;
    }

    let api = "http://127.0.0.1:43200".to_string();
    let portal = if http_get_body("127.0.0.1", 43201, "/login").is_some() {
        "http://127.0.0.1:43201".into()
    } else if let Some(url) = health
        .get("portal_url")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        url.trim_end_matches('/').to_string()
    } else {
        api.clone()
    };
    let gateway = if http_get_body("127.0.0.1", 43210, "/health").is_some() {
        Some("http://127.0.0.1:43210".into())
    } else {
        None
    };
    Some((api, portal, gateway))
}

fn http_get_body(host: &str, port: u16, path: &str) -> Option<String> {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    let mut stream = TcpStream::connect((host, port)).ok()?;
    let _ = stream.set_read_timeout(Some(Duration::from_millis(800)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(800)));
    let req = format!(
        "GET {path} HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(req.as_bytes()).ok()?;
    let mut raw = String::new();
    stream.read_to_string(&mut raw).ok()?;
    let (_head, body) = raw.split_once("\r\n\r\n")?;
    Some(body.to_string())
}
