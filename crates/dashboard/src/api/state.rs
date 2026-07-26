use crate::auth_session::SessionStore;
use crate::control::chat_runtime::ChatRuntimeHost;
use crate::control::web_chat::WebChatHub;
use crate::control::web_chat_tail::WebChatTailHub;
use crate::db::DashboardDb;
use crate::events::EventBus;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AppState {
    pub db: DashboardDb,
    pub events: Arc<EventBus>,
    pub sessions: SessionStore,
    pub web_chat: WebChatHub,
    pub web_chat_tail: WebChatTailHub,
    pub chat_runtime: ChatRuntimeHost,
    pub version: String,
    pub static_dir: Option<PathBuf>,
    pub serve_ui: bool,
    pub workspace_paths: Vec<String>,
    pub tasks_root: PathBuf,
    pub host: String,
    pub port: u16,
    pub started_at: String,
    pub pid: u32,
    pub managed_local_llm: crate::managed_local_llm::ManagedLocalLlm,
    /// One-shot Desktop bootstrap token (process memory only). Consumed by
    /// `/api/auth/desktop-bootstrap` to mint a local `dw_session` cookie.
    pub desktop_bootstrap_token: Arc<Mutex<Option<String>>>,
    /// Loopback auth bypass for CI/e2e. Frozen at startup from
    /// `ANYCODE_DASHBOARD_TEST_AUTH_BYPASS` (per-app in tests) so parallel
    /// test apps no longer race on a process-global env var.
    pub test_auth_bypass: bool,
}
