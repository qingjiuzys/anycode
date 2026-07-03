//! In-process headless daemon: channels and built-in cron scheduler.

use crate::app_config::{load_config_for_session, load_runtime_config, LoadOpts};
use crate::channels;
use crate::scheduler::{self, CronDelivery};
use crate::workspace::touch_project_dir;
use std::path::PathBuf;
use std::time::Duration;

fn default_working_dir() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Run the built-in cron scheduler until exit (holds `scheduler.lock`).
pub async fn run_builtin_scheduler() -> anyhow::Result<()> {
    let working_dir = default_working_dir();
    touch_project_dir(working_dir.clone());
    let config = load_runtime_config(LoadOpts {
        config_file: None,
        ignore_approval: false,
        workspace_overlay_dir: Some(working_dir.clone()),
        ..Default::default()
    })
    .await?;
    scheduler::run_builtin_scheduler(
        config,
        working_dir,
        Duration::from_secs(30),
        None,
        CronDelivery::None,
    )
    .await
}

/// Run WeChat iLink bridge (long-running; embeds scheduler when lock available).
pub async fn run_wechat_bridge() -> anyhow::Result<()> {
    channels::wechat::run_bridged_start(None, "default".to_string(), None, false).await
}

/// Run Telegram long-polling bridge.
pub async fn run_telegram_bridge() -> anyhow::Result<()> {
    let config = load_config_for_session(None, false).await?;
    channels::tg::run_telegram_polling(
        config,
        channels::tg::TelegramRunArgs {
            bot_token: None,
            chat_id: None,
            agent: "default".to_string(),
            directory: None,
        },
    )
    .await
}

/// Run Discord long-polling bridge.
pub async fn run_discord_bridge() -> anyhow::Result<()> {
    let config = load_config_for_session(None, false).await?;
    channels::discord_channel::run_discord_polling(
        config,
        channels::discord_channel::DiscordRunArgs {
            bot_token: None,
            channel_id: None,
            agent: "default".to_string(),
            directory: None,
        },
    )
    .await
}
