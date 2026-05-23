//! `~/.anycode/config.json` user file (`AnyCodeConfig`) and persistence.

use super::schema::{
    ChannelsConfigFile, LspConfigFile, MemoryConfigFile, MemoryPipelineConfigFile, RoutingConfig,
    RuntimeSettingsFile, SecurityConfigFile, SessionConfigFile, SkillsConfigFile,
    StatusLineConfigFile,
};
use crate::i18n::{tr, tr_args};
use anycode_agent::ModelInstructionsConfig;
use fluent_bundle::FluentArgs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

fn default_model_instructions_enabled() -> bool {
    true
}

fn default_model_instructions_max_depth() -> usize {
    10
}

/// `config.json` 中的 `model_instructions` 段：AGENTS.md 等文件发现配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ModelInstructionsConfigFile {
    #[serde(default = "default_model_instructions_enabled")]
    pub(crate) enabled: bool,
    #[serde(default)]
    pub(crate) filename: Option<String>,
    #[serde(default = "default_model_instructions_max_depth")]
    pub(crate) max_depth: usize,
}

impl Default for ModelInstructionsConfigFile {
    fn default() -> Self {
        Self {
            enabled: default_model_instructions_enabled(),
            filename: None,
            max_depth: default_model_instructions_max_depth(),
        }
    }
}

impl From<ModelInstructionsConfigFile> for ModelInstructionsConfig {
    fn from(f: ModelInstructionsConfigFile) -> Self {
        Self {
            enabled: f.enabled,
            filename: f.filename,
            max_depth: Some(f.max_depth),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AnyCodeConfig {
    // V1 固定：z.ai（= BigModel）
    pub(crate) provider: String,
    // 套餐：coding（编码套餐） / general（通用）
    pub(crate) plan: String,
    pub(crate) api_key: String,
    /// 按厂商 id 存额外密钥（如 `anthropic`、`openrouter`），用于与全局不同厂商混跑 routing。
    #[serde(default)]
    pub(crate) provider_credentials: HashMap<String, String>,
    pub(crate) base_url: Option<String>,
    // V1 先固定为 glm-4（后续可扩展为编码套餐的多个模型）
    pub(crate) model: String,
    pub(crate) temperature: f32,
    pub(crate) max_tokens: u32,
    #[serde(default)]
    pub(crate) routing: RoutingConfig,
    #[serde(default)]
    pub(crate) runtime: RuntimeSettingsFile,
    #[serde(default)]
    pub(crate) security: SecurityConfigFile,
    /// 整段覆盖 system（非空则不再注入默认段、记忆、append）。支持 `@相对或绝对路径` 从文件读取。
    #[serde(default)]
    pub(crate) system_prompt_override: Option<String>,
    /// 接在合成 system 末尾。支持 `@path` 读文件（相对路径相对配置文件所在目录）。
    #[serde(default)]
    pub(crate) system_prompt_append: Option<String>,
    #[serde(default)]
    pub(crate) memory: MemoryConfigFile,
    /// z.ai OpenAI 兼容栈：首轮带 tools 时 `tool_choice: required`（环境变量 `ANYCODE_ZAI_TOOL_CHOICE_*` 仍可覆盖）。
    #[serde(default)]
    pub(crate) zai_tool_choice_first_turn: bool,
    #[serde(default)]
    pub(crate) skills: SkillsConfigFile,
    #[serde(default)]
    pub(crate) session: SessionConfigFile,
    #[serde(default)]
    pub(crate) model_instructions: ModelInstructionsConfigFile,
    /// 全屏 TUI 底部 status line（JSON key `statusLine`）。
    #[serde(default, rename = "statusLine")]
    pub(crate) status_line: StatusLineConfigFile,
    /// 流式终端与行式 REPL 共用此段（备用屏等）。`terminal.alternateScreen` 为 true 时 DEC 备用屏；显式 `ANYCODE_TERM_ALT_SCREEN` 可解析时覆盖（见 CHANGELOG）。
    #[serde(default, rename = "terminal")]
    pub(crate) terminal: TerminalConfigFile,
    /// 通道特定配置（wechat、telegram、discord等）
    #[serde(default)]
    pub(crate) channels: ChannelsConfigFile,
    #[serde(default)]
    pub(crate) lsp: LspConfigFile,
    /// 工具结果 / 回合结束外向通知（HTTP、shell），与 `memory.pipeline.hook_*` 独立。
    #[serde(default)]
    pub(crate) notifications: anycode_core::SessionNotificationSettings,
}

/// `config.json` 的 `terminal` 段。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct TerminalConfigFile {
    /// `true`：DEC 备用屏（独立全屏画布）；`false` 或未设置：由入口（`anycode tui` / REPL）与运行环境决定；显式 env 优先。
    #[serde(default, rename = "alternateScreen")]
    pub(crate) alternate_screen: Option<bool>,
}

pub(crate) fn default_base_url_for(plan: &str) -> &'static str {
    anycode_llm::zai_default_chat_url_for_plan(plan)
}

fn anycode_config_path() -> anyhow::Result<PathBuf> {
    let home = std::env::var("HOME")?;
    Ok(PathBuf::from(home).join(".anycode").join("config.json"))
}

pub(crate) fn resolve_memory_directory(path_opt: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("{}", tr("err-no-home-memory")))?;
    match path_opt {
        None => Ok(home.join(".anycode/memory")),
        Some(p) if p.is_absolute() => Ok(p),
        Some(p) => Ok(home.join(p)),
    }
}

