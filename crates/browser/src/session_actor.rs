//! Per-session CDP actor — one Tokio task owns Browser + pages.

use crate::ax_tree::fetch_ax_tree_yaml;
use crate::chromium::resolve_chromium_executable;
use crate::error::{BrowserError, BrowserResult};
use crate::policy::{cdp_method_allowed, validate_navigation_url};
use crate::snapshot::snapshot_script;
use crate::types::{
    BrowserScreenshot, BrowserSnapshot, BrowserState, BrowserTabInfo, BrowserViewport, LockHolder,
    ScreencastFrame, ScreencastMetadata,
};
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::page::{
    CaptureScreenshotFormat, EventScreencastFrame, ScreencastFrameAckParams, StartScreencastFormat,
    StartScreencastParams, StopScreencastParams,
};
use chromiumoxide::Page;
use futures::StreamExt;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, oneshot, Mutex};
use uuid::Uuid;

const VIEWPORT_W: u32 = 1280;
const VIEWPORT_H: u32 = 720;

#[derive(Clone)]
pub struct SessionActorHandle {
    inner: Arc<SessionActorInner>,
}

struct SessionActorInner {
    cmd_tx: mpsc::Sender<ActorCmd>,
    lock: Arc<Mutex<LockHolder>>,
}

enum ActorCmd {
    ListTabs(oneshot::Sender<BrowserResult<Vec<BrowserTabInfo>>>),
    NewTab(oneshot::Sender<BrowserResult<String>>),
    CloseTab {
        tab_id: String,
        respond: oneshot::Sender<BrowserResult<()>>,
    },
    SelectTab {
        tab_id: String,
        respond: oneshot::Sender<BrowserResult<()>>,
    },
    Navigate {
        url: String,
        respond: oneshot::Sender<BrowserResult<BrowserState>>,
    },
    State(oneshot::Sender<BrowserResult<BrowserState>>),
    Snapshot {
        root_ref: Option<String>,
        respond: oneshot::Sender<BrowserResult<BrowserSnapshot>>,
    },
    Screenshot(oneshot::Sender<BrowserResult<BrowserScreenshot>>),
    Click {
        ref_id: String,
        respond: oneshot::Sender<BrowserResult<()>>,
    },
    TypeText {
        ref_id: String,
        text: String,
        submit: bool,
        respond: oneshot::Sender<BrowserResult<()>>,
    },
    PressKey {
        key: String,
        respond: oneshot::Sender<BrowserResult<()>>,
    },
    Scroll {
        direction: String,
        amount: i32,
        respond: oneshot::Sender<BrowserResult<()>>,
    },
    Cdp {
        method: String,
        params: Value,
        respond: oneshot::Sender<BrowserResult<Value>>,
    },
    SetLock {
        lock: LockHolder,
        respond: oneshot::Sender<BrowserResult<LockHolder>>,
    },
    SubscribeScreencast(broadcast::Sender<ScreencastFrame>),
    Shutdown,
}

struct TabMeta {
    page: Page,
    url: String,
    title: String,
}

