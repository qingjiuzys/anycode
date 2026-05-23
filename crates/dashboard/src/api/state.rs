use crate::auth_session::SessionStore;
use crate::db::DashboardDb;
use crate::events::EventBus;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: DashboardDb,
    pub events: Arc<EventBus>,
    pub sessions: SessionStore,
    pub version: String,
    pub static_dir: Option<PathBuf>,
    pub workspace_paths: Vec<String>,
    pub tasks_root: PathBuf,
    pub host: String,
    pub port: u16,
    pub started_at: String,
    pub pid: u32,
}
