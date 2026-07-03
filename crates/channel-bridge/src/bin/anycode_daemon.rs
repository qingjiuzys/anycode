//! `anycode-daemon` — headless channels + cron (in-process, no CLI sidecar).

use anycode_channel_bridge::{
    run_builtin_scheduler, run_discord_bridge, run_telegram_bridge, run_wechat_bridge,
};
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "anycode-daemon", about = "anyCode headless channels and cron")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run embedded cron scheduler until exit.
    Scheduler,
    /// Run WeChat iLink bridge.
    WechatBridge,
    /// Run Telegram long-polling bridge.
    TelegramBridge,
    /// Run Discord long-polling bridge.
    DiscordBridge,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    match Cli::parse().command {
        Commands::Scheduler => run_builtin_scheduler().await?,
        Commands::WechatBridge => run_wechat_bridge().await?,
        Commands::TelegramBridge => run_telegram_bridge().await?,
        Commands::DiscordBridge => run_discord_bridge().await?,
    }
    Ok(())
}