impl SessionActorHandle {
    pub async fn spawn() -> BrowserResult<Self> {
        let chrome = resolve_chromium_executable()
            .ok_or_else(|| BrowserError::Unavailable(crate::chromium::chromium_doctor_message()))?;

        let user_data = std::env::temp_dir().join(format!("anycode-browser-{}", Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&user_data);

        let config = BrowserConfig::builder()
            .chrome_executable(chrome)
            .user_data_dir(&user_data)
            .arg("--headless=new")
            .arg("--disable-gpu")
            .arg("--no-sandbox")
            .arg("--disable-dev-shm-usage")
            .window_size(VIEWPORT_W, VIEWPORT_H)
            .build()
            .map_err(|e| BrowserError::Other(anyhow::anyhow!("{e}")))?;

        let (browser, mut handler) = Browser::launch(config)
            .await
            .map_err(|e| BrowserError::Other(anyhow::Error::from(e)))?;

        tokio::spawn(async move { while handler.next().await.is_some() {} });

        let first_page = browser
            .new_page("about:blank")
            .await
            .map_err(|e| BrowserError::Other(anyhow::Error::from(e)))?;

        let first_id = Uuid::new_v4().to_string();
        let mut tabs = HashMap::new();
        tabs.insert(
            first_id.clone(),
            TabMeta {
                page: first_page,
                url: "about:blank".into(),
                title: String::new(),
            },
        );

        let (cmd_tx, cmd_rx) = mpsc::channel(64);
        let lock = Arc::new(Mutex::new(LockHolder::Idle));

        tokio::spawn(session_loop(cmd_rx, browser, tabs, first_id, lock.clone()));

        Ok(Self {
            inner: Arc::new(SessionActorInner { cmd_tx, lock }),
        })
    }

    async fn send<R>(
        &self,
        build: impl FnOnce(oneshot::Sender<BrowserResult<R>>) -> ActorCmd,
    ) -> BrowserResult<R> {
        let (tx, rx) = oneshot::channel();
        self.inner
            .cmd_tx
            .send(build(tx))
            .await
            .map_err(|_| BrowserError::Unavailable("browser session closed".into()))?;
        rx.await
            .map_err(|_| BrowserError::Unavailable("browser session dropped".into()))?
    }

    pub async fn list_tabs(&self) -> BrowserResult<Vec<BrowserTabInfo>> {
        self.send(ActorCmd::ListTabs).await
    }

    pub async fn new_tab(&self) -> BrowserResult<String> {
        self.send(ActorCmd::NewTab).await
    }

    pub async fn close_tab(&self, tab_id: &str) -> BrowserResult<()> {
        self.send(|r| ActorCmd::CloseTab {
            tab_id: tab_id.into(),
            respond: r,
        })
        .await
    }

    pub async fn select_tab(&self, tab_id: &str) -> BrowserResult<()> {
        self.send(|r| ActorCmd::SelectTab {
            tab_id: tab_id.into(),
            respond: r,
        })
        .await
    }

    pub async fn navigate(&self, url: &str) -> BrowserResult<BrowserState> {
        self.send(|r| ActorCmd::Navigate {
            url: url.into(),
            respond: r,
        })
        .await
    }

    pub async fn state(&self) -> BrowserResult<BrowserState> {
        self.send(ActorCmd::State).await
    }

    pub async fn snapshot(&self, root_ref: Option<&str>) -> BrowserResult<BrowserSnapshot> {
        self.send(|r| ActorCmd::Snapshot {
            root_ref: root_ref.map(str::to_string),
            respond: r,
        })
        .await
    }

    pub async fn screenshot(&self) -> BrowserResult<BrowserScreenshot> {
        self.send(ActorCmd::Screenshot).await
    }

    pub async fn click(&self, ref_id: &str) -> BrowserResult<()> {
        self.send(|r| ActorCmd::Click {
            ref_id: ref_id.into(),
            respond: r,
        })
        .await
    }

    pub async fn type_text(&self, ref_id: &str, text: &str, submit: bool) -> BrowserResult<()> {
        self.send(|r| ActorCmd::TypeText {
            ref_id: ref_id.into(),
            text: text.into(),
            submit,
            respond: r,
        })
        .await
    }

    pub async fn press_key(&self, key: &str) -> BrowserResult<()> {
        self.send(|r| ActorCmd::PressKey {
            key: key.into(),
            respond: r,
        })
        .await
    }

    pub async fn scroll(&self, direction: &str, amount: i32) -> BrowserResult<()> {
        self.send(|r| ActorCmd::Scroll {
            direction: direction.into(),
            amount,
            respond: r,
        })
        .await
    }

    pub async fn cdp(&self, method: &str, params: Value) -> BrowserResult<Value> {
        self.send(|r| ActorCmd::Cdp {
            method: method.into(),
            params,
            respond: r,
        })
        .await
    }

    pub async fn set_lock(&self, lock: LockHolder) -> BrowserResult<LockHolder> {
        self.send(|r| ActorCmd::SetLock { lock, respond: r }).await
    }

    pub fn lock_holder(&self) -> Arc<Mutex<LockHolder>> {
        self.inner.lock.clone()
    }

    pub async fn subscribe_screencast(&self, tx: broadcast::Sender<ScreencastFrame>) {
        let _ = self
            .inner
            .cmd_tx
            .send(ActorCmd::SubscribeScreencast(tx))
            .await;
    }

    pub async fn shutdown(&self) {
        let _ = self.inner.cmd_tx.send(ActorCmd::Shutdown).await;
    }
}

async fn session_loop(
    mut cmd_rx: mpsc::Receiver<ActorCmd>,
    browser: Browser,
    mut tabs: HashMap<String, TabMeta>,
    mut active_tab: String,
    lock: Arc<Mutex<LockHolder>>,
) {
    let mut screencast_tx: Option<broadcast::Sender<ScreencastFrame>> = None;
    let mut screencast_task: Option<tokio::task::JoinHandle<()>> = None;

    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            ActorCmd::ListTabs(respond) => {
                let list = tabs
                    .iter()
                    .map(|(id, t)| BrowserTabInfo {
                        tab_id: id.clone(),
                        url: t.url.clone(),
                        title: t.title.clone(),
                        active: id == &active_tab,
                    })
                    .collect();
                let _ = respond.send(Ok(list));
            }
            ActorCmd::NewTab(respond) => {
                let result = match browser.new_page("about:blank").await {
                    Ok(page) => {
                        let id = Uuid::new_v4().to_string();
                        tabs.insert(
                            id.clone(),
                            TabMeta {
                                page,
                                url: "about:blank".into(),
                                title: String::new(),
                            },
                        );
                        active_tab = id.clone();
                        Ok(id)
                    }
                    Err(e) => Err(BrowserError::Other(e.into())),
                };
                let _ = respond.send(result);
                restart_screencast_if_needed(
                    &screencast_tx,
                    &mut screencast_task,
                    &tabs,
                    &active_tab,
                );
            }
            ActorCmd::CloseTab { tab_id, respond } => {
                let result = if tabs.len() <= 1 {
                    Err(BrowserError::Other(anyhow::anyhow!(
                        "cannot close last tab"
                    )))
                } else if let Some(entry) = tabs.remove(&tab_id) {
                    let _ = entry.page.close().await;
                    if active_tab == tab_id {
                        active_tab = tabs.keys().next().cloned().unwrap_or_default();
                    }
                    Ok(())
                } else {
                    Err(BrowserError::TabNotFound(tab_id))
                };
                let _ = respond.send(result);
            }
            ActorCmd::SelectTab { tab_id, respond } => {
                let result = if tabs.contains_key(&tab_id) {
                    active_tab = tab_id;
                    Ok(())
                } else {
                    Err(BrowserError::TabNotFound(tab_id))
                };
                let _ = respond.send(result);
                restart_screencast_if_needed(
                    &screencast_tx,
                    &mut screencast_task,
                    &tabs,
                    &active_tab,
                );
            }
            ActorCmd::Navigate { url, respond } => {
                let result = navigate_tab(&mut tabs, &active_tab, &url, &lock).await;
                let _ = respond.send(result);
                restart_screencast_if_needed(
                    &screencast_tx,
                    &mut screencast_task,
                    &tabs,
                    &active_tab,
                );
            }
            ActorCmd::State(respond) => {
                let result = state_tab(&tabs, &active_tab, &lock).await;
                let _ = respond.send(result);
            }
            ActorCmd::Snapshot { root_ref, respond } => {
                let result = snapshot_tab(&tabs, &active_tab, root_ref.as_deref()).await;
                let _ = respond.send(result);
            }
            ActorCmd::Screenshot(respond) => {
                let result = screenshot_tab(&tabs, &active_tab).await;
                let _ = respond.send(result);
            }
            ActorCmd::Click { ref_id, respond } => {
                let result = click_ref(&tabs, &active_tab, &ref_id).await;
                let _ = respond.send(result);
            }
            ActorCmd::TypeText {
                ref_id,
                text,
                submit,
                respond,
            } => {
                let result = type_ref(&tabs, &active_tab, &ref_id, &text, submit).await;
                let _ = respond.send(result);
            }
            ActorCmd::PressKey { key, respond } => {
                let result = press_key(&tabs, &active_tab, &key).await;
                let _ = respond.send(result);
            }
            ActorCmd::Scroll {
                direction,
                amount,
                respond,
            } => {
                let result = scroll_tab(&tabs, &active_tab, &direction, amount).await;
                let _ = respond.send(result);
            }
            ActorCmd::Cdp {
                method,
                params,
                respond,
            } => {
                let result = cdp_tab(&tabs, &active_tab, &method, params).await;
                let _ = respond.send(result);
            }
            ActorCmd::SetLock {
                lock: new_lock,
                respond,
            } => {
                let mut g = lock.lock().await;
                *g = new_lock;
                let _ = respond.send(Ok(new_lock));
            }
            ActorCmd::SubscribeScreencast(tx) => {
                screencast_tx = Some(tx.clone());
                if let Some(page) = tabs.get(&active_tab).map(|t| t.page.clone()) {
                    if let Some(task) = screencast_task.take() {
                        task.abort();
                    }
                    screencast_task = Some(spawn_screencast_task(page, tx));
                }
            }
            ActorCmd::Shutdown => {
                if let Some(task) = screencast_task.take() {
                    task.abort();
                }
                break;
            }
        }
    }
    if let Some(task) = screencast_task.take() {
        task.abort();
    }
}

