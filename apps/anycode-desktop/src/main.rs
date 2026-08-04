#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "macos")]
mod apple_media;
#[cfg(not(target_os = "macos"))]
#[path = "apple_media_stub.rs"]
mod apple_media;
mod dashboard_backend;

use dashboard_backend::{
    apply_dashboard_env, dashboard_http_ready, desktop_api_base, DashboardServerState,
    start_in_process,
};

use std::time::{Duration, Instant};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, RunEvent, Url,
};

fn wait_for_dashboard_ready(timeout_secs: u64) -> bool {
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    while Instant::now() < deadline {
        if dashboard_http_ready() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(400));
    }
    eprintln!("anycode-desktop: dashboard not HTTP-ready after {timeout_secs}s");
    false
}

fn navigate_workbench(app: &tauri::AppHandle, w: &tauri::WebviewWindow) -> bool {
    let Some(api_base) = desktop_api_base() else {
        return false;
    };
    // Serve bundled UI from the in-process loopback server (same origin as /api/*).
    let bootstrap = app
        .try_state::<DashboardServerState>()
        .and_then(|state| state.take_bootstrap_token());
    let url = match bootstrap.as_deref() {
        Some(token) if !token.is_empty() => {
            format!("{api_base}/api/auth/desktop-bootstrap?token={token}")
        }
        _ => format!("{api_base}/"),
    };
    w.eval(&format!("window.location.replace({url:?});"))
        .is_ok()
}


#[tauri::command]
fn open_external_url(url: String) -> Result<(), String> {
    let url = url.trim();
    if url.is_empty() {
        return Err("empty url".into());
    }
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("unsupported url scheme".into());
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = url;
        Err("open_external_url unsupported on this platform".into())
    }
}

/// Native folder picker (returns absolute path, or null if cancelled).
#[tauri::command]
fn pick_directory(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    app.run_on_main_thread(move || {
        let picked = rfd::FileDialog::new()
            .set_title("选择目录")
            .pick_folder();
        let _ = tx.send(picked.map(|p| p.display().to_string()));
    })
    .map_err(|e| e.to_string())?;
    rx.recv().map_err(|e| e.to_string())
}

fn ensure_existing_path(path: &str) -> Result<&std::path::Path, String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("empty path".into());
    }
    let p = std::path::Path::new(path);
    if !p.exists() {
        return Err(format!("path does not exist: {path}"));
    }
    Ok(p)
}

/// Reveal a local file/directory in Finder / Explorer / file manager.
#[tauri::command]
fn reveal_in_file_manager(path: String) -> Result<(), String> {
    let p = ensure_existing_path(&path)?;
    let path = p.to_string_lossy();
    #[cfg(target_os = "macos")]
    {
        // `-R` selects the item in Finder (reveal), instead of opening it.
        std::process::Command::new("open")
            .args(["-R", path.as_ref()])
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .args(["/select,", path.as_ref()])
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    {
        // Best-effort: open the containing directory.
        let parent = p
            .parent()
            .filter(|parent| parent.as_os_str().len() > 0)
            .unwrap_or(p);
        std::process::Command::new("xdg-open")
            .arg(parent)
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = path;
        Err("reveal_in_file_manager unsupported on this platform".into())
    }
}

/// Open a local file/directory with the OS default application.
#[tauri::command]
fn open_local_path(path: String) -> Result<(), String> {
    let p = ensure_existing_path(&path)?;
    let path = p.to_string_lossy();
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path.as_ref())
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", path.as_ref()])
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path.as_ref())
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = path;
        Err("open_local_path unsupported on this platform".into())
    }
}

