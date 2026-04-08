use crate::app_config::{apply_wechat_bridge_no_tool_approval, Config};
use crate::bootstrap::initialize_runtime;
use crate::channel_task::{build_channel_task, ChannelTaskInput};
use anycode_agent::AgentRuntime;
use anycode_core::TaskResult;
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

const DISCORD_API: &str = "https://discord.com/api/v10";
const DISCORD_REPLY_CHUNK: usize = 1800;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct DiscordCredentials {
    bot_token: String,
    channel_id: String,
}

pub(crate) struct DiscordRunArgs {
    pub bot_token: Option<String>,
    pub channel_id: Option<String>,
    pub agent: String,
    pub directory: Option<PathBuf>,
}

fn resolve_token(cli_token: Option<String>) -> Result<String> {
    if let Some(v) = cli_token {
        let t = v.trim().to_string();
        if !t.is_empty() {
            return Ok(t);
        }
    }
    if let Ok(v) = std::env::var("DISCORD_BOT_TOKEN") {
        let t = v.trim().to_string();
        if !t.is_empty() {
            return Ok(t);
        }
    }
    if let Some(saved) = load_saved_credentials() {
        let t = saved.bot_token.trim().to_string();
        if !t.is_empty() {
            return Ok(t);
        }
    }
    anyhow::bail!("missing Discord bot token; provide --bot-token or DISCORD_BOT_TOKEN");
}

fn resolve_channel_id(cli_id: Option<String>) -> Result<String> {
    if let Some(v) = cli_id {
        let t = v.trim().to_string();
        if !t.is_empty() {
            return Ok(t);
        }
    }
    if let Ok(v) = std::env::var("DISCORD_CHANNEL_ID") {
        let t = v.trim().to_string();
        if !t.is_empty() {
            return Ok(t);
        }
    }
    if let Some(saved) = load_saved_credentials() {
        let t = saved.channel_id.trim().to_string();
        if !t.is_empty() {
            return Ok(t);
        }
    }
    anyhow::bail!("missing Discord channel id; provide --channel-id or DISCORD_CHANNEL_ID");
}

fn discord_credentials_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no HOME directory found"))?;
    let p = home.join(".anycode/channels");
    std::fs::create_dir_all(&p)?;
    Ok(p.join("discord.json"))
}

fn load_saved_credentials() -> Option<DiscordCredentials> {
    let path = discord_credentials_path().ok()?;
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<DiscordCredentials>(&content).ok()
}

fn save_credentials(cred: &DiscordCredentials) -> Result<()> {
    let path = discord_credentials_path()?;
    std::fs::write(path, serde_json::to_string_pretty(cred)?)?;
    Ok(())
}

pub(crate) fn persist_credentials(token: String, channel_id: String) -> Result<()> {
    let bot_token = token.trim().to_string();
    let channel_id = channel_id.trim().to_string();
    if bot_token.is_empty() || channel_id.is_empty() {
        anyhow::bail!("token and channel_id must not be empty");
    }
    save_credentials(&DiscordCredentials {
        bot_token,
        channel_id,
    })
}

