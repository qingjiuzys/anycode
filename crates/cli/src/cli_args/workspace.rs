use super::*;

#[derive(Subcommand, Debug)]
pub(crate) enum WorkspaceCommands {
    /// List recent workspaces
    List {
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Show current workspace status
    Status,
    /// Register a workspace path or refresh last_seen
    Touch { path: Option<PathBuf> },
    /// Set default mode for a workspace
    SetMode { mode: String, path: Option<PathBuf> },
    /// Set default channel profile for a workspace
    SetChannel {
        channel: String,
        path: Option<PathBuf>,
    },
    /// Set a human-readable label for a workspace
    Label {
        label: String,
        path: Option<PathBuf>,
    },
}
