//! 交互式 OAuth（浏览器授权 + 本地回调，含 PKCE）用于远程 Streamable HTTP MCP，基于 `rmcp` 的 `OAuthState`。

use axum::extract::{Query, State};
use axum::response::Html;
use axum::routing::get;
use axum::Router;
use rmcp::transport::auth::{AuthError, AuthorizationManager, OAuthState};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{oneshot, Mutex};

/// 登录选项（`mcp_url` 为 MCP 端点，如 `https://example.com/mcp`）。
#[derive(Debug, Clone)]
pub struct McpOAuthLoginOptions {
    pub mcp_url: String,
    pub redirect_host: String,
    pub redirect_port: u16,
    pub callback_path: String,
    pub client_metadata_url: Option<String>,
    pub scopes: Vec<String>,
    pub client_name: Option<String>,
    pub open_browser: bool,
    pub callback_timeout: Duration,
    /// 若指定，将完整 OAuth 凭证（含 refresh token）写入 JSON；运行时可在 `ANYCODE_MCP_SERVERS` 中配置同路径以自动续期。
    pub credentials_store: Option<PathBuf>,
}

impl Default for McpOAuthLoginOptions {
    fn default() -> Self {
        Self {
            mcp_url: String::new(),
            redirect_host: "127.0.0.1".to_string(),
            redirect_port: 9876,
            callback_path: "/callback".to_string(),
            client_metadata_url: None,
            scopes: Vec::new(),
            client_name: Some("anycode".to_string()),
            open_browser: true,
            callback_timeout: Duration::from_secs(15 * 60),
            credentials_store: None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum McpOAuthLoginError {
    #[error("OAuth: {0}")]
    Auth(#[from] AuthError),
    #[error("IO: {0}")]
    Io(#[from] std::io::Error),
    #[error("callback timed out or channel closed")]
    CallbackTimedOut,
    #[error("{0}")]
    Msg(String),
}

#[derive(Deserialize)]
struct CallbackQuery {
    code: String,
    state: String,
}

#[derive(Clone)]
struct CallbackAppState {
    tx: Arc<Mutex<Option<oneshot::Sender<(String, String)>>>>,
}

const HTML_OK: &str =
    "<!DOCTYPE html><html><body><p>Authorization OK. You can close this tab.</p></body></html>";
const HTML_ERR: &str = "<!DOCTYPE html><html><body><p>Callback already handled or invalid. Close this tab and check the terminal.</p></body></html>";

async fn oauth_callback(
    Query(q): Query<CallbackQuery>,
    State(st): State<CallbackAppState>,
) -> Html<&'static str> {
    let mut slot = st.tx.lock().await;
    if let Some(tx) = slot.take() {
        let _ = tx.send((q.code, q.state));
        Html(HTML_OK)
    } else {
        Html(HTML_ERR)
    }
}

fn normalize_callback_path(p: &str) -> String {
    let p = p.trim();
    if p.is_empty() {
        return "/callback".to_string();
    }
    if p.starts_with('/') {
        p.to_string()
    } else {
        format!("/{p}")
    }
}

fn open_browser(url: &str) {
    let _ = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(url).spawn()
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
            .args(["/C", "start", ""])
            .arg(url)
            .spawn()
    } else {
        std::process::Command::new("xdg-open").arg(url).spawn()
    };
}

/// 完成浏览器 OAuth 流程，返回 **access_token** 明文（不含 `Bearer ` 前缀）。
///
/// 将 token 填入 `ANYCODE_MCP_SERVERS` 对应条目的 `bearer_token` / `oauth_token`，或写入文件后配合环境变量使用。
pub async fn mcp_oauth_login(options: McpOAuthLoginOptions) -> Result<String, McpOAuthLoginError> {
    if options.mcp_url.trim().is_empty() {
        return Err(McpOAuthLoginError::Msg("mcp_url is empty".into()));
    }

    let path = normalize_callback_path(&options.callback_path);
    let redirect_uri = format!(
        "http://{}:{}{}",
        options.redirect_host, options.redirect_port, path
    );

    let (tx, rx) = oneshot::channel();
    let app_state = CallbackAppState {
        tx: Arc::new(Mutex::new(Some(tx))),
    };

    let app = Router::new()
        .route(path.as_str(), get(oauth_callback))
        .with_state(app_state);

    let addr = format!("{}:{}", options.redirect_host, options.redirect_port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    let server = axum::serve(listener, app);
    let server_task = tokio::spawn(async move {
        if let Err(e) = server.await {
            tracing::warn!(error = %e, "local OAuth callback server ended");
        }
    });

    let flow_result = async {
        let mut oauth = if let Some(ref path) = options.credentials_store {
            let mut manager = AuthorizationManager::new(options.mcp_url.trim()).await?;
            manager.set_credential_store(crate::mcp_oauth_store::JsonFileCredentialStore::new(
                path.clone(),
            ));
            OAuthState::Unauthorized(manager)
        } else {
            OAuthState::new(options.mcp_url.trim(), None).await?
        };

        let scope_refs: Vec<&str> = options.scopes.iter().map(|s| s.as_str()).collect();
        oauth
            .start_authorization_with_metadata_url(
                &scope_refs,
                &redirect_uri,
                options.client_name.as_deref(),
                options.client_metadata_url.as_deref(),
            )
            .await?;

        let auth_url = oauth.get_authorization_url().await?;
        println!("Open this URL in your browser to authorize:\n{auth_url}\n");
        if options.open_browser {
            open_browser(&auth_url);
        }

        let (code, state) = tokio::time::timeout(options.callback_timeout, rx)
            .await
            .map_err(|_| McpOAuthLoginError::CallbackTimedOut)?
            .map_err(|_| McpOAuthLoginError::CallbackTimedOut)?;

        oauth.handle_callback(&code, &state).await?;

        let manager = oauth.into_authorization_manager().ok_or_else(|| {
            McpOAuthLoginError::Msg("expected authorized OAuth state after callback".into())
        })?;

        Ok(manager.get_access_token().await?)
    }
    .await;

    server_task.abort();

    flow_result
}

#[cfg(test)]
mod tests {
    use super::normalize_callback_path;

    #[test]
    fn normalize_callback_path_defaults() {
        assert_eq!(normalize_callback_path(""), "/callback");
        assert_eq!(normalize_callback_path("  "), "/callback");
        assert_eq!(normalize_callback_path("cb"), "/cb");
        assert_eq!(
            normalize_callback_path("/oauth/callback"),
            "/oauth/callback"
        );
    }
}
