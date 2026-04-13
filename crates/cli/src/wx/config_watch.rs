//! `config.json` 文件系统监听 + debounce，驱动微信桥 LLM 运行时热更新。

use crate::app_config::{
    apply_wechat_bridge_no_tool_approval, load_config_for_session, resolve_config_path,
};
use crate::bootstrap::initialize_runtime;
use anycode_agent::AgentRuntime;
use notify::{
    Config as NotifyConfig, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tracing::{info, warn};

const DEBOUNCE_MS: u64 = 320;

/// 与 `BridgeState` 共享：notify 线程与轮询路径都据此重载。
#[derive(Clone)]
pub(crate) struct ConfigReloadHandle {
    pub runtime: Arc<RwLock<Arc<AgentRuntime>>>,
    pub config_file: Option<PathBuf>,
    pub ignore_approval: bool,
    pub last_config_mtime: Arc<StdMutex<Option<SystemTime>>>,
}

/// `config.json` 保存后 mtime 变化则重建 `AgentRuntime`（与 notify 共用同一套 mtime 去重）。
pub(crate) async fn reload_runtime_if_config_changed(handle: &ConfigReloadHandle) {
    let path = match resolve_config_path(handle.config_file.clone()) {
        Ok(p) => p,
        Err(e) => {
            tracing::debug!(target: "anycode_cli", "wx bridge: resolve config path: {e}");
            return;
        }
    };
    let mtime = match std::fs::metadata(&path) {
        Ok(m) => m.modified().ok(),
        Err(_) => return,
    };
    {
        let guard = handle
            .last_config_mtime
            .lock()
            .expect("last_config_mtime lock");
        if *guard == mtime {
            return;
        }
    }

    let mut cfg =
        match load_config_for_session(handle.config_file.clone(), handle.ignore_approval).await {
            Ok(c) => c,
            Err(e) => {
                warn!(
                    target: "anycode_cli",
                    "wx bridge: config reload skipped (parse/load): {e}"
                );
                *handle.last_config_mtime.lock().expect("mtime") = mtime;
                return;
            }
        };
    apply_wechat_bridge_no_tool_approval(&mut cfg);
    let new_rt = match initialize_runtime(&cfg, None, None).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(
                target: "anycode_cli",
                "wx bridge: runtime reinit failed, keeping previous runtime: {e}"
            );
            *handle.last_config_mtime.lock().expect("mtime") = mtime;
            return;
        }
    };

    *handle.runtime.write().await = new_rt;
    *handle.last_config_mtime.lock().expect("mtime") = mtime;
    info!(
        target: "anycode_cli",
        "wx bridge: config hot-reloaded (model / routing / credentials)"
    );
}

fn event_concerns_config_path(ev: &Event, config_path: &Path) -> bool {
    let want = match config_path.file_name().and_then(|n| n.to_str()) {
        Some(w) => w,
        None => return false,
    };
    ev.paths.iter().any(|p| {
        p.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n == want)
    })
}

fn run_notify_loop(config_path: PathBuf, tx: mpsc::UnboundedSender<()>) {
    let (event_tx, event_rx) = std::sync::mpsc::channel();
    let mut watcher: RecommendedWatcher =
        match RecommendedWatcher::new(event_tx, NotifyConfig::default()) {
            Ok(w) => w,
            Err(e) => {
                warn!(
                    target: "anycode_cli",
                    "wx bridge: notify watcher create failed ({e}); poll-only hot-reload"
                );
                return;
            }
        };

    let to_watch = config_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(config_path.as_path());

    if let Err(e) = watcher.watch(to_watch, RecursiveMode::NonRecursive) {
        warn!(
            target: "anycode_cli",
            "wx bridge: notify watch {:?} failed ({e}); poll-only hot-reload",
            to_watch
        );
        return;
    }

    info!(
        target: "anycode_cli",
        "wx bridge: file watcher on {:?} (config {:?})",
        to_watch,
        config_path.file_name().and_then(|s| s.to_str()).unwrap_or("?")
    );

    loop {
        match event_rx.recv() {
            Ok(Ok(ev)) => {
                if !event_concerns_config_path(&ev, &config_path) {
                    continue;
                }
                match ev.kind {
                    EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => {
                        let _ = tx.send(());
                    }
                    _ => {}
                }
            }
            Ok(Err(e)) => {
                warn!(target: "anycode_cli", "wx bridge: notify error: {e}");
            }
            Err(_) => break,
        }
    }
}

/// 后台监听配置路径；失败时仅打日志，仍依赖轮询 `mtime` 后备。
pub(crate) fn spawn_config_file_watcher(handle: ConfigReloadHandle, config_path: PathBuf) {
    let (tx, mut rx) = mpsc::unbounded_channel::<()>();
    let watch_path = config_path.clone();
    if std::thread::Builder::new()
        .name("anycode-wx-config-notify".into())
        .spawn(move || run_notify_loop(watch_path, tx))
        .is_err()
    {
        warn!(target: "anycode_cli", "wx bridge: could not spawn notify thread; poll-only hot-reload");
        return;
    }

    tokio::spawn(async move {
        while rx.recv().await.is_some() {
            while rx.try_recv().is_ok() {}
            tokio::time::sleep(Duration::from_millis(DEBOUNCE_MS)).await;
            reload_runtime_if_config_changed(&handle).await;
        }
    });
}
