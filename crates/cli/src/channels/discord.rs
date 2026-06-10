use crate::app_config::{apply_wechat_bridge_no_tool_approval, Config};
use crate::bootstrap::initialize_runtime;
use crate::channel_task::{build_channel_task, im_task_failure_detail_excerpt, ChannelTaskInput};
use anycode_agent::AgentRuntime;
use anycode_core::{SecretRef, TaskResult, VisionImage};
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
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
    // 1. CLI提供的token
    if let Some(v) = cli_token {
        let t = v.trim();
        if !t.is_empty() {
            // 支持SecretRef语法
            let secret_ref = SecretRef::from_string(t);
            if let Ok(resolved) = resolve_secret_ref(&secret_ref) {
                return Ok(resolved);
            }
            return Ok(t.to_string());
        }
    }

    // 2. 环境变量
    if let Ok(v) = std::env::var("DISCORD_BOT_TOKEN") {
        let t = v.trim();
        if !t.is_empty() {
            return Ok(t.to_string());
        }
    }

    // 3. 保存的凭据
    if let Some(saved) = load_saved_credentials() {
        let t = saved.bot_token.trim();
        if !t.is_empty() {
            // 支持SecretRef语法
            let secret_ref = SecretRef::from_string(t);
            if let Ok(resolved) = resolve_secret_ref(&secret_ref) {
                return Ok(resolved);
            }
            return Ok(t.to_string());
        }
    }

    anyhow::bail!("missing Discord bot token; provide --bot-token or DISCORD_BOT_TOKEN");
}

