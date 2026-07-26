//! `anycode-daemon` — headless cron scheduler (in-process, no CLI sidecar).

use anycode_channel_bridge::run_builtin_scheduler;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "anycode-daemon", about = "anyCode headless cron scheduler")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run embedded cron scheduler until exit.
    Scheduler,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    match Cli::parse().command {
        Commands::Scheduler => run_builtin_scheduler().await?,
    }
    Ok(())
}
