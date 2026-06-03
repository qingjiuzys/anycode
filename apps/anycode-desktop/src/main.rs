#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{
    menu::{Menu, MenuItem},
    path::BaseDirectory,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, RunEvent,
};

struct SidecarState(Mutex<Vec<Child>>);

fn resolve_anycode_program(app: &tauri::AppHandle) -> PathBuf {
    if let Ok(resource) = app.path().resolve("bin/anycode", BaseDirectory::Resource) {
        if resource.is_file() {
            return resource;
        }
    }
    PathBuf::from("anycode")
}

fn spawn_sidecar(label: &str, program: &Path, args: &[&str]) -> Option<Child> {
    match Command::new(program).args(args).spawn() {
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

fn wait_for_dashboard_port(timeout_secs: u64) {
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    while Instant::now() < deadline {
        if std::net::TcpStream::connect("127.0.0.1:43180").is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(400));
    }
    eprintln!("anycode-desktop: dashboard port 43180 not ready after {timeout_secs}s");
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(SidecarState(Mutex::new(Vec::new())))
        .setup(|app| {
            let program = resolve_anycode_program(app.handle());
            let mut children = Vec::new();
            if let Some(c) = spawn_sidecar("anycode dashboard", &program, &["dashboard"]) {
                children.push(c);
                wait_for_dashboard_port(45);
            }
            if std::env::var("ANYCODE_DESKTOP_WECHAT")
                .ok()
                .is_some_and(|v| matches!(v.as_str(), "1" | "true" | "yes"))
            {
                if let Some(c) = spawn_sidecar(
                    "WeChat bridge",
                    &program,
                    &["channel", "wechat", "--run-as-bridge"],
                ) {
                    children.push(c);
                }
            }
            if let Some(state) = app.try_state::<SidecarState>() {
                *state.0.lock().unwrap() = children;
            }

            let open_i = MenuItem::with_id(app, "open", "Open Workbench", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open_i, &quit_i])?;
            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
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
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
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