fn spawn_screencast_task(
    page: Page,
    tx: broadcast::Sender<ScreencastFrame>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let params = StartScreencastParams::builder()
            .format(StartScreencastFormat::Jpeg)
            .quality(80_i64)
            .every_nth_frame(2_i64)
            .build();
        if page.execute(params).await.is_err() {
            return;
        }
        let mut events = match page.event_listener::<EventScreencastFrame>().await {
            Ok(stream) => stream,
            Err(_) => return,
        };
        while let Some(frame) = events.next().await {
            if tx.receiver_count() == 0 {
                continue;
            }
            let meta = &frame.metadata;
            let image_base64 =
                <chromiumoxide::Binary as AsRef<str>>::as_ref(&frame.data).to_string();
            let _ = tx.send(ScreencastFrame {
                image_base64,
                metadata: ScreencastMetadata {
                    offset_top: meta.offset_top as i64,
                    page_scale_factor: meta.page_scale_factor,
                    device_width: meta.device_width as i64,
                    device_height: meta.device_height as i64,
                    scroll_offset_x: meta.scroll_offset_x as i64,
                    scroll_offset_y: meta.scroll_offset_y as i64,
                    timestamp: None,
                },
            });
            if let Ok(ack) = ScreencastFrameAckParams::builder()
                .session_id(frame.session_id)
                .build()
            {
                let _ = page.execute(ack).await;
            }
        }
        let _ = page.execute(StopScreencastParams::default()).await;
    })
}

