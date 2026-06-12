//! HTTP probes for Telegram / Discord channel setup (verify token, list chats, test message).

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

const TELEGRAM_API: &str = "https://api.telegram.org";
const DISCORD_API: &str = "https://discord.com/api/v10";
const PROBE_TIMEOUT: Duration = Duration::from_secs(15);

/// View Channel + Send Messages + Read Message History
const DISCORD_BOT_PERMISSIONS: u64 = 0x400 | 0x800 | 0x10000;

fn probe_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(PROBE_TIMEOUT)
        .user_agent("anycode-channel-setup/1.0")
        .build()
        .context("build HTTP client")
}

fn normalize_token(token: &str) -> Result<String> {
    let t = token.trim();
    if t.is_empty() {
        anyhow::bail!("token must not be empty");
    }
    Ok(t.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramBotInfo {
    pub id: i64,
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramChatOption {
    pub chat_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordBotInfo {
    pub id: String,
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordTestResult {
    pub message_id: String,
    pub channel_id: String,
}

pub fn discord_invite_url(application_id: &str) -> String {
    format!(
        "https://discord.com/api/oauth2/authorize?client_id={application_id}&permissions={DISCORD_BOT_PERMISSIONS}&scope=bot%20applications.commands"
    )
}

pub async fn verify_telegram_bot(token: &str) -> Result<TelegramBotInfo> {
    let token = normalize_token(token)?;
    let client = probe_client()?;
    let url = format!("{TELEGRAM_API}/bot{token}/getMe");
    let resp = client.get(&url).send().await.context("telegram getMe")?;
    let body: serde_json::Value = resp.json().await.context("parse telegram getMe")?;
    if !body.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
        let desc = body
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("invalid token");
        anyhow::bail!("Telegram API: {desc}");
    }
    let result = body
        .get("result")
        .ok_or_else(|| anyhow!("telegram getMe missing result"))?;
    let id = result
        .get("id")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| anyhow!("telegram getMe missing bot id"))?;
    let username = result
        .get("username")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if username.is_empty() {
        anyhow::bail!("telegram bot has no username");
    }
    Ok(TelegramBotInfo {
        id,
        username,
        first_name: result
            .get("first_name")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    })
}

pub async fn list_telegram_chats(token: &str) -> Result<Vec<TelegramChatOption>> {
    let token = normalize_token(token)?;
    let client = probe_client()?;
    let url = format!("{TELEGRAM_API}/bot{token}/getUpdates?limit=25");
    let resp = client
        .get(&url)
        .send()
        .await
        .context("telegram getUpdates")?;
    let body: serde_json::Value = resp.json().await.context("parse getUpdates")?;
    if !body.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
        let desc = body
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("getUpdates failed");
        anyhow::bail!("Telegram API: {desc}");
    }
    let updates = body
        .get("result")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut by_id: HashMap<String, TelegramChatOption> = HashMap::new();
    for upd in updates {
        let chat = upd.get("message").and_then(|m| m.get("chat")).or_else(|| {
            upd.get("callback_query")
                .and_then(|c| c.get("message"))
                .and_then(|m| m.get("chat"))
        });
        let Some(chat) = chat else { continue };
        let id = chat.get("id").and_then(|v| v.as_i64());
        let Some(id) = id else { continue };
        let chat_id = id.to_string();
        let title = chat
            .get("title")
            .or_else(|| chat.get("first_name"))
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let username = chat
            .get("username")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let chat_type = chat
            .get("type")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        by_id.insert(
            chat_id.clone(),
            TelegramChatOption {
                chat_id,
                title,
                username,
                chat_type,
            },
        );
    }
    let mut out: Vec<TelegramChatOption> = by_id.into_values().collect();
    out.sort_by(|a, b| a.chat_id.cmp(&b.chat_id));
    Ok(out)
}

pub async fn verify_discord_bot(token: &str) -> Result<DiscordBotInfo> {
    let token = normalize_token(token)?;
    let client = probe_client()?;
    let resp = client
        .get(format!("{DISCORD_API}/users/@me"))
        .header("Authorization", format!("Bot {token}"))
        .send()
        .await
        .context("discord users/@me")?;
    let status = resp.status();
    let body: serde_json::Value = resp.json().await.unwrap_or_default();
    if !status.is_success() {
        return Err(discord_api_error(status.as_u16(), &body));
    }
    let id = body
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let username = body
        .get("username")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if id.is_empty() || username.is_empty() {
        anyhow::bail!("discord response missing bot identity");
    }
    Ok(DiscordBotInfo {
        id,
        username,
        global_name: body
            .get("global_name")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    })
}

pub async fn test_discord_channel(token: &str, channel_id: &str) -> Result<DiscordTestResult> {
    let token = normalize_token(token)?;
    let channel_id = channel_id.trim();
    if channel_id.is_empty() {
        anyhow::bail!("channel id must not be empty");
    }
    let client = probe_client()?;
    let url = format!("{DISCORD_API}/channels/{channel_id}/messages");
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bot {token}"))
        .json(&serde_json::json!({
            "content": "✅ anyCode connection test — you can delete this message."
        }))
        .send()
        .await
        .context("discord send test message")?;
    let status = resp.status();
    let body: serde_json::Value = resp.json().await.unwrap_or_default();
    if !status.is_success() {
        return Err(discord_api_error(status.as_u16(), &body));
    }
    let message_id = body
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Ok(DiscordTestResult {
        message_id,
        channel_id: channel_id.to_string(),
    })
}

fn discord_api_error(status: u16, body: &serde_json::Value) -> anyhow::Error {
    let msg = body
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("discord API error");
    let code = body.get("code").and_then(|v| v.as_i64());
    let hint = match status {
        401 => "Check that the bot token is correct.",
        403 => {
            "Invite the bot to your server and grant View Channel + Send Messages in this channel."
        }
        404 => "Channel ID not found — enable Developer Mode and copy the channel ID again.",
        _ => "",
    };
    if hint.is_empty() {
        anyhow!("Discord HTTP {status} (code={code:?}): {msg}")
    } else {
        anyhow!("Discord HTTP {status}: {msg}. {hint}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invite_url_contains_permissions() {
        let url = discord_invite_url("123456789");
        assert!(url.contains("client_id=123456789"));
        assert!(url.contains("permissions="));
    }

    #[tokio::test]
    async fn verify_telegram_rejects_empty() {
        assert!(verify_telegram_bot("  ").await.is_err());
    }

    #[tokio::test]
    async fn verify_discord_rejects_empty() {
        assert!(verify_discord_bot("").await.is_err());
    }
}
