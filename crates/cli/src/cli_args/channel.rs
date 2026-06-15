use super::*;

#[derive(Subcommand, Debug)]
pub(crate) enum ChannelCommands {
    /// Print local channel bridge status hints (credentials, last cron target, scheduler lock)
    Status {
        /// Channel name: wechat / telegram / discord / all
        #[arg(default_value = "all")]
        channel: String,
        /// JSON output
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// WeChat: scan to bind; installs login autostart bridge on success
    Wechat {
        /// Data directory (default ~/.anycode/wechat; `WCC_DATA_DIR` for legacy wechat-claude-code paths)
        #[arg(long, env = "WCC_DATA_DIR")]
        data_dir: Option<PathBuf>,
        /// Invoked by LaunchAgent/systemd to run the message bridge (do not use manually)
        #[arg(long, hide = true)]
        run_as_bridge: bool,
        /// Same as `anycode run --agent` (only with `--run-as-bridge`)
        #[arg(long, default_value = "workspace-assistant", hide = true)]
        agent: String,
    },
    /// Send a redacted real WeChat test message to the last channel target
    #[command(hide = true)]
    WechatSendTest {
        /// Test message body. Include a unique marker such as [anycode-e2e:<run_id>].
        #[arg(long)]
        message: String,
        /// Data directory (default ~/.anycode/wechat; `WCC_DATA_DIR` for legacy wechat-claude-code paths)
        #[arg(long, env = "WCC_DATA_DIR")]
        data_dir: Option<PathBuf>,
        /// JSON output
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Send a local file (image / video / xlsx / pdf / …) via CDN to the last WeChat target
    #[command(hide = true)]
    WechatSendMediaTest {
        /// Local file path to send
        #[arg(long)]
        path: String,
        /// Optional caption sent before the media item
        #[arg(long)]
        caption: Option<String>,
        /// Data directory (default ~/.anycode/wechat)
        #[arg(long, env = "WCC_DATA_DIR")]
        data_dir: Option<PathBuf>,
        /// JSON output
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Telegram bridge (Bot Token + polling)
    Telegram {
        /// Telegram bot token (fallback: TELEGRAM_BOT_TOKEN)
        #[arg(long)]
        bot_token: Option<String>,
        /// Limit processing to one chat id (optional)
        #[arg(long)]
        chat_id: Option<String>,
        /// Agent type
        #[arg(short, long, default_value = "workspace-assistant")]
        agent: String,
        /// Working directory
        #[arg(short = 'C', long)]
        directory: Option<PathBuf>,
    },
    /// Discord bridge (Bot Token + channel polling)
    Discord {
        /// Discord bot token (fallback: DISCORD_BOT_TOKEN)
        #[arg(long)]
        bot_token: Option<String>,
        /// Discord channel id (fallback: DISCORD_CHANNEL_ID)
        #[arg(long)]
        channel_id: Option<String>,
        /// Agent type
        #[arg(short, long, default_value = "workspace-assistant")]
        agent: String,
        /// Working directory
        #[arg(short = 'C', long)]
        directory: Option<PathBuf>,
    },
    /// Persist Telegram bot token to `~/.anycode/channels/telegram.json` (for later `channel telegram` without flags)
    TelegramSetToken {
        #[arg(long)]
        token: String,
        #[arg(long)]
        chat_id: Option<String>,
    },
    /// Persist Discord bot token + channel id to `~/.anycode/channels/discord.json`
    DiscordSetToken {
        #[arg(long)]
        token: String,
        #[arg(long)]
        channel_id: String,
    },
}
