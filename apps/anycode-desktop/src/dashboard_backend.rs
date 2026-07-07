//! In-process Digital Workbench server (replaces `anycode dashboard` sidecar).

use anycode_dashboard::server::{run_with_shutdown, DashboardConfig, default_db_path};
use anycode_dashboard::load_workspace_paths;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

pub struct DashboardServerState {
    shutdown_tx: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
}

impl DashboardServerState {
    pub fn new() -> Self {
        Self {
            shutdown_tx: Mutex::new(None),
        }
    }

    pub fn stop(&self) {
        if let Ok(mut guard) = self.shutdown_tx.lock() {
            if let Some(tx) = guard.take() {
                let _ = tx.send(());
            }
        }
    }
}

pub fn apply_dashboard_env(app: &AppHandle) {
    apply_local_account_env_if_unset();
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
    std::env::set_var("ANYCODE_DASHBOARD_EMBEDDED_CHAT", "1");
    std::env::set_var("ANYCODE_DASHBOARD_INPROCESS_TRIGGERS", "1");
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
}

pub fn start_in_process(app: AppHandle) {
    apply_dashboard_env(&app);
    let static_dir = std::env::var("ANYCODE_DASHBOARD_STATIC")
        .ok()
        .map(PathBuf::from)
        .filter(|p| p.join("index.html").is_file());
    let serve_ui = static_dir.is_some();
    if serve_ui {
        std::env::remove_var("ANYCODE_DASHBOARD_API_ONLY");
    }
    let config = DashboardConfig {
        host: "127.0.0.1".into(),
        port: 43_180,
        db_path: default_db_path(),
        static_dir,
        serve_ui,
        version: env!("CARGO_PKG_VERSION").into(),
    };
    let paths = load_workspace_paths();
    let (tx, rx) = tokio::sync::oneshot::channel();
    if let Some(state) = app.try_state::<DashboardServerState>() {
        if let Ok(mut guard) = state.shutdown_tx.lock() {
            *guard = Some(tx);
        }
    }
    tauri::async_runtime::spawn(async move {
        if let Err(e) = run_with_shutdown(config, paths, rx).await {
            eprintln!("anycode-desktop: dashboard server exited: {e:#}");
        }
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

/// When launched from Finder/DMG, shell env is empty. If a local account-service is
/// listening on loopback, point Workbench cloud login at it instead of anycode.work.
fn apply_local_account_env_if_unset() {
    if std::env::var("ANYCODE_ACCOUNT_API_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .is_some()
    {
        return;
    }

    let Some(health_body) = http_get_body("127.0.0.1", 43200, "/health") else {
        return;
    };
    let Ok(health) = serde_json::from_str::<serde_json::Value>(&health_body) else {
        return;
    };
    if health.get("ok").and_then(|v| v.as_bool()) != Some(true) {
        return;
    }

    std::env::set_var("ANYCODE_ACCOUNT_API_URL", "http://127.0.0.1:43200");

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
        "http://127.0.0.1:43200".into()
    };
    std::env::set_var("ANYCODE_ACCOUNT_PORTAL_URL", portal);
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