fn restart_screencast_if_needed(
    screencast_tx: &Option<broadcast::Sender<ScreencastFrame>>,
    screencast_task: &mut Option<tokio::task::JoinHandle<()>>,
    tabs: &HashMap<String, TabMeta>,
    active_tab: &str,
) {
    let Some(tx) = screencast_tx.clone() else {
        return;
    };
    let Some(page) = tabs.get(active_tab).map(|t| t.page.clone()) else {
        return;
    };
    if let Some(task) = screencast_task.take() {
        task.abort();
    }
    *screencast_task = Some(spawn_screencast_task(page, tx));
}

async fn page_url_string(page: &Page, fallback: &str) -> String {
    page.url()
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| fallback.to_string())
}

async fn active_page<'a>(
    tabs: &'a HashMap<String, TabMeta>,
    active: &str,
) -> BrowserResult<&'a Page> {
    tabs.get(active)
        .map(|t| &t.page)
        .ok_or_else(|| BrowserError::TabNotFound(active.to_string()))
}

async fn navigate_tab(
    tabs: &mut HashMap<String, TabMeta>,
    active: &str,
    url: &str,
    lock: &Arc<Mutex<LockHolder>>,
) -> BrowserResult<BrowserState> {
    let _url = validate_navigation_url(url)?;
    let entry = tabs
        .get_mut(active)
        .ok_or_else(|| BrowserError::TabNotFound(active.to_string()))?;
    entry
        .page
        .goto(url)
        .await
        .map_err(|e| BrowserError::Other(e.into()))?;
    entry.url = page_url_string(&entry.page, url).await;
    entry.title = entry
        .page
        .get_title()
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    state_tab(tabs, active, lock).await
}

async fn state_tab(
    tabs: &HashMap<String, TabMeta>,
    active: &str,
    lock: &Arc<Mutex<LockHolder>>,
) -> BrowserResult<BrowserState> {
    let entry = tabs
        .get(active)
        .ok_or_else(|| BrowserError::TabNotFound(active.to_string()))?;
    let url = page_url_string(&entry.page, &entry.url).await;
    let title = entry
        .page
        .get_title()
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| entry.title.clone());
    let lock_val = *lock.lock().await;
    Ok(BrowserState {
        url,
        title,
        lock: lock_val,
    })
}

async fn snapshot_tab(
    tabs: &HashMap<String, TabMeta>,
    active: &str,
    root_ref: Option<&str>,
) -> BrowserResult<BrowserSnapshot> {
    let page = active_page(tabs, active).await?;
    #[derive(Deserialize)]
    struct Snap {
        title: String,
        url: String,
        yaml: String,
    }
    let snap: Snap = page
        .evaluate(snapshot_script(root_ref).as_str())
        .await
        .map_err(|e| BrowserError::Other(e.into()))?
        .into_value()
        .map_err(|e| BrowserError::Other(e.into()))?;

    let mut yaml = snap.yaml;
    if root_ref.is_none() {
        if let Some(ax) = fetch_ax_tree_yaml(page).await {
            if !ax.is_empty() {
                yaml = format!(
                    "{yaml}\n\n# Accessibility tree (CDP supplement; use ref= for clicks)\n{ax}"
                );
            }
        }
    }

    Ok(BrowserSnapshot {
        url: snap.url,
        title: snap.title,
        yaml,
    })
}

