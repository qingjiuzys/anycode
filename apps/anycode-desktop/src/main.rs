#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{
    menu::{Menu, MenuItem},
    path::BaseDirectory,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, RunEvent, Url,
};

const DASHBOARD_URL: &str = "http://127.0.0.1:43180/";

struct SidecarState(Mutex<Vec<Child>>);

fn resolve_resource_path(app: &tauri::AppHandle, candidates: &[&str]) -> Option<PathBuf> {
    for rel in candidates {
        if let Ok(p) = app.path().resolve(rel, BaseDirectory::Resource) {
            if p.is_file() || p.is_dir() {
                return Some(p);
            }
        }
    }
    None
}

fn resolve_anycode_program(app: &tauri::AppHandle) -> PathBuf {
    resolve_resource_path(
        app,
        &[
            "resources/bin/anycode",
            "bin/anycode",
            "_up_/resources/bin/anycode",
        ],
    )
    .unwrap_or_else(|| PathBuf::from("anycode"))
}

fn spawn_sidecar(label: &str, program: &Path, args: &[&str], app: &tauri::AppHandle) -> Option<Child> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    if let Some(tpl) = resolve_resource_path(
        app,
        &[
            "resources/project-templates",
            "project-templates",
            "_up_/resources/project-templates",
        ],
    ) {
        cmd.env("ANYCODE_PROJECT_TEMPLATES", tpl);
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
            cmd.env("ANYCODE_DASHBOARD_STATIC", ui);
        }
    }
    if let Some(browser) = resolve_resource_path(
        app,
        &[
            "resources/browser",
            "browser",
            "_up_/resources/browser",
        ],
    ) {
        if browser.join("run.sh").is_file() {
            cmd.env("ANYCODE_BROWSER_MCP_ROOT", browser);
        }
    }
    match cmd.spawn() {
        Ok(child) => {
            eprintln!(
                "anycode-desktop: started {label} (pid {}, bin {})",
                child.id(),
                program.display()
            );
            Some(child)
        }
        Err(e) => {
            eprintln!(
                "anycode-desktop: could not start {label} via {}: {e}",
                program.display()
            );
            None
        }
    }
}

fn stop_sidecars(state: &SidecarState) {
    if let Ok(mut guard) = state.0.lock() {
        for mut child in guard.drain(..) {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn dashboard_ready() -> bool {
    std::net::TcpStream::connect("127.0.0.1:43180").is_ok()
}

fn wait_for_dashboard_port(timeout_secs: u64) -> bool {
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    while Instant::now() < deadline {
        if dashboard_ready() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(400));
    }
    eprintln!("anycode-desktop: dashboard port 43180 not ready after {timeout_secs}s");
    false
}

fn show_workbench(app: &tauri::AppHandle, ready: bool) {
    let Some(w) = app.get_webview_window("main") else {
        return;
    };
    if ready {
        if let Ok(url) = Url::parse(DASHBOARD_URL) {
            let _ = w.navigate(url);
        }
    }
    let _ = w.show();
    let _ = w.set_focus();
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(SidecarState(Mutex::new(Vec::new())))
        .setup(|app| {
            let program = resolve_anycode_program(app.handle());
            eprintln!(
                "anycode-desktop: using anycode binary {}",
                program.display()
            );
            let mut children = Vec::new();
            let dashboard_ok = if let Some(c) =
                spawn_sidecar("anycode dashboard", &program, &["dashboard"], app.handle())
            {
                children.push(c);
                wait_for_dashboard_port(60)
            } else {
                false
            };
            if std::env::var("ANYCODE_DESKTOP_WECHAT")
                .ok()
                .is_some_and(|v| matches!(v.as_str(), "1" | "true" | "yes"))
            {
                if let Some(c) = spawn_sidecar(
                    "WeChat bridge",
                    &program,
                    &["channel", "wechat", "--run-as-bridge"],
                    app.handle(),
                ) {
                    children.push(c);
                }
            }
            if let Some(state) = app.try_state::<SidecarState>() {
                *state.0.lock().unwrap() = children;
            }
            show_workbench(app.handle(), dashboard_ok);

            let open_i = MenuItem::with_id(app, "open", "Open Workbench", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open_i, &quit_i])?;
            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show_workbench(app, dashboard_ready()),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_workbench(tray.app_handle(), dashboard_ready());
                    }
                })
                .build(app)?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while running anycode desktop")
        .run(|app, event| {
            if matches!(event, RunEvent::Exit | RunEvent::ExitRequested { .. }) {
                if let Some(state) = app.try_state::<SidecarState>() {
                    stop_sidecars(&*state);
                }
            }
        });
}
