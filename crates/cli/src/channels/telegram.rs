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
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex as AsyncMutex, RwLock};

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
    #[serde(default)]
    callback_query: Option<TgCallbackQuery>,
}

#[derive(Debug, Clone, Deserialize)]
struct TgCallbackQuery {
    id: String,
    #[serde(default)]
    data: Option<String>,
    #[serde(default)]
    message: Option<TgCallbackMessageShell>,
}

/// Minimal shape: only need `chat.id` from the message attached to a callback.
#[derive(Debug, Clone, Deserialize)]
struct TgCallbackMessageShell {
    chat: TgChat,
}

#[derive(Debug, Clone, Deserialize)]
struct TgMessage {
    message_id: i64,
    text: Option<String>,
    #[serde(default)]
    caption: Option<String>,
    #[serde(default)]
    photo: Option<Vec<TgPhotoSize>>,
    #[serde(default)]
    voice: Option<TgVoice>,
    chat: TgChat,
    from: Option<TgUser>,
}

#[derive(Debug, Clone, Deserialize)]
struct TgPhotoSize {
    file_id: String,
    width: u32,
    height: u32,
}

#[derive(Debug, Clone, Deserialize)]
struct TgVoice {
    file_id: String,
    #[serde(default)]
    mime_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct TgGetFileResponse {
    ok: bool,
    result: Option<TgFileMeta>,
}

#[derive(Debug, Clone, Deserialize)]
struct TgFileMeta {
    file_path: String,
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

async fn tg_download_file_bytes(
    client: &Client,
    bot_token: &str,
    file_id: &str,
) -> Option<Vec<u8>> {
    let meta_url = format!("{TELEGRAM_BASE}/bot{bot_token}/getFile");
    let meta: TgGetFileResponse = client
        .get(&meta_url)
        .query(&[("file_id", file_id)])
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    if !meta.ok {
        return None;
    }
    let path = meta.result?.file_path;
    let file_url = format!("{TELEGRAM_BASE}/file/bot{bot_token}/{path}");
    client
        .get(&file_url)
        .send()
        .await
        .ok()?
        .bytes()
        .await
        .ok()
        .map(|b| b.to_vec())
}

async fn tg_download_photo_b64(client: &Client, bot_token: &str, file_id: &str) -> Option<String> {
    let bytes = tg_download_file_bytes(client, bot_token, file_id).await?;
    Some(STANDARD.encode(bytes))
}

async fn tg_transcribe_voice(client: &Client, bot_token: &str, voice: &TgVoice) -> Option<String> {
    use anycode_llm::{
        config_file::read_config_value,
        media::{MediaClientRegistry, SttClient},
    };
    let bytes = tg_download_file_bytes(client, bot_token, &voice.file_id).await?;
    let (_, cfg) = read_config_value(None).ok()?;
    let reg = MediaClientRegistry::from_config(&cfg);
    let prof = reg.stt.as_ref()?;
    let stt = SttClient::new(prof.profile.clone());
    let filename = voice
        .mime_type
        .as_deref()
        .and_then(|m| {
            if m.contains("mpeg") || m.contains("mp3") {
                Some("voice.mp3")
            } else {
                Some("voice.ogg")
            }
        })
        .unwrap_or("voice.ogg");
    stt.transcribe(&bytes, filename)
        .await
        .ok()
        .map(|r| r.text)
        .filter(|t| !t.trim().is_empty())
}

async fn resolve_telegram_prompt(
    client: &Client,
    bot_token: &str,
    msg: &TgMessage,
) -> Option<(String, Vec<VisionImage>)> {
    let mut prompt = msg
        .text
        .clone()
        .or_else(|| msg.caption.clone())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_default();
    let mut vision_images = Vec::new();
    if let Some(photos) = &msg.photo {
        if let Some(largest) = photos
            .iter()
            .max_by_key(|p| p.width.saturating_mul(p.height))
        {
            if let Some(b64) = tg_download_photo_b64(client, bot_token, &largest.file_id).await {
                vision_images.push(VisionImage::new("image/jpeg", b64));
            }
        }
    }
    if prompt.is_empty() {
        if let Some(voice) = &msg.voice {
            prompt = tg_transcribe_voice(client, bot_token, voice)
                .await
                .unwrap_or_else(|| "(voice message — STT unavailable)".to_string());
        } else if !vision_images.is_empty() {
            prompt = "Please describe or analyze this image.".to_string();
        }
    }
    if prompt.is_empty() && vision_images.is_empty() {
        return None;
    }
    Some((prompt, vision_images))
}

async fn tg_answer_callback_query(client: &Client, bot_token: &str, query_id: &str) {
    let url = format!("{TELEGRAM_BASE}/bot{bot_token}/answerCallbackQuery");
    let payload = json!({ "callback_query_id": query_id });
    let _ = client.post(&url).json(&payload).send().await;
}

async fn execute_prompt(
    runtime: &Arc<AgentRuntime>,
    config: &Config,
    agent: &str,
    working_directory: &str,
    chat_id: i64,
    user_id: i64,
    prompt: String,
    user_vision_images: Vec<VisionImage>,
) -> String {
    let task = build_channel_task(
        ChannelTaskInput {
            agent_type: agent.to_string(),
            prompt,
            working_directory: working_directory.to_string(),
            channel_id: chat_id.to_string(),
            user_id: user_id.to_string(),
            channel_name: "telegram",
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

    let client = Client::new();
    let qbroker = Arc::new(super::tg_ask::TelegramQuestionBroker::new());
    let ask_host = super::tg_ask::TelegramAskUserQuestionHost::new(
        Arc::clone(&qbroker),
        client.clone(),
        bot_token.clone(),
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
    .context("initialize runtime for telegram")?;
    let runtime_for_sched = Arc::new(RwLock::new(Arc::clone(&runtime)));

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
    let root = tg_data_root()?;
    let state_path = root.join("state.json");
    let mut state = load_state(&state_path);

    // Serialize per-chat agent runs; callbacks stay on the polling task.
    type ChatExecLocks = HashMap<i64, Arc<AsyncMutex<()>>>;
    let chat_exec_locks: Arc<AsyncMutex<ChatExecLocks>> = Arc::new(AsyncMutex::new(HashMap::new()));

    println!("telegram bridge started (polling). Press Ctrl+C to stop.");
    loop {
        let url = format!("{TELEGRAM_BASE}/bot{bot_token}/getUpdates");
        let resp = client
            .get(&url)
            .query(&[
                ("timeout", "25"),
                ("offset", &state.offset.to_string()),
                ("allowed_updates", "[\"message\",\"callback_query\"]"),
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
            if let Some(cb) = upd.callback_query {
                if let (Some(data), Some(msg_shell)) = (cb.data.as_deref(), cb.message.as_ref()) {
                    if let Some(idx) = super::tg_ask::parse_uq_callback_data(data) {
                        let chat_id = msg_shell.chat.id;
                        if let Some(only) = allowed_chat {
                            if chat_id != only {
                                continue;
                            }
                        }
                        let _ = qbroker.resolve_callback(chat_id, idx).await;
                        tg_answer_callback_query(&client, &bot_token, &cb.id).await;
                    }
                }
                continue;
            }
            let Some(msg) = upd.message else {
                continue;
            };
            let chat_id = msg.chat.id;
            if let Some(only) = allowed_chat {
                if chat_id != only {
                    continue;
                }
            }
            if let Some(text) = msg.text.as_deref() {
                let prompt_text = text.trim().to_string();
                if prompt_text.len() == 1 {
                    if let Some(d @ b'1'..=b'8') = prompt_text.as_bytes().first().copied() {
                        let idx = (d - b'1') as usize;
                        if qbroker.resolve_callback(chat_id, idx).await {
                            continue;
                        }
                    }
                }
            }
            let has_media =
                msg.photo.as_ref().is_some_and(|p| !p.is_empty()) || msg.voice.is_some();
            let has_text = msg
                .text
                .as_deref()
                .or(msg.caption.as_deref())
                .is_some_and(|s| !s.trim().is_empty());
            if !has_text && !has_media {
                continue;
            }
            let user_id = msg.from.as_ref().map(|u| u.id).unwrap_or(chat_id);

            let m = {
                let mut g = chat_exec_locks.lock().await;
                g.entry(chat_id)
                    .or_insert_with(|| Arc::new(AsyncMutex::new(())))
                    .clone()
            };
            let runtime = runtime.clone();
            let agent = args.agent.clone();
            let working_directory = working_directory.clone();
            let config_sp = config.clone();
            let bot_token_sp = bot_token.clone();
            let client_sp = client.clone();
            let message_id = msg.message_id;
            tokio::spawn(async move {
                let _guard = m.lock().await;
                let Some((prompt_text, vision_images)) =
                    resolve_telegram_prompt(&client_sp, &bot_token_sp, &msg).await
                else {
                    return;
                };
                let out = super::tg_ask::with_telegram_chat_scope(
                    chat_id,
                    execute_prompt(
                        &runtime,
                        &config_sp,
                        &agent,
                        &working_directory,
                        chat_id,
                        user_id,
                        prompt_text,
                        vision_images,
                    ),
                )
                .await;
                runtime.sync_memory_durability();
                let reply_url = format!("{TELEGRAM_BASE}/bot{bot_token_sp}/sendMessage");
                for chunk in split_for_telegram(&out) {
                    let payload = json!({
                        "chat_id": chat_id,
                        "text": chunk,
                        "reply_to_message_id": message_id
                    });
                    let _ = client_sp.post(&reply_url).json(&payload).send().await;
                }
            });
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