fn split_for_discord(s: &str) -> Vec<String> {
    if s.chars().count() <= DISCORD_REPLY_CHUNK {
        return vec![s.to_string()];
    }
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in s.chars() {
        cur.push(ch);
        if cur.chars().count() >= DISCORD_REPLY_CHUNK {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

async fn execute_prompt(
    runtime: &Arc<AgentRuntime>,
    agent: &str,
    working_directory: &str,
    channel_id: &str,
    user_id: &str,
    prompt: String,
) -> String {
    let task = build_channel_task(ChannelTaskInput {
        agent_type: agent.to_string(),
        prompt,
        working_directory: working_directory.to_string(),
        channel_id: channel_id.to_string(),
        user_id: user_id.to_string(),
        channel_name: "discord",
    });
    match runtime.execute_task(task).await {
        Ok(TaskResult::Success { output, .. }) => output,
        Ok(TaskResult::Failure { error, .. }) => format!("Task failed: {error}"),
        Ok(TaskResult::Partial { success, remaining }) => format!("{success}\n{remaining}"),
        Err(e) => format!("Runtime error: {e}"),
    }
}

pub(crate) async fn run_discord_polling(mut config: Config, args: DiscordRunArgs) -> Result<()> {
    apply_wechat_bridge_no_tool_approval(&mut config);
    let token = resolve_token(args.bot_token)?;
    let channel_id = resolve_channel_id(args.channel_id)?;
    let workdir = std::fs::canonicalize(
        args.directory
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))),
    )
    .unwrap_or_else(|_| PathBuf::from("."));
    let working_directory = workdir.to_string_lossy().to_string();

    let runtime = initialize_runtime(&config, None)
        .await
        .context("initialize runtime for discord")?;
    let client = Client::new();
    let mut last_seen: Option<String> = None;
    println!(
        "discord bridge started (polling channel {}). Press Ctrl+C to stop.",
        channel_id
    );

    loop {
        let url = format!("{DISCORD_API}/channels/{channel_id}/messages");
        let mut req = client
            .get(&url)
            .header("Authorization", format!("Bot {token}"))
            .query(&[("limit", "20")]);
        if let Some(after) = &last_seen {
            req = req.query(&[("after", after.as_str())]);
        }
        let resp = req.send().await.context("discord poll request")?;
        if resp.status().as_u16() == 429 {
            let retry_secs = resp
                .json::<serde_json::Value>()
                .await
                .ok()
                .and_then(|v| v.get("retry_after").and_then(|x| x.as_f64()))
                .unwrap_or(2.0);
            tokio::time::sleep(std::time::Duration::from_millis(
                (retry_secs * 1000.0) as u64,
            ))
            .await;
            continue;
        }
        if !resp.status().is_success() {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            continue;
        }
        let mut msgs = resp
            .json::<Vec<serde_json::Value>>()
            .await
            .context("discord poll decode")?;
        msgs.reverse();
        for m in msgs {
            let id = m
                .get("id")
                .and_then(|x| x.as_str())
                .unwrap_or_default()
                .to_string();
            if id.is_empty() {
                continue;
            }
            last_seen = Some(id);
            if m.get("author")
                .and_then(|a| a.get("bot"))
                .and_then(|x| x.as_bool())
                == Some(true)
            {
                continue;
            }
            let content = m
                .get("content")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if content.is_empty() {
                continue;
            }
            let user_id = m
                .get("author")
                .and_then(|a| a.get("id"))
                .and_then(|x| x.as_str())
                .unwrap_or("unknown")
                .to_string();
            let out = execute_prompt(
                &runtime,
                &args.agent,
                &working_directory,
                &channel_id,
                &user_id,
                content,
            )
            .await;
            let send_url = format!("{DISCORD_API}/channels/{channel_id}/messages");
            for chunk in split_for_discord(&out) {
                let payload = json!({ "content": chunk });
                let _ = client
                    .post(&send_url)
                    .header("Authorization", format!("Bot {token}"))
                    .json(&payload)
                    .send()
                    .await;
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
    }
}

pub(crate) async fn run_discord_setup() -> Result<()> {
    use dialoguer::{theme::ColorfulTheme, Input};

    let bot_token: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Discord Bot Token")
        .interact_text()?;
    let channel_id: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Discord Channel ID")
        .interact_text()?;

    let token = bot_token.trim().to_string();
    let cid = channel_id.trim().to_string();
    if token.is_empty() {
        anyhow::bail!("Discord Bot Token 不能为空");
    }
    if cid.is_empty() {
        anyhow::bail!("Discord Channel ID 不能为空");
    }
    save_credentials(&DiscordCredentials {
        bot_token: token,
        channel_id: cid,
    })?;
    println!("Discord channel 已配置完成。");
    println!("使用 `anycode channel discord` 启动轮询桥。");
    Ok(())
}
