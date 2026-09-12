use crate::auth_session::SessionStore;
use crate::control::chat_runtime::ChatRuntimeHost;
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
    /// One-shot Desktop bootstrap token (process memory only). Consumed by
    /// `/api/auth/desktop-bootstrap` to mint a local `dw_session` cookie.
    pub desktop_bootstrap_token: Arc<Mutex<Option<String>>>,
    /// Loopback auth bypass for CI/e2e. Frozen at startup from
    /// `ANYCODE_DASHBOARD_TEST_AUTH_BYPASS` (per-app in tests) so parallel
    /// test apps no longer race on a process-global env var.
    pub test_auth_bypass: bool,
    /// Whether this dashboard instance is embedded in the Desktop shell.
    /// Frozen once at construction from `ANYCODE_DASHBOARD_EMBEDDED_DESKTOP`
    /// (per-app in tests) so auth, origin, bootstrap and SPA marker decisions
    /// no longer depend on a mutable process-global env var.
    pub embedded_desktop: bool,
    /// LAN colleague discovery and handoff (optional).
    pub lan_hub: Option<std::sync::Arc<crate::lan::LanHub>>,
    /// Opt-in GraphRunner product routes. Default off.
    pub harness_graph: bool,
    /// Opt-in unified Kernel adapters. Default off; not implied by graph.
    pub harness_unified_kernel: bool,
    /// Per-project gray allowlist captured at construction. Empty means all local projects.
    pub harness_gray_projects: Vec<String>,
    /// Server-only 818cloud introspect client. Desktop must leave this `None`.
    pub accounts_sso: Option<std::sync::Arc<anycode_harness_cloud818::identity::AccountsClient>>,
    /// Registered SSO v2 redirect, frozen at construction. Desktop never chooses it.
    pub sso_redirect: Option<String>,
    /// In-memory PKCE hops for Desktop pairing. Tokens are one-shot and never logged.
    pub sso_hops: std::sync::Arc<SsoHopStore>,
}

#[derive(Default)]
pub struct SsoHopStore {
    inner: tokio::sync::Mutex<SsoHopMaps>,
}

#[derive(Default)]
struct SsoHopMaps {
    by_poll: std::collections::HashMap<String, SsoHop>,
    by_state: std::collections::HashMap<String, String>,
}

#[derive(Clone)]
pub struct SsoHop {
    pub verifier: String,
    pub state: String,
    pub redirect_uri: String,
    pub access_token: Option<String>,
    pub created: std::time::Instant,
}

impl SsoHopStore {
    pub async fn insert(&self, poll_token: String, hop: SsoHop) {
        let mut maps = self.inner.lock().await;
        maps.by_state.insert(hop.state.clone(), poll_token.clone());
        maps.by_poll.insert(poll_token, hop);
    }

    pub async fn by_poll(&self, poll_token: &str) -> Option<SsoHop> {
        self.inner.lock().await.by_poll.get(poll_token).cloned()
    }

    pub async fn by_state(&self, state: &str) -> Option<(String, SsoHop)> {
        let maps = self.inner.lock().await;
        let poll = maps.by_state.get(state)?.clone();
        let hop = maps.by_poll.get(&poll)?.clone();
        Some((poll, hop))
    }

    pub async fn set_access_token(&self, poll_token: &str, token: String) -> bool {
        let mut maps = self.inner.lock().await;
        if let Some(hop) = maps.by_poll.get_mut(poll_token) {
            hop.access_token = Some(token);
            return true;
        }
        false
    }

    pub async fn consume_if_ready(&self, poll_token: &str) -> SsoHopPoll {
        let mut maps = self.inner.lock().await;
        let Some(hop) = maps.by_poll.get(poll_token) else {
            return SsoHopPoll::Unknown;
        };
        if hop.created.elapsed() > std::time::Duration::from_secs(300) {
            let state = hop.state.clone();
            maps.by_poll.remove(poll_token);
            maps.by_state.remove(&state);
            return SsoHopPoll::Expired;
        }
        let Some(token) = hop.access_token.clone() else {
            return SsoHopPoll::Pending;
        };
        let state = hop.state.clone();
        maps.by_poll.remove(poll_token);
        maps.by_state.remove(&state);
        SsoHopPoll::Ready(token)
    }
}

pub enum SsoHopPoll {
    Unknown,
    Pending,
    Expired,
    Ready(String),
}

/// Confidential SSO client for the Workbench server. Desktop / Tauri never
/// receives `PRODUCT_SSO_CLIENT_SECRET` — this returns `None` when embedded.
pub fn accounts_client_if_server_side(
    embedded_desktop: bool,
) -> Option<std::sync::Arc<anycode_harness_cloud818::identity::AccountsClient>> {
    if embedded_desktop {
        return None;
    }
    let issuer = std::env::var("ANYCODE_HARNESS_SSO_ISSUER").ok()?;
    let secret = std::env::var("PRODUCT_SSO_CLIENT_SECRET").ok()?;
    let loopback = issuer.starts_with("http://127.0.0.1:")
        || issuer.starts_with("http://127.0.0.1/")
        || issuer == "http://127.0.0.1"
        || issuer.starts_with("http://localhost:")
        || issuer.starts_with("http://localhost/")
        || issuer == "http://localhost";
    anycode_harness_cloud818::identity::AccountsClient::new(&issuer, secret, loopback)
        .ok()
        .map(std::sync::Arc::new)
}
