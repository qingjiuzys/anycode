use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TelegramCredentials {
    pub bot_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiscordCredentials {
    pub bot_token: String,
    pub channel_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelCredentialsStatus {
    pub telegram: bool,
    pub discord: bool,
    pub wechat: bool,
    pub any_configured: bool,
}

fn channels_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("no home directory")?;
    let p = home.join(".anycode/channels");
    std::fs::create_dir_all(&p)?;
    Ok(p)
}

pub fn save_telegram_credentials(cred: &TelegramCredentials) -> Result<PathBuf> {
    let token = cred.bot_token.trim();
    if token.is_empty() {
        anyhow::bail!("Telegram bot token must not be empty");
    }
    let path = channels_dir()?.join("telegram.json");
    let body = TelegramCredentials {
        bot_token: token.to_string(),
        chat_id: cred
            .chat_id
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
    };
    std::fs::write(&path, serde_json::to_string_pretty(&body)?)?;
    Ok(path)
}

pub fn save_discord_credentials(cred: &DiscordCredentials) -> Result<PathBuf> {
    let token = cred.bot_token.trim();
    let channel_id = cred.channel_id.trim();
    if token.is_empty() {
        anyhow::bail!("Discord bot token must not be empty");
    }
    if channel_id.is_empty() {
        anyhow::bail!("Discord channel id must not be empty");
    }
    let path = channels_dir()?.join("discord.json");
    let body = DiscordCredentials {
        bot_token: token.to_string(),
        channel_id: channel_id.to_string(),
    };
    std::fs::write(&path, serde_json::to_string_pretty(&body)?)?;
    Ok(path)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TelegramCredentialsView {
    pub configured: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiscordCredentialsView {
    pub configured: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelsSettingsView {
    pub telegram: TelegramCredentialsView,
    pub discord: DiscordCredentialsView,
    pub wechat: bool,
    pub platform: String,
    pub telegram_start_command: String,
    pub discord_start_command: String,
}

pub fn load_telegram_credentials() -> Option<TelegramCredentials> {
    let path = dirs::home_dir()?.join(".anycode/channels/telegram.json");
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

pub fn load_discord_credentials() -> Option<DiscordCredentials> {
    let path = dirs::home_dir()?.join(".anycode/channels/discord.json");
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

pub fn build_channels_settings_view() -> ChannelsSettingsView {
    let tg_path = dirs::home_dir().map(|h| h.join(".anycode/channels/telegram.json"));
    let dc_path = dirs::home_dir().map(|h| h.join(".anycode/channels/discord.json"));
    let tg = load_telegram_credentials();
    let dc = load_discord_credentials();
    let platform = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    };
    ChannelsSettingsView {
        telegram: TelegramCredentialsView {
            configured: tg.is_some(),
            chat_id: tg.as_ref().and_then(|c| c.chat_id.clone()),
            path: tg_path
                .filter(|p| p.is_file())
                .map(|p| p.display().to_string()),
        },
        discord: DiscordCredentialsView {
            configured: dc.is_some(),
            channel_id: dc.as_ref().map(|c| c.channel_id.clone()),
            path: dc_path
                .filter(|p| p.is_file())
                .map(|p| p.display().to_string()),
        },
        wechat: dirs::home_dir()
            .map(|h| h.join(".anycode/wechat"))
            .is_some_and(|p| p.is_dir()),
        platform: platform.into(),
        telegram_start_command: "anycode channel telegram".into(),
        discord_start_command: "anycode channel discord".into(),
    }
}

pub fn channel_credentials_status() -> ChannelCredentialsStatus {
    let home = dirs::home_dir();
    let telegram = home
        .as_ref()
        .map(|h| h.join(".anycode/channels/telegram.json"))
        .is_some_and(|p| p.is_file());
    let discord = home
        .as_ref()
        .map(|h| h.join(".anycode/channels/discord.json"))
        .is_some_and(|p| p.is_file());
    let wechat = home
        .as_ref()
        .map(|h| h.join(".anycode/wechat"))
        .is_some_and(|p| p.is_dir());
    ChannelCredentialsStatus {
        telegram,
        discord,
        wechat,
        any_configured: telegram || discord || wechat,
    }
}