fn show_workbench(app: &tauri::AppHandle, ready: bool) {
    let Some(w) = app.get_webview_window("main") else {
        return;
    };
    if ready {
        if !navigate_workbench(app, &w) {
            let _ = w.eval(
                r#"document.body.innerHTML = '<div style="display:grid;place-content:center;height:100vh;font-family:system-ui;background:#09090b;color:#f4f4f5;text-align:center;padding:24px;max-width:420px;margin:0 auto"><div><h2 style="margin:0 0 8px">Workbench 未能启动</h2><p style="color:#a1a1aa;margin:0 0 12px">本地工作台 API 未就绪。</p><ol style="color:#71717a;font-size:13px;margin:0;padding-left:1.2rem;text-align:left;line-height:1.6"><li>完全退出 anyCode（Cmd+Q），再重新打开</li><li>若仍失败，在终端查看 Console.app 或 <code style="color:#d4d4d8">log show --predicate process == \"anycode-desktop\"</code></li></ol></div></div>';"#,
            );
        }
    } else {
        let _ = w.eval(
            r#"document.body.innerHTML = '<div style="display:grid;place-content:center;height:100vh;font-family:system-ui;background:#09090b;color:#f4f4f5;text-align:center;padding:24px;max-width:420px;margin:0 auto"><div><h2 style="margin:0 0 8px">Workbench 未能启动</h2><p style="color:#a1a1aa;margin:0 0 12px">本地工作台服务启动超时。</p><ol style="color:#71717a;font-size:13px;margin:0;padding-left:1.2rem;text-align:left;line-height:1.6"><li>完全退出 anyCode（Cmd+Q），再重新打开</li><li>若仍失败，在终端查看 Console.app 或 <code style="color:#d4d4d8">log show --predicate process == \"anycode-desktop\"</code></li></ol></div></div>';"#,
        );
    }
    let _ = w.show();
    let _ = w.set_focus();
}

fn handle_anycode_deep_link(app: &tauri::AppHandle, url: &Url) {
    if url.scheme() != "anycode" {
        return;
    }
    let Some(code) = url
        .query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.into_owned())
    else {
        return;
    };
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        match anycode_setup::link_device(&code).await {
            Ok(session) => {
                eprintln!(
                    "anycode-desktop: cloud account linked: {}",
                    session.user_email.as_deref().unwrap_or("(unknown)")
                );
                let app_for_ui = app.clone();
                let _ = app.run_on_main_thread(move || {
                    if let Some(w) = app_for_ui.get_webview_window("main") {
                        let _ = w.show();
                        let _ = w.set_focus();
                        let _ = w.eval(
                            "window.dispatchEvent(new CustomEvent('anycode-cloud-linked'));",
                        );
                    }
                });
            }
            Err(e) => eprintln!("anycode-desktop: auth link failed: {e:#}"),
        }
    });
}

fn register_deep_link_handlers(app: &tauri::AppHandle) {
    use tauri_plugin_deep_link::DeepLinkExt;

    #[cfg(any(windows, target_os = "linux"))]
    {
        if let Err(e) = app.deep_link().register_all() {
            eprintln!("anycode-desktop: deep link register_all failed: {e}");
        }
    }

    let handle = app.clone();
    app.deep_link().on_open_url(move |event| {
        for url in event.urls() {
            handle_anycode_deep_link(&handle, &url);
        }
    });

    if let Ok(Some(urls)) = app.deep_link().get_current() {
        for url in urls {
            handle_anycode_deep_link(app, &url);
        }
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            open_external_url,
            reveal_in_file_manager,
            open_local_path,
            pick_directory,
            apple_media::apple_media_capabilities,
            apple_media::apple_media_transcribe,
            apple_media::apple_media_ocr_image,
            apple_media::apple_media_synthesize,
            apple_media::apple_media_read_pasteboard,
            apple_media::apple_media_notify,
        ])
        .manage(DashboardServerState::new())
        .setup(|app| {
            register_deep_link_handlers(app.handle());

            apply_dashboard_env(app.handle());
            start_in_process(app.handle().clone());

            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let dashboard_ok = wait_for_dashboard_ready(90);
                let show_handle = handle.clone();
                let _ = handle.run_on_main_thread(move || {
                    show_workbench(&show_handle, dashboard_ok);
                });
            });

            let open_i = MenuItem::with_id(app, "open", "Open Workbench", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open_i, &quit_i])?;
            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show_workbench(app, dashboard_http_ready()),
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
                        show_workbench(tray.app_handle(), dashboard_http_ready());
                    }
                })
                .build(app)?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while running anycode desktop")
        .run(|app, event| {
            // Deep links are handled via tauri_plugin_deep_link::DeepLinkExt::on_open_url
            // (see register_deep_link_handlers); RunEvent::Opened does not exist in Tauri 2.
            if matches!(event, RunEvent::Exit | RunEvent::ExitRequested { .. }) {
                if let Some(state) = app.try_state::<DashboardServerState>() {
                    state.stop();
                }
            }
        });
}
