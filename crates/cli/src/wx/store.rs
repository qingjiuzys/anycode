//! 账号、会话、config.env（与 Node 版目录布局兼容）。

use crate::i18n::{tr, tr_args};
use anyhow::{Context, Result};
use fluent_bundle::FluentArgs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountData {
    pub bot_token: String,
    pub account_id: String,
    pub base_url: String,
    pub user_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    #[default]
    Idle,
    Processing,
    WaitingPermission,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WcSession {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sdk_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_sdk_session_id: Option<String>,
    pub working_directory: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_mode: Option<String>,
    #[serde(default)]
    pub state: SessionState,
    #[serde(default)]
    pub chat_history: Vec<ChatMessage>,
    #[serde(default = "default_max_hist")]
    pub max_history_length: usize,
}

fn default_max_hist() -> usize {
    100
}

impl WcSession {
    pub fn with_default_cwd() -> Self {
        Self {
            working_directory: crate::workspace::canonical_root_string(),
            max_history_length: 100,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct WccConfig {
    pub working_directory: String,
    pub model: Option<String>,
    pub permission_mode: Option<String>,
    pub runtime_mode: Option<String>,
    pub system_prompt: Option<String>,
}

/// 微信数据根目录：默认 `~/.anycode/wechat`（与 anyCode 其它落盘一致）。
/// 环境变量 `WCC_DATA_DIR` 非空时覆盖（与旧 wechat-claude-code / Node 桥目录兼容）。
pub fn wcc_data_dir(explicit: Option<PathBuf>) -> PathBuf {
    if let Some(p) = explicit {
        return p;
    }
    if let Ok(s) = std::env::var("WCC_DATA_DIR") {
        let t = s.trim();
        if !t.is_empty() {
            return PathBuf::from(t);
        }
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".anycode")
        .join("wechat")
}

pub fn validate_account_id(id: &str) -> Result<()> {
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_.@=-".contains(c))
    {
        let mut a = FluentArgs::new();
        a.set("id", format!("{id:?}"));
        anyhow::bail!("{}", tr_args("wx-err-bad-account-id", &a));
    }
    Ok(())
}

pub fn load_wcc_config(data_root: &Path) -> WccConfig {
    let default_wd = crate::workspace::canonical_root_string();
    let path = data_root.join("config.env");
    let Ok(raw) = fs::read_to_string(&path) else {
        return WccConfig {
            working_directory: default_wd,
            ..Default::default()
        };
    };
    let mut c = WccConfig {
        working_directory: default_wd.clone(),
        ..Default::default()
    };
    let mut saw_working_directory = false;
    for line in raw.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let Some((k, v)) = t.split_once('=') else {
            continue;
        };
        match k.trim() {
            "workingDirectory" => {
                saw_working_directory = true;
                c.working_directory = v.trim().to_string();
            }
            "model" => c.model = Some(v.trim().to_string()),
            "permissionMode" => c.permission_mode = Some(v.trim().to_string()),
            "runtimeMode" => c.runtime_mode = Some(v.trim().to_string()),
            "systemPrompt" => c.system_prompt = Some(v.trim().to_string()),
            _ => {}
        }
    }
    if saw_working_directory && c.working_directory.trim().is_empty() {
        c.working_directory = default_wd;
    }
    c
}

pub fn load_latest_account(data_root: &Path) -> Result<AccountData> {
    let dir = data_root.join("accounts");
    let rd = fs::read_dir(&dir).with_context(|| {
        let mut a = FluentArgs::new();
        a.set("path", dir.display().to_string());
        tr_args("wx-err-read-account-dir", &a)
    })?;
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().map_or(true, |x| x != "json") {
            continue;
        }
        let mt = e.metadata().ok().and_then(|m| m.modified().ok());
        let Some(mt) = mt else { continue };
        if best.as_ref().map_or(true, |(t, _)| mt > *t) {
            best = Some((mt, p));
        }
    }
    let path = best
        .map(|(_, p)| p)
        .with_context(|| {
            let mut a = FluentArgs::new();
            a.set("path", dir.display().to_string());
            tr_args("wx-err-no-account-json", &a)
        })?;
    let raw = fs::read_to_string(&path).with_context(|| {
        let mut a = FluentArgs::new();
        a.set("path", path.display().to_string());
        tr_args("wx-err-read-file", &a)
    })?;
    let acc: AccountData = serde_json::from_str(&raw).with_context(|| {
        let mut a = FluentArgs::new();
        a.set("path", path.display().to_string());
        tr_args("wx-err-parse-json", &a)
    })?;
    validate_account_id(&acc.account_id)?;
    Ok(acc)
}

fn session_path(data_root: &Path, account_id: &str) -> Result<PathBuf> {
    validate_account_id(account_id)?;
    Ok(data_root
        .join("sessions")
        .join(format!("{}.json", account_id)))
}

pub fn load_session(data_root: &Path, account_id: &str) -> Result<WcSession> {
    let p = session_path(data_root, account_id)?;
    if !p.is_file() {
        return Ok(WcSession::with_default_cwd());
    }
    let raw = fs::read_to_string(&p)?;
    let mut s: WcSession =
        serde_json::from_str(&raw).unwrap_or_else(|_| WcSession::with_default_cwd());
    if s.working_directory.is_empty() {
        s.working_directory = crate::workspace::canonical_root_string();
    }
    if s.max_history_length == 0 {
        s.max_history_length = 100;
    }
    Ok(s)
}

pub fn save_session(data_root: &Path, account_id: &str, session: &WcSession) -> Result<()> {
    let dir = data_root.join("sessions");
    fs::create_dir_all(&dir)?;
    let p = session_path(data_root, account_id)?;
    let mut s = session.clone();
    let max = s.max_history_length.max(1);
    if s.chat_history.len() > max {
        s.chat_history = s.chat_history[s.chat_history.len() - max..].to_vec();
    }
    let raw = serde_json::to_string_pretty(&s)? + "\n";
    fs::write(&p, raw)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&p)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&p, perms)?;
    }
    Ok(())
}

pub fn chat_history_text(session: &WcSession, limit: Option<usize>) -> String {
    let h = &session.chat_history;
    let slice: &[ChatMessage] = match limit {
        Some(n) if n > 0 => {
            let start = h.len().saturating_sub(n);
            &h[start..]
        }
        _ => h,
    };
    if slice.is_empty() {
        return tr("wx-history-empty");
    }
    let mut lines = Vec::new();
    for m in slice {
        let role = if m.role == "user" {
            tr("wx-role-user")
        } else {
            tr("wx-role-assistant")
        };
        lines.push(format!("[{}] {}:", m.timestamp, role));
        lines.push(m.content.clone());
        lines.push(String::new());
    }
    lines.join("\n")
}

pub fn add_chat_message(session: &mut WcSession, role: &str, content: &str) {
    session.chat_history.push(ChatMessage {
        role: role.into(),
        content: content.into(),
        timestamp: chrono::Utc::now().timestamp_millis(),
    });
    let max = session.max_history_length.max(1);
    if session.chat_history.len() > max {
        session.chat_history = session.chat_history[session.chat_history.len() - max..].to_vec();
    }
}
