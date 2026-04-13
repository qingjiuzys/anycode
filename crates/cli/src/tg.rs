use crate::app_config::{apply_wechat_bridge_no_tool_approval, Config};
use crate::bootstrap::initialize_runtime;
use crate::channel_task::{build_channel_task, im_task_failure_detail_excerpt, ChannelTaskInput};
use anycode_agent::AgentRuntime;
use anycode_core::{SecretRef, TaskResult};
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

const TELEGRAM_BASE: &str = "https://api.telegram.org";
const TELEGRAM_REPLY_CHUNK: usize = 3500;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct TelegramState {
    offset: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct TelegramCredentials {
    bot_token: String,
    chat_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct TgGetUpdates {
    ok: bool,
    result: Vec<TgUpdate>,
}

#[derive(Debug, Clone, Deserialize)]
struct TgUpdate {
    update_id: i64,
    message: Option<TgMessage>,
}

#[derive(Debug, Clone, Deserialize)]
struct TgMessage {
    message_id: i64,
    text: Option<String>,
    chat: TgChat,
    from: Option<TgUser>,
}

#[derive(Debug, Clone, Deserialize)]
struct TgChat {
    id: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct TgUser {
    id: i64,
}

pub(crate) struct TelegramRunArgs {
    pub bot_token: Option<String>,
    pub chat_id: Option<String>,
    pub agent: String,
    pub directory: Option<PathBuf>,
}

fn resolve_bot_token(cli_token: Option<String>) -> Result<String> {
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
    if let Ok(v) = std::env::var("TELEGRAM_BOT_TOKEN") {
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

    anyhow::bail!("missing Telegram bot token; provide --bot-token or TELEGRAM_BOT_TOKEN");
}

/// 解析SecretRef为实际值
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
                .or_else(|_| std::env::var(format!("TELEGRAM_{}", key.to_uppercase())))
                .map_err(|_| anyhow::anyhow!("提供商凭证未找到: 尝试环境变量 '{}'", env_key))
        }
    }
}

fn split_for_telegram(s: &str) -> Vec<String> {
    if s.chars().count() <= TELEGRAM_REPLY_CHUNK {
        return vec![s.to_string()];
    }
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in s.chars() {
        cur.push(ch);
        if cur.chars().count() >= TELEGRAM_REPLY_CHUNK {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn tg_data_root() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no HOME directory found"))?;
    let p = home.join(".anycode/telegram");
    std::fs::create_dir_all(&p)?;
    Ok(p)
}

fn telegram_credentials_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no HOME directory found"))?;
    let p = home.join(".anycode/channels");
    std::fs::create_dir_all(&p)?;
    Ok(p.join("telegram.json"))
}

fn load_saved_credentials() -> Option<TelegramCredentials> {
    let path = telegram_credentials_path().ok()?;
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<TelegramCredentials>(&content).ok()
}

fn save_credentials(cred: &TelegramCredentials) -> Result<()> {
    let path = telegram_credentials_path()?;
    std::fs::write(path, serde_json::to_string_pretty(cred)?)?;
    Ok(())
}

/// `anycode channel telegram-set-token`：写入凭据，供后续无 `--bot-token` 启动。
pub(crate) fn persist_credentials(token: String, chat_id: Option<String>) -> Result<()> {
    let bot_token = token.trim().to_string();
    if bot_token.is_empty() {
        anyhow::bail!("token must not be empty");
    }
    save_credentials(&TelegramCredentials {
        bot_token,
        chat_id: chat_id
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
    })
}

fn load_state(path: &PathBuf) -> TelegramState {
    match std::fs::read_to_string(path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => TelegramState::default(),
    }
}

fn save_state(path: &PathBuf, st: &TelegramState) {
    if let Ok(payload) = serde_json::to_string_pretty(st) {
        let _ = std::fs::write(path, payload);
    }
}

async fn execute_prompt(
    runtime: &Arc<AgentRuntime>,
    agent: &str,
    working_directory: &str,
    chat_id: i64,
    user_id: i64,
    prompt: String,
) -> String {
    let task = build_channel_task(ChannelTaskInput {
        agent_type: agent.to_string(),
        prompt,
        working_directory: working_directory.to_string(),
        channel_id: chat_id.to_string(),
        user_id: user_id.to_string(),
        channel_name: "telegram",
    });
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

pub(crate) async fn run_telegram_polling(mut config: Config, args: TelegramRunArgs) -> Result<()> {
    apply_wechat_bridge_no_tool_approval(&mut config);
    let saved = load_saved_credentials();
    let bot_token = resolve_bot_token(args.bot_token)?;
    let effective_chat = args
        .chat_id
        .or_else(|| saved.and_then(|c| c.chat_id))
        .and_then(|s| if s.trim().is_empty() { None } else { Some(s) });
    let allowed_chat = effective_chat.and_then(|s| s.trim().parse::<i64>().ok());
    let workdir = std::fs::canonicalize(
        args.directory
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))),
    )
    .unwrap_or_else(|_| PathBuf::from("."));
    let working_directory = workdir.to_string_lossy().to_string();

    let runtime = initialize_runtime(&config, None, None)
        .await
        .context("initialize runtime for telegram")?;
    let client = Client::new();
    let root = tg_data_root()?;
    let state_path = root.join("state.json");
    let mut state = load_state(&state_path);

    println!("telegram bridge started (polling). Press Ctrl+C to stop.");
    loop {
        let url = format!("{TELEGRAM_BASE}/bot{bot_token}/getUpdates");
        let resp = client
            .get(&url)
            .query(&[
                ("timeout", "25"),
                ("offset", &state.offset.to_string()),
                ("allowed_updates", "[\"message\"]"),
            ])
            .send()
            .await
            .context("telegram getUpdates request")?;
        let body: TgGetUpdates = resp.json().await.context("telegram getUpdates decode")?;
        if !body.ok {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            continue;
        }

        for upd in body.result {
            state.offset = upd.update_id + 1;
            let Some(msg) = upd.message else {
                continue;
            };
            let chat_id = msg.chat.id;
            if let Some(only) = allowed_chat {
                if chat_id != only {
                    continue;
                }
            }
            let Some(text) = msg.text else {
                continue;
            };
            let trimmed = text.trim();
            if trimmed.is_empty() {
                continue;
            }
            let user_id = msg.from.as_ref().map(|u| u.id).unwrap_or(chat_id);
            let out = execute_prompt(
                &runtime,
                &args.agent,
                &working_directory,
                chat_id,
                user_id,
                trimmed.to_string(),
            )
            .await;
            runtime.sync_memory_durability();
            let reply_url = format!("{TELEGRAM_BASE}/bot{bot_token}/sendMessage");
            for chunk in split_for_telegram(&out) {
                let payload = json!({
                    "chat_id": chat_id,
                    "text": chunk,
                    "reply_to_message_id": msg.message_id
                });
                let _ = client.post(&reply_url).json(&payload).send().await;
            }
        }
        save_state(&state_path, &state);
    }
}

pub(crate) async fn run_telegram_setup() -> Result<()> {
    use dialoguer::{theme::ColorfulTheme, Input};

    let bot_token: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Telegram Bot Token")
        .interact_text()?;
    let chat_id: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Telegram Chat ID（可留空）")
        .allow_empty(true)
        .interact_text()?;

    let token = bot_token.trim().to_string();
    if token.is_empty() {
        anyhow::bail!("Telegram Bot Token 不能为空");
    }
    let normalized_chat = if chat_id.trim().is_empty() {
        None
    } else {
        Some(chat_id.trim().to_string())
    };

    save_credentials(&TelegramCredentials {
        bot_token: token,
        chat_id: normalized_chat,
    })?;
    println!("Telegram channel 已配置完成。");
    println!("使用 `anycode channel telegram` 启动轮询桥。");
    Ok(())
}