async fn screenshot_tab(
    tabs: &HashMap<String, TabMeta>,
    active: &str,
) -> BrowserResult<BrowserScreenshot> {
    let page = active_page(tabs, active).await?;
    let bytes = page
        .screenshot(
            chromiumoxide::page::ScreenshotParams::builder()
                .format(CaptureScreenshotFormat::Png)
                .full_page(false)
                .build(),
        )
        .await
        .map_err(|e| BrowserError::Other(e.into()))?;
    Ok(BrowserScreenshot {
        image_base64: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes),
        viewport: BrowserViewport {
            width: VIEWPORT_W,
            height: VIEWPORT_H,
        },
    })
}

async fn click_ref(
    tabs: &HashMap<String, TabMeta>,
    active: &str,
    ref_id: &str,
) -> BrowserResult<()> {
    let page = active_page(tabs, active).await?;
    let script = format!(
        r#"((ref) => {{
          const el = window.__anycodeBrowserRefs && window.__anycodeBrowserRefs[ref];
          if (!el) throw new Error('ref not found: ' + ref);
          el.click();
          return true;
        }})("{ref_id}")"#
    );
    page.evaluate(script.as_str())
        .await
        .map_err(|e| BrowserError::RefNotFound(format!("{ref_id}: {e}")))?;
    Ok(())
}

async fn type_ref(
    tabs: &HashMap<String, TabMeta>,
    active: &str,
    ref_id: &str,
    text: &str,
    submit: bool,
) -> BrowserResult<()> {
    let page = active_page(tabs, active).await?;
    let escaped = serde_json::to_string(text).unwrap_or_else(|_| "\"\"".into());
    let script = format!(
        r#"((ref, text, submit) => {{
          const el = window.__anycodeBrowserRefs && window.__anycodeBrowserRefs[ref];
          if (!el) throw new Error('ref not found: ' + ref);
          el.focus();
          if ('value' in el) el.value = text;
          else el.textContent = text;
          el.dispatchEvent(new Event('input', {{ bubbles: true }}));
          if (submit) el.dispatchEvent(new KeyboardEvent('keydown', {{ key: 'Enter', bubbles: true }}));
          return true;
        }})("{ref_id}", {escaped}, {submit})"#,
        submit = submit
    );
    page.evaluate(script.as_str())
        .await
        .map_err(|e| BrowserError::RefNotFound(format!("{ref_id}: {e}")))?;
    Ok(())
}

async fn press_key(tabs: &HashMap<String, TabMeta>, active: &str, key: &str) -> BrowserResult<()> {
    let page = active_page(tabs, active).await?;
    let escaped = serde_json::to_string(key).unwrap_or_else(|_| "\"\"".into());
    let script = format!(
        r#"((key) => {{
          document.activeElement?.dispatchEvent(new KeyboardEvent('keydown', {{ key, bubbles: true }}));
          return true;
        }})({escaped})"#
    );
    page.evaluate(script.as_str())
        .await
        .map_err(|e| BrowserError::Other(e.into()))?;
    Ok(())
}

async fn scroll_tab(
    tabs: &HashMap<String, TabMeta>,
    active: &str,
    direction: &str,
    amount: i32,
) -> BrowserResult<()> {
    let page = active_page(tabs, active).await?;
    let (dx, dy) = match direction {
        "up" => (0, -amount),
        "down" => (0, amount),
        "left" => (-amount, 0),
        "right" => (amount, 0),
        _ => (0, amount),
    };
    let script = format!("window.scrollBy({dx}, {dy}); true");
    page.evaluate(script.as_str())
        .await
        .map_err(|e| BrowserError::Other(e.into()))?;
    Ok(())
}

async fn cdp_tab(
    tabs: &HashMap<String, TabMeta>,
    active: &str,
    method: &str,
    params: Value,
) -> BrowserResult<Value> {
    if !cdp_method_allowed(method) {
        return Err(BrowserError::CdpDenied(method.to_string()));
    }
    let page = active_page(tabs, active).await?;
    if method == "Runtime.evaluate" {
        let expr = params
            .get("expression")
            .and_then(|v| v.as_str())
            .unwrap_or("null");
        let val = page
            .evaluate(expr)
            .await
            .map_err(|e| BrowserError::Other(anyhow::Error::from(e)))?;
        return Ok(serde_json::json!({ "result": val.value() }));
    }
    if method == "Page.captureScreenshot" {
        let shot = screenshot_tab(tabs, active).await?;
        return Ok(serde_json::json!({ "data": shot.image_base64 }));
    }
    Err(BrowserError::CdpDenied(format!(
        "method {method} not wired yet"
    )))
}