/// 解析SecretRef为实际值（与Telegram适配器共用）
fn resolve_secret_ref(secret_ref: &SecretRef) -> Result<String> {
    match secret_ref {
        SecretRef::Direct(value) => Ok(value.clone()),
        SecretRef::EnvVar(var_name) => {
            std::env::var(var_name).map_err(|_| anyhow::anyhow!("环境变量 '{}' 未设置", var_name))
        }
        SecretRef::File(path) => {
            let full_path = if path.is_absolute() {
                path.clone()
            } else {
                dirs::home_dir()
                    .ok_or_else(|| anyhow::anyhow!("无法找到HOME目录"))?
                    .join(path)
            };
            std::fs::read_to_string(&full_path)
                .map(|s| s.trim().to_string())
                .map_err(|e| anyhow::anyhow!("无法读取密钥文件 '{}': {}", full_path.display(), e))
        }
        SecretRef::ProviderCredential { provider, key } => {
            let env_key = format!("{}_{}", provider.to_uppercase(), key.to_uppercase());
            std::env::var(&env_key)
                .or_else(|_| std::env::var(format!("DISCORD_{}", key.to_uppercase())))
                .map_err(|_| anyhow::anyhow!("提供商凭证未找到: 尝试环境变量 '{}'", env_key))
        }
    }
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

async fn discord_download_url(client: &Client, url: &str) -> Option<Vec<u8>> {
    client
        .get(url)
        .send()
        .await
        .ok()?
        .bytes()
        .await
        .ok()
        .map(|b| b.to_vec())
}

async fn discord_transcribe_audio(
    client: &Client,
    url: &str,
    mime: &str,
    filename: Option<&str>,
) -> Option<String> {
    use anycode_llm::{
        config_file::read_config_value,
        media::{MediaClientRegistry, SttClient},
    };
    let bytes = discord_download_url(client, url).await?;
    let (_, cfg) = read_config_value(None).ok()?;
    let reg = MediaClientRegistry::from_config(&cfg);
    let prof = reg.stt.as_ref()?;
    let stt = SttClient::new(prof.profile.clone());
    let name = filename.filter(|s| !s.is_empty()).unwrap_or_else(|| {
        if mime.contains("mpeg") || mime.contains("mp3") {
            "voice.mp3"
        } else if mime.contains("wav") {
            "voice.wav"
        } else {
            "voice.ogg"
        }
    });
    stt.transcribe(&bytes, name)
        .await
        .ok()
        .map(|r| r.text)
        .filter(|t| !t.trim().is_empty())
}

fn attachment_is_probably_voice(att: &serde_json::Value, mime: &str) -> bool {
    if mime.starts_with("audio/") {
        return true;
    }
    if mime != "application/octet-stream" {
        return false;
    }
    att.get("filename")
        .and_then(|x| x.as_str())
        .is_some_and(|n| {
            let lower = n.to_ascii_lowercase();
            lower.contains("voice")
                || lower.ends_with(".ogg")
                || lower.ends_with(".mp3")
                || lower.ends_with(".wav")
                || lower.ends_with(".m4a")
        })
}

async fn resolve_discord_prompt(
    client: &Client,
    message: &serde_json::Value,
) -> Option<(String, Vec<VisionImage>)> {
    let mut prompt = message
        .get("content")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let mut vision_images = Vec::new();
    let mut saw_voice = false;
    if let Some(atts) = message.get("attachments").and_then(|v| v.as_array()) {
        for att in atts {
            let mime = att
                .get("content_type")
                .and_then(|x| x.as_str())
                .unwrap_or("application/octet-stream");
            let Some(url) = att.get("url").and_then(|x| x.as_str()) else {
                continue;
            };
            if mime.starts_with("image/") {
                if let Some(bytes) = discord_download_url(client, url).await {
                    vision_images.push(VisionImage::new(mime.to_string(), STANDARD.encode(bytes)));
                }
                continue;
            }
            if prompt.is_empty() && attachment_is_probably_voice(att, mime) {
                saw_voice = true;
                let filename = att.get("filename").and_then(|x| x.as_str());
                if let Some(text) = discord_transcribe_audio(client, url, mime, filename).await {
                    prompt = text;
                }
            }
        }
    }
    if prompt.is_empty() {
        if saw_voice {
            prompt = "(voice message — STT unavailable)".to_string();
        } else if !vision_images.is_empty() {
            prompt = "Please describe or analyze this image.".to_string();
        }
    }
    if prompt.is_empty() && vision_images.is_empty() {
        return None;
    }
    Some((prompt, vision_images))
}

async fn execute_prompt(
    runtime: &Arc<AgentRuntime>,
    config: &Config,
    agent: &str,
    working_directory: &str,
    channel_id: &str,
    user_id: &str,
    prompt: String,
    user_vision_images: Vec<VisionImage>,
) -> String {
    let task = build_channel_task(
        ChannelTaskInput {
            agent_type: agent.to_string(),
            prompt,
            working_directory: working_directory.to_string(),
            channel_id: channel_id.to_string(),
            user_id: user_id.to_string(),
            channel_name: "discord",
            user_vision_images,
        },
        config,
    );
    match runtime.execute_task(task).await {
        Ok(TaskResult::Success { output, .. }) => output,
        Ok(TaskResult::Failure { error, details }) => {
            let mut s = format!("Task failed: {error}");
            if let Some(ex) = im_task_failure_detail_excerpt(details.as_deref(), 1500) {
                s.push('\n');
                s.push_str(&ex);
            }
            s
        }
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

    let client = Client::new();
    let qbroker = Arc::new(super::discord_ask::DiscordQuestionBroker::new());
    let ask_host = super::discord_ask::DiscordAskUserQuestionHost::new(
        Arc::clone(&qbroker),
        client.clone(),
        token.clone(),
        channel_id.clone(),
    )
    .into_arc();
    let project_enabled =
        crate::workbench::project_skills::load_project_enabled_skills(&workdir).await;
    let runtime = initialize_runtime(
        &config,
        None,
        Some(ask_host),
        crate::bootstrap::MemoryAttachMode::Exclusive,
        project_enabled,
    )
    .await
    .context("initialize runtime for discord")?;
    let runtime_for_sched = Arc::new(tokio::sync::RwLock::new(Arc::clone(&runtime)));

    let cwd_sched = workdir.clone();
    let sched_cfg = config.clone();
    let sched_runtime = Arc::clone(&runtime_for_sched);
    crate::scheduler::spawn_embedded_scheduler(
        sched_cfg,
        cwd_sched,
        sched_runtime,
        crate::scheduler::CronDelivery::None,
        30,
    );
    let qbroker_for_poll = Arc::clone(&qbroker);
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
            if !content.is_empty()
                && qbroker_for_poll
                    .try_resolve_numeric(&channel_id, &content)
                    .await
            {
                continue;
            }
            let Some((prompt_text, vision_images)) = resolve_discord_prompt(&client, &m).await
            else {
                continue;
            };
            let user_id = m
                .get("author")
                .and_then(|a| a.get("id"))
                .and_then(|x| x.as_str())
                .unwrap_or("unknown")
                .to_string();
            let out = super::discord_ask::with_discord_channel_scope(
                channel_id.clone(),
                execute_prompt(
                    &runtime,
                    &config,
                    &args.agent,
                    &working_directory,
                    &channel_id,
                    &user_id,
                    prompt_text,
                    vision_images,
                ),
            )
            .await;
            runtime.sync_memory_durability();
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