pub(crate) fn normalize_memory_backend(raw: &str) -> anyhow::Result<String> {
    let b = raw.trim().to_lowercase();
    let b = if b.is_empty() { "file".to_string() } else { b };
    match b.as_str() {
        "noop" | "none" | "off" => Ok("noop".to_string()),
        "file" => Ok("file".to_string()),
        "hybrid" => Ok("hybrid".to_string()),
        "pipeline" | "layered" | "guigen" => Ok("pipeline".to_string()),
        _ => {
            let mut a = FluentArgs::new();
            a.set("b", raw.trim());
            anyhow::bail!("{}", tr_args("err-memory-backend", &a));
        }
    }
}

pub(crate) fn normalize_embedding_provider(raw: Option<&str>) -> anyhow::Result<String> {
    let s = raw.unwrap_or("http").trim().to_lowercase();
    match s.as_str() {
        "" | "http" | "openai" | "remote" => Ok("http".to_string()),
        "local" | "onnx" | "fastembed" => Ok("local".to_string()),
        other => anyhow::bail!(
            "invalid memory.pipeline.embedding_provider: {:?} (allowed: http, local, onnx, fastembed)",
            other
        ),
    }
}

pub(crate) fn resolve_embedding_local_cache_dir(
    p: Option<PathBuf>,
) -> anyhow::Result<Option<PathBuf>> {
    let Some(p) = p.filter(|x| !x.as_os_str().is_empty()) else {
        return Ok(None);
    };
    if p.is_absolute() {
        return Ok(Some(p));
    }
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("{}", tr("err-no-home-memory")))?;
    Ok(Some(home.join(p)))
}

pub(crate) fn merge_memory_pipeline_settings(
    f: &MemoryPipelineConfigFile,
) -> anycode_core::MemoryPipelineSettings {
    use anycode_core::MemoryPipelineSettings;
    let mut s = MemoryPipelineSettings::default();
    if let Some(v) = f.buffer_ttl_secs {
        s.buffer_ttl_secs = v;
    }
    if let Some(v) = f.max_buffer_fragments {
        s.max_buffer_fragments = v;
    }
    if let Some(v) = f.promote_touch_threshold {
        s.promote_touch_threshold = v;
    }
    if let Some(v) = f.reinforce_on_recall_match {
        s.reinforce_on_recall_match = v;
    }
    if let Some(v) = f.merge_legacy_file_recall {
        s.merge_legacy_file_recall = v;
    }
    if let Some(v) = f.buffer_wal_enabled {
        s.buffer_wal_enabled = v;
    }
    if let Some(v) = f.buffer_wal_fsync_every_n {
        s.buffer_wal_fsync_every_n = v.max(1);
    }
    if let Some(v) = f.hook_after_tool_result {
        s.hook_after_tool_result = v;
    }
    if let Some(v) = f.hook_after_agent_turn {
        s.hook_after_agent_turn = v;
    }
    if let Some(v) = f.hook_max_bytes {
        s.hook_max_bytes = v.max(256);
    }
    if let Some(ref v) = f.hook_tool_deny_prefixes {
        if !v.is_empty() {
            s.hook_tool_deny_prefixes = v.clone();
        }
    }
    if let Some(v) = f.embedding_enabled {
        s.embedding_enabled = v;
    }
    if f.embedding_model
        .as_ref()
        .map(|m| !m.trim().is_empty())
        .unwrap_or(false)
    {
        s.embedding_enabled = true;
    }
    s
}

