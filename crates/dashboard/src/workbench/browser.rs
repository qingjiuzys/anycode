//! Native CDP browser sessions for workbench (shared with agent tools).
//!
//! ## User path — automated browser testing
//! 1. **Prerequisites:** Chromium (desktop bundle, `ANYCODE_CHROMIUM_PATH`, or Google Chrome).
//! 2. **Enable:** Settings → Notifications → Built-in browser (`browser.enabled` in `~/.anycode/config.json`).
//! 3. **Workbench panel:** Conversations → select a project-bound session → right rail **Browser** (globe) → enter URL.
//! 4. **Agent:** In the same conversation, ask e.g. “用 BrowserNavigate 打开 http://127.0.0.1:43180 并 BrowserSnapshot 检查页面”.
//!    Agent `Browser*` tools bind to the conversation id (`ANYCODE_DASHBOARD_SESSION_ID`); panel and agent share one CDP session.
//! 5. **Unavailable:** Panel shows setup steps; Doctor lists `browser_connector` status.

use anycode_browser::{
    chromium_doctor_message, resolve_chromium_executable, BrowserScreenshot, BrowserService,
    BrowserSessionInfo, BrowserState, LockHolder, ScreencastFrame,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock};

#[derive(Debug, Clone, Serialize)]
pub struct BrowserViewport {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct BrowserScreenshotResponse {
    pub image_base64: String,
    pub viewport: BrowserViewport,
}

impl From<BrowserScreenshot> for BrowserScreenshotResponse {
    fn from(s: BrowserScreenshot) -> Self {
        Self {
            image_base64: s.image_base64,
            viewport: BrowserViewport {
                width: s.viewport.width,
                height: s.viewport.height,
            },
        }
    }
}

pub struct BrowserSessionManager {
    service: Arc<BrowserService>,
}

impl Default for BrowserSessionManager {
    fn default() -> Self {
        Self {
            service: BrowserService::shared(),
        }
    }
}

impl BrowserSessionManager {
    pub fn doctor_message() -> String {
        if resolve_chromium_executable().is_some() {
            chromium_doctor_message()
        } else {
            crate::browser_connector::browser_unavailable_message()
        }
    }

    pub fn ensure_ready() -> Result<(), String> {
        if resolve_chromium_executable().is_some() {
            Ok(())
        } else {
            Err(crate::browser_connector::browser_unavailable_message())
        }
    }

    pub fn service(&self) -> Arc<BrowserService> {
        self.service.clone()
    }

    pub async fn create(
        &self,
        project_id: &str,
        conversation_id: Option<&str>,
    ) -> Result<BrowserSessionInfo> {
        let bind = conversation_id.map(str::to_string);
        self.service
            .create_session(project_id, conversation_id, bind.as_deref())
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    pub async fn navigate(&self, session_id: &str, url: &str) -> Result<BrowserState> {
        self.service
            .navigate_user(session_id, url)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    pub async fn state(&self, session_id: &str) -> Result<BrowserState> {
        self.service
            .state(session_id)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    pub async fn screenshot(&self, session_id: &str) -> Result<BrowserScreenshotResponse> {
        self.service
            .screenshot(session_id)
            .await
            .map(|s| s.into())
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    pub async fn set_lock(&self, session_id: &str, lock: LockHolder) -> Result<LockHolder> {
        self.service
            .set_lock(session_id, lock)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    pub async fn subscribe_screencast(
        &self,
        session_id: &str,
    ) -> Result<tokio::sync::broadcast::Receiver<ScreencastFrame>> {
        self.service
            .subscribe_screencast(session_id)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    pub async fn close(&self, session_id: &str) -> Result<()> {
        self.service
            .close_session(session_id)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))
    }
}

pub fn shared_manager() -> Arc<BrowserSessionManager> {
    static MANAGER: OnceLock<Arc<BrowserSessionManager>> = OnceLock::new();
    MANAGER
        .get_or_init(|| Arc::new(BrowserSessionManager::default()))
        .clone()
}

#[derive(Deserialize)]
pub struct CreateBrowserSessionBody {
    pub project_id: String,
    #[serde(default)]
    pub conversation_id: Option<String>,
}
