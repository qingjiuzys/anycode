use super::*;

#[derive(Subcommand, Debug)]
pub(crate) enum DashboardSubcommands {
    /// Print service / DB / UI dist status (read-only)
    Status {
        #[arg(long)]
        db: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Diagnose DB, port, dist, loopback binding
    Doctor {
        #[arg(long)]
        db: Option<PathBuf>,
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value_t = 43_180)]
        port: u16,
        #[arg(long)]
        static_dir: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Manage local API tokens (for non-loopback access)
    Token {
        #[command(subcommand)]
        sub: DashboardTokenCommands,
    },
    /// Database maintenance helpers
    Db {
        #[command(subcommand)]
        sub: DashboardDbCommands,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum DashboardTokenCommands {
    /// Create a new API token (plaintext shown once)
    Create {
        #[arg(long, default_value = "dashboard")]
        name: String,
        #[arg(long)]
        expires_days: Option<i64>,
        #[arg(long)]
        db: Option<PathBuf>,
    },
    /// List tokens (prefix only)
    List {
        #[arg(long)]
        db: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Revoke a token by id
    Revoke {
        id: String,
        #[arg(long)]
        db: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum DashboardDbCommands {
    /// Read-only DB operations summary
    Check {
        #[arg(long)]
        db: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Copy projects.db to a backup path
    Backup {
        #[arg(long)]
        db: Option<PathBuf>,
        #[arg(long)]
        output: Option<PathBuf>,
    },
}