/// `-c` 指定文件，否则 `~/.anycode/config.json`。
///
/// 供微信桥等长驻进程监视配置文件变更（mtime）时使用，规则与 `load_config` 一致。
pub(crate) fn resolve_config_path(config_file: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    match config_file {
        Some(p) => Ok(p),
        None => anycode_config_path(),
    }
}

fn load_anycode_config_from_path(path: &Path) -> anyhow::Result<Option<AnyCodeConfig>> {
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path)?;
    Ok(Some(serde_json::from_str(&content)?))
}

/// 显式 `-c path` 且文件不存在时返回 Err；默认路径不存在则 `Ok(None)`。
pub(crate) fn load_anycode_config_resolved(
    config_file: Option<PathBuf>,
) -> anyhow::Result<Option<AnyCodeConfig>> {
    let path = resolve_config_path(config_file.clone())?;
    match load_anycode_config_from_path(&path)? {
        Some(c) => Ok(Some(c)),
        None => {
            if config_file.is_some() {
                let mut a = FluentArgs::new();
                a.set("path", path.display().to_string());
                anyhow::bail!("{}", tr_args("err-config-not-found", &a));
            }
            Ok(None)
        }
    }
}

pub(crate) fn load_anycode_config() -> anyhow::Result<Option<AnyCodeConfig>> {
    load_anycode_config_resolved(None)
}

fn save_anycode_config_to(path: &Path, cfg: &AnyCodeConfig) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(cfg)?)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

pub(crate) fn save_anycode_config_resolved(
    config_file: Option<PathBuf>,
    cfg: &AnyCodeConfig,
) -> anyhow::Result<()> {
    let path = resolve_config_path(config_file)?;
    save_anycode_config_to(&path, cfg)
}

pub(crate) fn save_anycode_config(cfg: &AnyCodeConfig) -> anyhow::Result<()> {
    save_anycode_config_to(&anycode_config_path()?, cfg)
}

pub(crate) fn load_or_default_anycode_config(
    config_file: Option<PathBuf>,
) -> anyhow::Result<AnyCodeConfig> {
    Ok(
        load_anycode_config_resolved(config_file.clone())?.unwrap_or(AnyCodeConfig {
            provider: "z.ai".to_string(),
            plan: "coding".to_string(),
            api_key: String::new(),
            provider_credentials: HashMap::new(),
            base_url: None,
            model: "glm-5".to_string(),
            temperature: 0.7,
            max_tokens: 8192,
            routing: RoutingConfig::default(),
            runtime: RuntimeSettingsFile::default(),
            security: SecurityConfigFile::default(),
            system_prompt_override: None,
            system_prompt_append: None,
            memory: MemoryConfigFile::default(),
            zai_tool_choice_first_turn: false,
            skills: SkillsConfigFile::default(),
            session: SessionConfigFile::default(),
            model_instructions: ModelInstructionsConfigFile::default(),
            status_line: StatusLineConfigFile::default(),
            terminal: TerminalConfigFile::default(),
            channels: ChannelsConfigFile::default(),
            lsp: LspConfigFile::default(),
            notifications: Default::default(),
        }),
    )
}
