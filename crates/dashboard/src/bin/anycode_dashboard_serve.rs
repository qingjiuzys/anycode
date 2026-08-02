//! Standalone Workbench HTTP server for dev and Playwright e2e (replaces `anycode dashboard`).

use anycode_dashboard::load_workspace_paths;
use anycode_dashboard::server::{default_db_path, run_with_shutdown, DashboardConfig};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "anycode-dashboard-serve",
    about = "Run Digital Workbench HTTP server"
)]
struct Args {
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
    #[arg(long, default_value_t = 43_180)]
    port: u16,
    #[arg(long)]
    db: Option<PathBuf>,
    #[arg(long)]
    static_dir: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    let db_path = args
        .db
        .or_else(|| {
            std::env::var("ANYCODE_DASHBOARD_DB")
                .ok()
                .map(PathBuf::from)
        })
        .unwrap_or_else(default_db_path);
    let config = DashboardConfig {
        host: args.host,
        port: args.port,
        db_path,
        static_dir: args.static_dir,
        serve_ui: true,
        version: env!("CARGO_PKG_VERSION").into(),
        desktop_bootstrap_token: None,
        bound_port_tx: None,
    };
    let paths = load_workspace_paths();
    let (_tx, rx) = tokio::sync::oneshot::channel();
    tokio::select! {
        res = run_with_shutdown(config, paths, rx) => res?,
        _ = tokio::signal::ctrl_c() => {},
    }
    Ok(())
}
