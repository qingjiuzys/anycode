use crate::auth_session::SessionStore;
use crate::control::chat_runtime::ChatRuntimeHost;
use crate::control::web_chat::WebChatHub;
use crate::control::web_chat_tail::WebChatTailHub;
use crate::db::DashboardDb;
use crate::events::EventBus;
use std::path::PathBuf;
use std::sync::Arc;

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
}
