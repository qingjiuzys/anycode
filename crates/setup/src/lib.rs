//! Shared first-run setup: config readiness, memory presets, workspace layout, channel helpers.

mod channel_probe;
mod channels;
mod config;
mod memory;
mod quick_auth;
mod status;
mod wechat_ilink;
mod workspace;

pub use channel_probe::{
    discord_invite_url, list_telegram_chats, test_discord_channel, verify_discord_bot,
    verify_telegram_bot, DiscordBotInfo, DiscordTestResult, TelegramBotInfo, TelegramChatOption,
};
pub use channels::{
    build_channels_settings_view, channel_credentials_status, load_discord_credentials,
    load_telegram_credentials, save_discord_credentials, save_telegram_credentials,
    ChannelCredentialsStatus, ChannelsSettingsView, DiscordCredentials, DiscordCredentialsView,
    TelegramCredentials, TelegramCredentialsView,
};
pub use config::has_usable_model_config;
pub use memory::{apply_memory_preset, memory_preset_from_label, MemorySetupPreset};
pub use quick_auth::{quick_auth_presets, QuickAuthChoice, QUICK_AUTH_CHOICES};
pub use status::{
    build_setup_status, load_setup_status, SetupStatus, SetupStepId, SetupStepStatus,
};
pub use wechat_ilink::{fetch_wechat_qr, poll_wechat_qr_status, WechatQrPayload, WechatQrStatus};
pub use workspace::{ensure_layout, workspace_root};
