use super::*;

#[derive(Subcommand, Debug)]
pub(crate) enum McpCommands {
    /// Print MCP env/policy diagnostics (no live server connection required)
    Status {
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Browser OAuth for remote MCP; prints access token (for ANYCODE_MCP_SERVERS `bearer_token`)
    #[cfg(feature = "mcp-oauth")]
    OauthLogin {
        /// MCP endpoint URL, e.g. https://example.com/mcp
        #[arg(long)]
        url: String,
        /// Local callback bind address (127.0.0.1 recommended)
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        /// Local callback port
        #[arg(long, default_value_t = 9876)]
        port: u16,
        /// Callback path (must match redirect_uri; default /callback)
        #[arg(long, default_value = "/callback")]
        callback_path: String,
        /// OAuth client metadata URL (SEP-991; optional for most servers)
        #[arg(long)]
        client_metadata_url: Option<String>,
        /// Requested OAuth scopes (repeat flag)
        #[arg(long = "scope", action = clap::ArgAction::Append)]
        scopes: Vec<String>,
        /// Do not open browser; only print the authorization URL
        #[arg(long, default_value_t = false)]
        no_browser: bool,
        /// Write access_token as one line to this file (mind file permissions)
        #[arg(long)]
        write_token: Option<PathBuf>,
        /// Write full OAuth JSON (refresh_token) for ANYCODE_MCP_SERVERS `oauth_credentials_path` auto-refresh
        #[arg(long)]
        credentials_store: Option<PathBuf>,
    },
}
