//! 用户配置（`~/.anycode/config.json`）、运行时 `Config` 与 `model` / `config` 子命令逻辑。

use crate::cli_args::{ModelAuthCommands, ModelCommands};
use crate::copilot_auth;
use crate::i18n::{tr, tr_args};
use anycode_agent::{CompactPolicy, RuntimePromptConfig};
use anycode_core::{FeatureFlag, FeatureRegistry, ModelRouteProfile, RuntimeMode};
use anycode_llm::{
    is_known_provider_id, normalize_provider_id, resolve_context_window_tokens,
    transport_for_provider_id, LlmTransport, ZAI_MODEL_CATALOG,
};
use anyhow::Context;
use fluent_bundle::FluentArgs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

#[derive(Debug, Clone, Serialize)]
struct ZaiModelListRow {
    id: &'static str,
    label: &'static str,
}

fn zai_model_list_rows() -> Vec<ZaiModelListRow> {
    ZAI_MODEL_CATALOG
        .iter()
        .map(|e| ZaiModelListRow {
            id: e.api_name,
            label: e.display_name,
        })
        .collect()
}

fn is_known_zai_model(model: &str) -> bool {
    let m = model.trim();
    if m.is_empty() {
        return false;
    }
    ZAI_MODEL_CATALOG.iter().any(|e| e.api_name == m)
}

fn is_zai_family_provider(p: &str) -> bool {
    matches!(p.trim(), "z.ai" | "zai" | "bigmodel")
}

fn is_anthropic_family_provider(p: &str) -> bool {
    matches!(p.trim(), "anthropic" | "claude")
}

#[path = "model_interactive.rs"]
mod model_interactive;

/// 无子命令时：`anycode model` 交互配置（OpenClaw 全目录 + 按任务路由）。
pub(crate) async fn run_model_interactive(config_file: Option<PathBuf>) -> anyhow::Result<()> {
    model_interactive::run(config_file).await
}

fn prompt_model_for_zai(is_tty: bool, existing: &Option<AnyCodeConfig>) -> anyhow::Result<String> {
    use dialoguer::theme::ColorfulTheme;
    use dialoguer::{Input, Select};

    let catalog_items: Vec<String> = ZAI_MODEL_CATALOG
        .iter()
        .map(|e| {
            let mut a = FluentArgs::new();
            a.set("api", e.api_name);
            a.set("display", e.display_name);
            tr_args("zai-model-catalog-entry", &a)
        })
        .chain(std::iter::once(tr("zai-model-custom")))
        .collect();

    let default_model = existing
        .as_ref()
        .filter(|c| is_zai_family_provider(&c.provider))
        .map(|c| c.model.clone())
        .unwrap_or_else(|| "glm-5".to_string());

    let idx = if is_tty {
        let default_i = ZAI_MODEL_CATALOG
            .iter()
            .position(|e| e.api_name == default_model.as_str())
            .unwrap_or(0);
        Select::with_theme(&ColorfulTheme::default())
            .with_prompt(&tr("wizard-pick-model-prompt"))
            .default(default_i.min(catalog_items.len().saturating_sub(1)))
            .items(&catalog_items)
            .interact()?
    } else {
        println!("{}", tr("wizard-pick-model-prompt"));
        for (i, label) in catalog_items.iter().enumerate() {
            println!("  {}) {}", i + 1, label);
        }
        loop {
            let v = prompt_line(&tr("model-pick-number"))?;
            if let Ok(n) = v.trim().parse::<usize>() {
                if n >= 1 && n <= catalog_items.len() {
                    break n - 1;
                }
            }
            println!("{}", tr("model-invalid"));
        }
    };

    if idx < ZAI_MODEL_CATALOG.len() {
        return Ok(ZAI_MODEL_CATALOG[idx].api_name.to_string());
    }

    if is_tty {
        Ok(Input::with_theme(&ColorfulTheme::default())
            .with_prompt(&tr("wizard-prompt-model-id"))
            .default(default_model)
            .interact_text()?)
    } else {
        let v = prompt_line(&tr("wizard-model-id-non-tty"))?;
        Ok(if v.is_empty() { default_model } else { v })
    }
}

const ANTHROPIC_MODEL_CHOICES: &[(&str, &str)] = &[
    ("claude-sonnet-4-20250514", "Claude Sonnet 4"),
    ("claude-3-5-sonnet-20241022", "Claude 3.5 Sonnet"),
    ("claude-3-opus-20240229", "Claude 3 Opus"),
];

fn prompt_model_for_anthropic(
    is_tty: bool,
    existing: &Option<AnyCodeConfig>,
) -> anyhow::Result<String> {
    use dialoguer::theme::ColorfulTheme;
    use dialoguer::{Input, Select};

    let mut labels: Vec<String> = ANTHROPIC_MODEL_CHOICES
        .iter()
        .map(|(id, title)| {
            let mut a = FluentArgs::new();
            a.set("id", *id);
            a.set("title", *title);
            tr_args("anthropic-model-catalog-entry", &a)
        })
        .collect();
    labels.push(tr("anthropic-model-custom"));

    let default_model = existing
        .as_ref()
        .filter(|c| is_anthropic_family_provider(&c.provider))
        .map(|c| c.model.clone())
        .unwrap_or_else(|| "claude-sonnet-4-20250514".to_string());

    let idx = if is_tty {
        let default_i = ANTHROPIC_MODEL_CHOICES
            .iter()
            .position(|(id, _)| *id == default_model.as_str())
            .unwrap_or(0);
        Select::with_theme(&ColorfulTheme::default())
            .with_prompt(&tr("wizard-pick-anthropic-prompt"))
            .default(default_i.min(labels.len().saturating_sub(1)))
            .items(&labels)
            .interact()?
    } else {
        println!("{}", tr("wizard-pick-anthropic-prompt"));
        for (i, label) in labels.iter().enumerate() {
            println!("  {}) {}", i + 1, label);
        }
        loop {
            let v = prompt_line(&tr("model-pick-number"))?;
            if let Ok(n) = v.trim().parse::<usize>() {
                if n >= 1 && n <= labels.len() {
                    break n - 1;
                }
            }
            println!("{}", tr("model-invalid"));
        }
    };

    if idx < ANTHROPIC_MODEL_CHOICES.len() {
        return Ok(ANTHROPIC_MODEL_CHOICES[idx].0.to_string());
    }

    if is_tty {
        Ok(Input::with_theme(&ColorfulTheme::default())
            .with_prompt(&tr("wizard-prompt-model-id"))
            .default(default_model)
            .interact_text()?)
    } else {
        let v = prompt_line(&tr("wizard-model-id-non-tty"))?;
        Ok(if v.is_empty() { default_model } else { v })
    }
}

/// `recommended_url`：用于 Input 默认值；若用户保留该默认，则写入 `None`（由运行时解析官方默认）。
fn prompt_api_key_and_base_url(
    is_tty: bool,
    existing: &Option<AnyCodeConfig>,
    provider_for_merge: &str,
    _plan: &str,
    recommended_url: &str,
) -> anyhow::Result<(String, Option<String>)> {
    use dialoguer::theme::ColorfulTheme;
    use dialoguer::{Input, Password};

    let default_api_key = existing
        .as_ref()
        .filter(|c| {
            c.provider == provider_for_merge
                || (is_zai_family_provider(provider_for_merge)
                    && is_zai_family_provider(&c.provider))
                || (is_anthropic_family_provider(provider_for_merge)
                    && is_anthropic_family_provider(&c.provider))
        })
        .map(|c| c.api_key.clone())
        .unwrap_or_default();

    let default_base_url = existing
        .as_ref()
        .filter(|c| {
            c.provider == provider_for_merge
                || (is_zai_family_provider(provider_for_merge)
                    && is_zai_family_provider(&c.provider))
                || (is_anthropic_family_provider(provider_for_merge)
                    && is_anthropic_family_provider(&c.provider))
        })
        .and_then(|c| c.base_url.clone())
        .unwrap_or_default();

    accent_line_api_key_prompt();
    let api_key: String = if is_tty {
        Password::with_theme(&ColorfulTheme::default())
            .with_prompt(&tr("wizard-api-key-prompt"))
            .allow_empty_password(default_api_key.is_empty())
            .interact()?
    } else {
        prompt_line(&tr("wizard-api-key-prompt"))?
    };
    let api_key = if api_key.is_empty() {
        default_api_key
    } else {
        api_key
    };
    if api_key.is_empty() {
        anyhow::bail!("{}", tr("cfg-api-empty"));
    }

    accent_line_base_url_prompt();
    let recommended_default = recommended_url.to_string();
    let shown_default = if default_base_url.is_empty() {
        recommended_default.clone()
    } else {
        default_base_url.clone()
    };

    let base_url_in: String = if is_tty {
        Input::with_theme(&ColorfulTheme::default())
            .with_prompt(&tr("wizard-base-url-merge-pty"))
            .default(shown_default.clone())
            .interact_text()?
    } else {
        let mut bu = FluentArgs::new();
        bu.set("url", shown_default.clone());
        let v = prompt_line(&tr_args("wizard-base-url-merge-fallback", &bu))?;
        if v.is_empty() {
            shown_default.clone()
        } else {
            v
        }
    };

    let base_url = normalize_base_url_input(&base_url_in, &shown_default, recommended_url);
    Ok((api_key, base_url))
}

fn accent_line_api_key_prompt() {
    use console::Style;
    println!(
        "{}",
        Style::new()
            .cyan()
            .bold()
            .apply_to(tr("wizard-api-key-prompt"))
    );
}

fn accent_line_base_url_prompt() {
    use console::Style;
    println!(
        "{}",
        Style::new()
            .cyan()
            .bold()
            .apply_to(tr("cfg-accent-base-url"))
    );
}

/// 与推荐默认一致则存 `None`，由 LLM 层使用官方默认。
fn normalize_base_url_input(
    base_url_in: &str,
    _shown_default: &str,
    recommended_url: &str,
) -> Option<String> {
    let v = base_url_in.trim();
    if v.is_empty() {
        return None;
    }
    if v == recommended_url {
        return None;
    }
    Some(v.to_string())
}

pub(crate) fn save_merged_config(
    config_file: Option<PathBuf>,
    existing: &Option<AnyCodeConfig>,
    provider: &str,
    plan: &str,
    model: &str,
    api_key: &str,
    base_url: Option<String>,
) -> anyhow::Result<()> {
    let security = existing
        .as_ref()
        .map(|c| c.security.clone())
        .unwrap_or_default();
    let routing = existing
        .as_ref()
        .map(|c| c.routing.clone())
        .unwrap_or_default();
    let runtime = existing
        .as_ref()
        .map(|c| c.runtime.clone())
        .unwrap_or_default();
    let temperature = existing.as_ref().map(|c| c.temperature).unwrap_or(0.7);
    let max_tokens = existing.as_ref().map(|c| c.max_tokens).unwrap_or(8192);
    let system_prompt_override = existing
        .as_ref()
        .and_then(|c| c.system_prompt_override.clone());
    let system_prompt_append = existing
        .as_ref()
        .and_then(|c| c.system_prompt_append.clone());
    let memory = existing
        .as_ref()
        .map(|c| c.memory.clone())
        .unwrap_or_default();
    let zai_tool_choice_first_turn = existing
        .as_ref()
        .map(|c| c.zai_tool_choice_first_turn)
        .unwrap_or(false);

    let cfg = AnyCodeConfig {
        provider: provider.to_string(),
        plan: plan.to_string(),
        api_key: api_key.to_string(),
        provider_credentials: existing
            .as_ref()
            .map(|c| c.provider_credentials.clone())
            .unwrap_or_default(),
        base_url,
        model: model.trim().to_string(),
        temperature,
        max_tokens,
        routing,
        runtime,
        security,
        system_prompt_override,
        system_prompt_append,
        memory,
        zai_tool_choice_first_turn,
        skills: existing
            .as_ref()
            .map(|c| c.skills.clone())
            .unwrap_or_default(),
        session: existing
            .as_ref()
            .map(|c| c.session.clone())
            .unwrap_or_default(),
    };

    validate_llm_provider(&cfg.provider)?;
    let norm = normalize_provider_id(&cfg.provider);
    if matches!(
        transport_for_provider_id(&norm),
        LlmTransport::AnthropicMessages
            | LlmTransport::BedrockConverse
            | LlmTransport::GithubCopilot
    ) && cfg.model.trim().is_empty()
    {
        anyhow::bail!("{}", tr("err-model-required"));
    }

    save_anycode_config_resolved(config_file, &cfg)?;
    Ok(())
}

pub(crate) async fn run_model_command(
    command: ModelCommands,
    config_file: Option<PathBuf>,
) -> anyhow::Result<()> {
    match command {
        ModelCommands::Auth { sub } => {
            match sub {
                ModelAuthCommands::Copilot => {
                    copilot_auth::run_github_copilot_device_login().await?
                }
            }
            Ok(())
        }
        ModelCommands::List { json, plain } => {
            let catalog = zai_model_list_rows();
            if json {
                println!("{}", serde_json::to_string_pretty(&catalog)?);
                return Ok(());
            }
            if plain {
                for m in &catalog {
                    println!("{}", m.id);
                }
                return Ok(());
            }
            println!("Provider: z.ai");
            println!();
            for m in &catalog {
                println!("- {} ({})", m.id, m.label);
            }
            Ok(())
        }
        ModelCommands::Status { json } => {
            let cfg = load_anycode_config_resolved(config_file.clone())?;
            if json {
                println!("{}", serde_json::to_string_pretty(&cfg)?);
                return Ok(());
            }
            match cfg {
                None => {
                    let p = resolve_config_path(config_file)?;
                    let mut a = FluentArgs::new();
                    a.set("path", p.display().to_string());
                    println!("{}", tr_args("wizard-no-config", &a));
                    println!("{}", tr("wizard-run-config-first"));
                    Ok(())
                }
                Some(c) => {
                    println!("provider: {}", c.provider);
                    println!("plan: {}", c.plan);
                    println!("model: {}", c.model);
                    println!(
                        "base_url: {}",
                        c.base_url
                            .clone()
                            .unwrap_or_else(|| "<default>".to_string())
                    );
                    println!("temperature: {}", c.temperature);
                    println!("max_tokens: {}", c.max_tokens);
                    println!("runtime.default_mode: {}", c.runtime.default_mode);
                    println!(
                        "runtime.features: {}",
                        c.runtime.enabled_features.join(", ")
                    );
                    println!("security.permission_mode: {}", c.security.permission_mode);
                    println!("security.require_approval: {}", c.security.require_approval);
                    println!("security.sandbox_mode: {}", c.security.sandbox_mode);
                    match (
                        c.system_prompt_override.as_deref(),
                        c.system_prompt_append.as_deref(),
                    ) {
                        (None, None) => {}
                        (Some(o), None) => {
                            println!(
                                "system_prompt_override: {}…",
                                o.chars().take(80).collect::<String>()
                            );
                        }
                        (None, Some(a)) => {
                            println!(
                                "system_prompt_append: {}…",
                                a.chars().take(80).collect::<String>()
                            );
                        }
                        (Some(o), Some(a)) => {
                            println!(
                                "system_prompt_override: {}…",
                                o.chars().take(40).collect::<String>()
                            );
                            println!(
                                "system_prompt_append: {}…",
                                a.chars().take(40).collect::<String>()
                            );
                        }
                    }
                    Ok(())
                }
            }
        }
        ModelCommands::Set { model } => {
            let path = resolve_config_path(config_file.clone())?;
            let mut cfg = load_anycode_config_resolved(config_file.clone())?.ok_or_else(|| {
                let mut a = FluentArgs::new();
                a.set("path", path.display().to_string());
                anyhow::anyhow!("{}", tr_args("wizard-no-config-model", &a))
            })?;

            let m = model.trim();
            if m.is_empty() {
                anyhow::bail!("{}", tr("wizard-model-empty"));
            }

            if is_zai_family_provider(&cfg.provider) {
                if !is_known_zai_model(m) {
                    let list = ZAI_MODEL_CATALOG
                        .iter()
                        .map(|e| e.api_name)
                        .collect::<Vec<_>>()
                        .join(", ");
                    let mut a = FluentArgs::new();
                    a.set("id", m.to_string());
                    a.set("list", list);
                    anyhow::bail!("{}", tr_args("wizard-unknown-model", &a));
                }
            } else if !is_anthropic_family_provider(&cfg.provider) {
                let mut ap = FluentArgs::new();
                ap.set("p", cfg.provider.clone());
                anyhow::bail!("{}", tr_args("wizard-provider-not-supported", &ap));
            }

            cfg.model = m.to_string();
            save_anycode_config_resolved(config_file, &cfg)?;
            let mut ok = FluentArgs::new();
            ok.set("model", cfg.model.clone());
            println!("✅ {}", tr_args("wizard-model-set-ok", &ok));
            Ok(())
        }
    }
}

/// 配置结构
#[derive(Debug, Clone)]
pub(crate) struct Config {
    pub(crate) llm: LLMConfig,
    pub(crate) memory: MemoryConfig,
    pub(crate) security: SecurityConfig,
    pub(crate) routing: RoutingConfig,
    pub(crate) runtime: RuntimeSettings,
    pub(crate) prompt: RuntimePromptConfig,
    pub(crate) skills: SkillsConfig,
    /// TUI 会话：自动压缩阈值等（`config.json` 的 `session` 段）。
    pub(crate) session: SessionConfig,
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeSettings {
    pub(crate) default_mode: RuntimeMode,
    pub(crate) features: FeatureRegistry,
    pub(crate) model_routes: ModelRouteProfile,
    /// 当前工作目录在 `~/.anycode/workspace/projects/index.json` 中匹配到的项目标签（仅内存叠加，不写回全局配置）。
    pub(crate) workspace_project_label: Option<String>,
    /// 同上：项目级通道 profile 提示（如 `web` / `wechat`）。
    pub(crate) workspace_channel_profile: Option<String>,
}

/// 运行时 `session` 段（与 `SessionConfigFile` 对应）。
#[derive(Debug, Clone)]
pub(crate) struct SessionConfig {
    /// 在发送新用户消息前，若上一轮 LLM 报告的 input tokens 达到阈值则先压缩会话。
    pub(crate) auto_compact: bool,
    /// 绝对阈值（input tokens）；>0 时优先于 `auto_compact_ratio × 有效窗口`。
    pub(crate) auto_compact_min_input_tokens: u32,
    /// 与有效上下文窗口相乘得到阈值（默认 0.88）。
    pub(crate) auto_compact_ratio: f32,
    /// 为 `true` 时根据当前 `provider` + `model` 自动推断窗口（[`resolve_context_window_tokens`]）。
    pub(crate) context_window_auto: bool,
    /// `context_window_auto == false` 时用于比例阈值的手动窗口（tokens）。
    pub(crate) context_window_tokens: u32,
}

impl From<SessionConfigFile> for SessionConfig {
    fn from(f: SessionConfigFile) -> Self {
        Self {
            auto_compact: f.auto_compact,
            auto_compact_min_input_tokens: f.auto_compact_min_input_tokens,
            auto_compact_ratio: f.auto_compact_ratio,
            context_window_auto: f.context_window_auto,
            context_window_tokens: f.context_window_tokens,
        }
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        SessionConfigFile::default().into()
    }
}

/// 有效上下文窗口（tokens）：自动推断或手动配置。
pub(crate) fn effective_session_context_window_tokens(
    session: &SessionConfig,
    provider_raw: &str,
    model_id: &str,
) -> u32 {
    if session.context_window_auto {
        let norm = normalize_provider_id(provider_raw.trim());
        resolve_context_window_tokens(&norm, model_id.trim())
    } else {
        session.context_window_tokens
    }
}

/// 自动压缩触发阈值（input tokens）。`effective_context_window` 由 [`effective_session_context_window_tokens`] 得到。
pub(crate) fn session_auto_compact_threshold(
    cfg: &SessionConfig,
    effective_context_window: u32,
) -> u32 {
    let policy = CompactPolicy {
        trigger_ratio: cfg.auto_compact_ratio.clamp(0.01, 1.0),
        hard_token_threshold: cfg.auto_compact_min_input_tokens,
        suppress_follow_up_questions: true,
    };
    if policy.hard_token_threshold > 0 {
        policy.hard_token_threshold
    } else {
        let t = (effective_context_window as f32) * policy.trigger_ratio;
        if t >= u32::MAX as f32 {
            u32::MAX
        } else {
            t as u32
        }
    }
}

/// TUI：在追加用户消息并发起 turn 之前，是否应先跑一次会话压缩。
pub(crate) fn should_auto_compact_before_send(
    cfg: &SessionConfig,
    provider_raw: &str,
    model_id: &str,
    last_reported_max_input_tokens: u32,
) -> bool {
    if !cfg.auto_compact {
        return false;
    }
    if last_reported_max_input_tokens == 0 {
        return false;
    }
    let win = effective_session_context_window_tokens(cfg, provider_raw, model_id);
    let th = session_auto_compact_threshold(cfg, win);
    th > 0 && last_reported_max_input_tokens >= th
}

#[derive(Debug, Clone)]
pub(crate) struct LLMConfig {
    pub(crate) provider: String,
    pub(crate) plan: String,
    pub(crate) model: String,
    pub(crate) api_key: String,
    pub(crate) base_url: Option<String>,
    pub(crate) temperature: f32,
    pub(crate) max_tokens: u32,
    /// 额外厂商密钥（如全局为 z.ai 时在此存 `anthropic` key，供 routing 混用）。
    pub(crate) provider_credentials: HashMap<String, String>,
    /// z.ai / OpenAI 兼容栈：首轮 agent 请求在带 tools 时使用 `tool_choice: required`（与 `ANYCODE_ZAI_TOOL_CHOICE_FIRST_TURN` 等价；环境变量优先）。
    pub(crate) zai_tool_choice_first_turn: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct MemoryConfig {
    pub(crate) path: PathBuf,
    pub(crate) auto_save: bool,
    /// `noop` | `none` | `off` | `file` | `hybrid`（运行时小写归一）
    pub(crate) backend: String,
}

/// `config.json` 中的 `memory` 段（serde）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MemoryConfigFile {
    /// `noop`（或 `none`/`off`）| `file` | `hybrid`；默认持久化 `file`。
    #[serde(default = "default_memory_backend_kind")]
    pub(crate) backend: String,
    /// 记忆根目录。默认 `$HOME/.anycode/memory`；**相对路径相对于 `$HOME`**。
    #[serde(default)]
    pub(crate) path: Option<PathBuf>,
    #[serde(default = "default_memory_auto_save_file")]
    pub(crate) auto_save: bool,
}

fn default_memory_backend_kind() -> String {
    "file".to_string()
}

fn default_memory_auto_save_file() -> bool {
    true
}

impl Default for MemoryConfigFile {
    fn default() -> Self {
        Self {
            backend: default_memory_backend_kind(),
            path: None,
            auto_save: default_memory_auto_save_file(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SecurityConfig {
    pub(crate) permission_mode: String,
    pub(crate) require_approval: bool,
    pub(crate) sandbox_mode: bool,
    /// 在交给模型前从工具列表中剔除名称匹配任一正则的项（常用于 `mcp__.*` 等）。
    pub(crate) mcp_tool_deny_patterns: Vec<String>,
    /// Claude `alwaysDeny` 式 blanket 串：`mcp__Server` 或 `mcp__Server__*` 整服屏蔽；与 `permissions.ts` `toolMatchesRule` 对齐。
    pub(crate) mcp_tool_deny_rules: Vec<String>,
    /// Claude `alwaysAllow`：blanket 或 `Tool(content)`；content 级在执行前求值，可覆盖 deny。
    pub(crate) always_allow_rules: Vec<String>,
    /// Claude `alwaysAsk`：命中后需交互确认（无回调时拒绝）。
    pub(crate) always_ask_rules: Vec<String>,
    /// 首轮从 LLM 工具列表隐藏全部 `mcp__*`，直至 `ToolSearch` 登记（与 Claude defer MCP 对齐）。
    pub(crate) defer_mcp_tools: bool,
    /// `-I` / `ANYCODE_IGNORE_APPROVAL`：本进程不注册交互式审批回调（不写入配置文件）。
    pub(crate) session_skip_interactive_approval: bool,
}

// ============================================================================
// anyCode 用户级配置（~/.anycode/config.json）
// ============================================================================

fn default_session_auto_compact() -> bool {
    true
}

fn default_auto_compact_ratio() -> f32 {
    0.88
}

fn default_context_window_tokens() -> u32 {
    128_000
}

fn default_context_window_auto() -> bool {
    true
}

/// `config.json` 的 `session` 段（serde）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SessionConfigFile {
    /// 发送新用户消息前是否可按阈值自动压缩会话。
    #[serde(default = "default_session_auto_compact")]
    pub(crate) auto_compact: bool,
    /// 绝对阈值（input tokens）；>0 时优先于比例阈值。
    #[serde(default)]
    pub(crate) auto_compact_min_input_tokens: u32,
    #[serde(default = "default_auto_compact_ratio")]
    pub(crate) auto_compact_ratio: f32,
    /// 为 `true` 时根据 `provider` + `model` 自动推断上下文窗口（见 anycode_llm）。
    #[serde(default = "default_context_window_auto")]
    pub(crate) context_window_auto: bool,
    /// `context_window_auto == false` 时使用的手动窗口大小（tokens）。
    #[serde(default = "default_context_window_tokens")]
    pub(crate) context_window_tokens: u32,
}

impl Default for SessionConfigFile {
    fn default() -> Self {
        Self {
            auto_compact: default_session_auto_compact(),
            auto_compact_min_input_tokens: 0,
            auto_compact_ratio: default_auto_compact_ratio(),
            context_window_auto: default_context_window_auto(),
            context_window_tokens: default_context_window_tokens(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct ModelProfile {
    /// 覆盖全局 `provider`（目录 id，如 `z.ai`、`anthropic`、`openrouter`）
    #[serde(default)]
    pub(crate) provider: Option<String>,
    /// 该 profile 专用 API Key（不填则按厂商从全局 `api_key` 或 `provider_credentials` 解析）
    #[serde(default)]
    pub(crate) api_key: Option<String>,
    /// 套餐：coding / general（不填则沿用全局 plan）
    #[serde(default)]
    pub(crate) plan: Option<String>,
    /// model id（不填则沿用全局 model）
    #[serde(default)]
    pub(crate) model: Option<String>,
    #[serde(default)]
    pub(crate) temperature: Option<f32>,
    #[serde(default)]
    pub(crate) max_tokens: Option<u32>,
    /// 覆盖 base_url（不填则沿用全局 base_url 或 plan 默认）
    #[serde(default)]
    pub(crate) base_url: Option<String>,
}

impl ModelProfile {
    // 预留：后续用于校验/合并 profile
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RoutingConfig {
    /// 默认 profile（可选）
    #[serde(default)]
    pub(crate) default: Option<ModelProfile>,
    /// 按 agent_type 覆盖（如 plan/explore/general-purpose）
    #[serde(default)]
    pub(crate) agents: HashMap<String, ModelProfile>,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            default: None,
            agents: HashMap::new(),
        }
    }
}

fn default_skills_enabled() -> bool {
    true
}

fn default_skill_run_timeout_ms() -> u64 {
    120_000
}

/// `config.json` 中的 `skills` 段（serde）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SkillsConfigFile {
    /// When false, no scan, no prompt injection, `Skill` tool resolves only cwd-based skills.
    #[serde(default = "default_skills_enabled")]
    pub(crate) enabled: bool,
    /// Extra roots scanned before `~/.anycode/skills` (lower precedence than user dir).
    #[serde(default)]
    pub(crate) extra_dirs: Vec<PathBuf>,
    /// If set, only these skill ids appear in the catalog and prompt.
    #[serde(default)]
    pub(crate) allowlist: Option<Vec<String>>,
    #[serde(default = "default_skill_run_timeout_ms")]
    pub(crate) run_timeout_ms: u64,
    /// Strip environment to a small whitelist for `Skill` tool subprocesses.
    #[serde(default)]
    pub(crate) minimal_env: bool,
    /// Also register `Skill` for explore/plan agents (default off).
    #[serde(default)]
    pub(crate) expose_on_explore_plan: bool,
}

impl Default for SkillsConfigFile {
    fn default() -> Self {
        Self {
            enabled: default_skills_enabled(),
            extra_dirs: vec![],
            allowlist: None,
            run_timeout_ms: default_skill_run_timeout_ms(),
            minimal_env: false,
            expose_on_explore_plan: false,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SkillsConfig {
    pub(crate) enabled: bool,
    pub(crate) extra_dirs: Vec<PathBuf>,
    pub(crate) allowlist: Option<Vec<String>>,
    pub(crate) run_timeout_ms: u64,
    pub(crate) minimal_env: bool,
    pub(crate) expose_on_explore_plan: bool,
}

impl From<SkillsConfigFile> for SkillsConfig {
    fn from(f: SkillsConfigFile) -> Self {
        Self {
            enabled: f.enabled,
            extra_dirs: f.extra_dirs,
            allowlist: f.allowlist,
            run_timeout_ms: f.run_timeout_ms,
            minimal_env: f.minimal_env,
            expose_on_explore_plan: f.expose_on_explore_plan,
        }
    }
}

impl Default for SkillsConfig {
    fn default() -> Self {
        SkillsConfigFile::default().into()
    }
}

fn default_runtime_mode() -> String {
    "code".to_string()
}

fn default_runtime_enabled_features() -> Vec<String> {
    vec![
        FeatureFlag::Skills.as_str().to_string(),
        FeatureFlag::ApprovalV2.as_str().to_string(),
        FeatureFlag::ContextCompression.as_str().to_string(),
        FeatureFlag::WorkspaceProfiles.as_str().to_string(),
        FeatureFlag::ChannelMode.as_str().to_string(),
    ]
}

fn default_runtime_model_routes() -> ModelRouteProfile {
    let mut mode_aliases = HashMap::new();
    mode_aliases.insert("general".to_string(), "code".to_string());
    mode_aliases.insert("explore".to_string(), "fast".to_string());
    mode_aliases.insert("plan".to_string(), "plan".to_string());
    mode_aliases.insert("code".to_string(), "code".to_string());
    mode_aliases.insert("channel".to_string(), "channel".to_string());
    mode_aliases.insert("goal".to_string(), "best".to_string());
    let mut agent_aliases = HashMap::new();
    agent_aliases.insert("summary".to_string(), "summary".to_string());
    ModelRouteProfile {
        default_alias: Some("code".to_string()),
        mode_aliases,
        agent_aliases,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RuntimeSettingsFile {
    #[serde(default = "default_runtime_mode")]
    pub(crate) default_mode: String,
    #[serde(default = "default_runtime_enabled_features")]
    pub(crate) enabled_features: Vec<String>,
    #[serde(default = "default_runtime_model_routes")]
    pub(crate) model_routes: ModelRouteProfile,
}

impl Default for RuntimeSettingsFile {
    fn default() -> Self {
        Self {
            default_mode: default_runtime_mode(),
            enabled_features: default_runtime_enabled_features(),
            model_routes: default_runtime_model_routes(),
        }
    }
}

/// 持久化到 ~/.anycode/config.json 的安全相关选项（与运行时 `SecurityConfig` 对应）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SecurityConfigFile {
    /// `default` | `auto` | `plan` | `bypass`
    #[serde(default = "default_security_permission_mode")]
    permission_mode: String,
    #[serde(default = "default_security_require_approval")]
    require_approval: bool,
    #[serde(default)]
    sandbox_mode: bool,
    /// 工具 API 名 deny 正则（例如 `^mcp__prod__`）；非法条目跳过并打日志。
    #[serde(default)]
    mcp_tool_deny_patterns: Vec<String>,
    /// MCP 工具 blanket deny 规则（非正则，见 `mcp_tool_deny_rules` 文档）。
    #[serde(default)]
    mcp_tool_deny_rules: Vec<String>,
    #[serde(default)]
    always_allow_rules: Vec<String>,
    #[serde(default)]
    always_ask_rules: Vec<String>,
    #[serde(default)]
    defer_mcp_tools: bool,
}

fn default_security_permission_mode() -> String {
    "default".to_string()
}

fn default_security_require_approval() -> bool {
    true
}

impl Default for SecurityConfigFile {
    fn default() -> Self {
        Self {
            permission_mode: default_security_permission_mode(),
            require_approval: default_security_require_approval(),
            sandbox_mode: false,
            mcp_tool_deny_patterns: vec![],
            mcp_tool_deny_rules: vec![],
            always_allow_rules: vec![],
            always_ask_rules: vec![],
            defer_mcp_tools: false,
        }
    }
}

fn validate_permission_mode(s: &str) -> anyhow::Result<()> {
    match s {
        "default" | "auto" | "plan" | "accept_edits" | "acceptEdits" | "bypass" => Ok(()),
        _ => {
            let mut a = FluentArgs::new();
            a.set("mode", s);
            anyhow::bail!("{}", tr_args("err-permission-mode", &a));
        }
    }
}

fn validate_runtime_mode(s: &str) -> anyhow::Result<RuntimeMode> {
    RuntimeMode::parse(s).ok_or_else(|| anyhow::anyhow!("invalid runtime mode: {}", s))
}

pub(crate) fn validate_llm_provider(s: &str) -> anyhow::Result<()> {
    let n = normalize_provider_id(s);
    if is_known_provider_id(&n) {
        return Ok(());
    }
    let mut a = FluentArgs::new();
    a.set("p", s);
    anyhow::bail!("{}", tr_args("err-provider", &a));
}

/// `repl --model` 等仅本会话的模型覆盖：与 `model set` 的 z.ai 目录校验一致；Anthropic 允许任意非空 id；
/// 其它厂商须为已知 provider，model 为非空字符串。
pub(crate) fn validate_session_model_override(provider: &str, model: &str) -> anyhow::Result<()> {
    let m = model.trim();
    if m.is_empty() {
        anyhow::bail!("{}", tr("err-model-required"));
    }
    if is_zai_family_provider(provider) {
        if !is_known_zai_model(m) {
            let list = ZAI_MODEL_CATALOG
                .iter()
                .map(|e| e.api_name)
                .collect::<Vec<_>>()
                .join(", ");
            let mut a = FluentArgs::new();
            a.set("id", m);
            a.set("list", list);
            anyhow::bail!("{}", tr_args("err-unknown-zai-model", &a));
        }
    } else if is_anthropic_family_provider(provider) {
        // 与配置文件一致，不强制枚举 Claude model id
    } else {
        validate_llm_provider(provider)?;
    }
    Ok(())
}

pub(crate) fn apply_optional_repl_model(
    config: &mut Config,
    model: Option<String>,
) -> anyhow::Result<()> {
    if let Some(m) = model {
        validate_session_model_override(&config.llm.provider, &m)?;
        config.llm.model = m;
    }
    Ok(())
}

/// 内联文本，或以 `@path` 从文件读取（相对路径相对 `base_dir`，通常为配置文件所在目录）。
fn resolve_system_prompt_field(raw: &str, base_dir: &Path) -> anyhow::Result<String> {
    if let Some(rest) = raw.strip_prefix('@') {
        let path_str = rest.trim();
        let p = if Path::new(path_str).is_absolute() {
            PathBuf::from(path_str)
        } else {
            base_dir.join(path_str)
        };
        fs::read_to_string(&p).with_context(|| {
            let mut a = FluentArgs::new();
            a.set("path", p.display().to_string());
            tr_args("err-read-system-prompt", &a)
        })
    } else {
        Ok(raw.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AnyCodeConfig {
    // V1 固定：z.ai（= BigModel）
    pub(crate) provider: String,
    // 套餐：coding（编码套餐） / general（通用）
    plan: String,
    api_key: String,
    /// 按厂商 id 存额外密钥（如 `anthropic`、`openrouter`），用于与全局不同厂商混跑 routing。
    #[serde(default)]
    provider_credentials: HashMap<String, String>,
    base_url: Option<String>,
    // V1 先固定为 glm-4（后续可扩展为编码套餐的多个模型）
    model: String,
    temperature: f32,
    max_tokens: u32,
    #[serde(default)]
    pub(crate) routing: RoutingConfig,
    #[serde(default)]
    pub(crate) runtime: RuntimeSettingsFile,
    #[serde(default)]
    security: SecurityConfigFile,
    /// 整段覆盖 system（非空则不再注入默认段、记忆、append）。支持 `@相对或绝对路径` 从文件读取。
    #[serde(default)]
    system_prompt_override: Option<String>,
    /// 接在合成 system 末尾。支持 `@path` 读文件（相对路径相对配置文件所在目录）。
    #[serde(default)]
    system_prompt_append: Option<String>,
    #[serde(default)]
    pub(crate) memory: MemoryConfigFile,
    /// z.ai OpenAI 兼容栈：首轮带 tools 时 `tool_choice: required`（环境变量 `ANYCODE_ZAI_TOOL_CHOICE_*` 仍可覆盖）。
    #[serde(default)]
    zai_tool_choice_first_turn: bool,
    #[serde(default)]
    skills: SkillsConfigFile,
    #[serde(default)]
    pub(crate) session: SessionConfigFile,
}

pub(crate) fn default_base_url_for(plan: &str) -> &'static str {
    // 参考：docs.z.ai（2026）
    // - 通用：https://api.z.ai/api/paas/v4/chat/completions
    // - 编码套餐：https://api.z.ai/api/coding/paas/v4/chat/completions
    match plan {
        "coding" => "https://api.z.ai/api/coding/paas/v4/chat/completions",
        _ => "https://api.z.ai/api/paas/v4/chat/completions",
    }
}

fn anycode_config_path() -> anyhow::Result<PathBuf> {
    let home = std::env::var("HOME")?;
    Ok(PathBuf::from(home).join(".anycode").join("config.json"))
}

fn resolve_memory_directory(path_opt: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("{}", tr("err-no-home-memory")))?;
    match path_opt {
        None => Ok(home.join(".anycode/memory")),
        Some(p) if p.is_absolute() => Ok(p),
        Some(p) => Ok(home.join(p)),
    }
}

fn normalize_memory_backend(raw: &str) -> anyhow::Result<String> {
    let b = raw.trim().to_lowercase();
    let b = if b.is_empty() { "file".to_string() } else { b };
    match b.as_str() {
        "noop" | "none" | "off" => Ok("noop".to_string()),
        "file" => Ok("file".to_string()),
        "hybrid" => Ok("hybrid".to_string()),
        _ => {
            let mut a = FluentArgs::new();
            a.set("b", raw.trim());
            anyhow::bail!("{}", tr_args("err-memory-backend", &a));
        }
    }
}

/// `-c` 指定文件，否则 `~/.anycode/config.json`。
fn resolve_config_path(config_file: Option<PathBuf>) -> anyhow::Result<PathBuf> {
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

fn load_anycode_config() -> anyhow::Result<Option<AnyCodeConfig>> {
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

fn save_anycode_config(cfg: &AnyCodeConfig) -> anyhow::Result<()> {
    save_anycode_config_to(&anycode_config_path()?, cfg)
}

fn load_or_default_anycode_config(config_file: Option<PathBuf>) -> anyhow::Result<AnyCodeConfig> {
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
        }),
    )
}

pub(crate) fn enable_feature_flag(
    config_file: Option<PathBuf>,
    feature: &str,
) -> anyhow::Result<Vec<String>> {
    let flag = FeatureFlag::parse(feature)
        .ok_or_else(|| anyhow::anyhow!("unknown feature: {}", feature))?;
    let mut cfg = load_or_default_anycode_config(config_file.clone())?;
    if !cfg
        .runtime
        .enabled_features
        .iter()
        .any(|item| item == flag.as_str())
    {
        cfg.runtime.enabled_features.push(flag.as_str().to_string());
        cfg.runtime.enabled_features.sort();
        cfg.runtime.enabled_features.dedup();
        save_anycode_config_resolved(config_file, &cfg)?;
    }
    Ok(cfg.runtime.enabled_features)
}

pub(crate) fn disable_feature_flag(
    config_file: Option<PathBuf>,
    feature: &str,
) -> anyhow::Result<Vec<String>> {
    let flag = FeatureFlag::parse(feature)
        .ok_or_else(|| anyhow::anyhow!("unknown feature: {}", feature))?;
    let mut cfg = load_or_default_anycode_config(config_file.clone())?;
    cfg.runtime
        .enabled_features
        .retain(|item| item != flag.as_str());
    save_anycode_config_resolved(config_file, &cfg)?;
    Ok(cfg.runtime.enabled_features)
}

pub(crate) fn set_default_runtime_mode(
    config_file: Option<PathBuf>,
    mode: &str,
) -> anyhow::Result<RuntimeMode> {
    let parsed = validate_runtime_mode(mode)?;
    let mut cfg = load_or_default_anycode_config(config_file.clone())?;
    cfg.runtime.default_mode = parsed.as_str().to_string();
    save_anycode_config_resolved(config_file, &cfg)?;
    Ok(parsed)
}

fn prompt_line(label: &str) -> anyhow::Result<String> {
    use std::io::Write;

    print!("{}", label);
    std::io::stdout().flush()?;
    let mut buf = String::new();
    std::io::stdin().read_line(&mut buf)?;
    Ok(buf.trim().to_string())
}

pub(crate) async fn run_config_wizard() -> anyhow::Result<()> {
    run_config_wizard_inner(true).await
}

async fn run_config_wizard_inner(offer_wechat_after: bool) -> anyhow::Result<()> {
    use console::Term;
    use dialoguer::theme::ColorfulTheme;
    use dialoguer::{Input, Password, Select};

    println!("{}", tr("cfg-wizard-title"));
    println!("{}", tr("cfg-wizard-v1"));
    println!("{}", tr("cfg-wizard-path"));
    println!();

    // 允许在非 TTY 环境下回退为“输入编号”模式（Cursor 工具窗口/重定向场景）
    let is_tty = Term::stdout().is_term();

    let existing = load_anycode_config().ok().flatten();
    if existing.is_some() {
        println!("{}", tr("cfg-existing-hint"));
        println!();
    }

    // 1) 选择套餐（与 openclaw 的“coding plan”对齐）
    let plan_idx = if is_tty {
        let plan_items = vec![tr("cfg-plan-coding"), tr("cfg-plan-general")];
        Select::with_theme(&ColorfulTheme::default())
            .with_prompt(&tr("cfg-plan-step-pty"))
            .default(0)
            .items(&plan_items)
            .interact()?
    } else {
        println!("{}", tr("cfg-plan-step-fallback-title"));
        println!("  1) {}", tr("cfg-plan-coding"));
        println!("  2) {}", tr("cfg-plan-general"));
        loop {
            let v = prompt_line(&tr("model-pick-number"))?;
            match v.trim() {
                "1" => break 0,
                "2" => break 1,
                _ => println!("{}", tr("cfg-plan-invalid")),
            }
        }
    };
    let plan = if plan_idx == 0 { "coding" } else { "general" }.to_string();

    // 2) 选择模型（先内置一组常用项 + 自定义）
    let model_idx = if is_tty {
        let model_items = vec![
            tr("cfg-model-glm5"),
            tr("cfg-model-glm47"),
            tr("cfg-model-custom"),
        ];
        Select::with_theme(&ColorfulTheme::default())
            .with_prompt(&tr("cfg-model-step-pty"))
            .default(0)
            .items(&model_items)
            .interact()?
    } else {
        println!("{}", tr("cfg-model-step-fallback-title"));
        println!("  1) {}", tr("cfg-model-glm5"));
        println!("  2) {}", tr("cfg-model-glm47"));
        println!("  3) {}", tr("cfg-model-custom"));
        loop {
            let v = prompt_line(&tr("model-pick-number"))?;
            match v.trim() {
                "1" => break 0,
                "2" => break 1,
                "3" => break 2,
                _ => println!("{}", tr("cfg-model-invalid")),
            }
        }
    };
    let model = match model_idx {
        0 => "glm-5".to_string(),
        1 => "glm-4.7".to_string(),
        _ => {
            if is_tty {
                Input::with_theme(&ColorfulTheme::default())
                    .with_prompt(&tr("cfg-model-custom-pty"))
                    .default("glm-5".to_string())
                    .interact_text()?
            } else {
                let v = prompt_line(&tr("cfg-model-custom-fallback"))?;
                if v.is_empty() {
                    "glm-5".to_string()
                } else {
                    v
                }
            }
        }
    };

    let default_api_key = existing
        .as_ref()
        .map(|c| c.api_key.clone())
        .unwrap_or_default();
    let default_base_url = existing
        .as_ref()
        .and_then(|c| c.base_url.clone())
        .unwrap_or_default();

    let api_key: String = if is_tty {
        Password::with_theme(&ColorfulTheme::default())
            .with_prompt(&tr("cfg-api-step-pty"))
            .allow_empty_password(default_api_key.is_empty())
            .interact()?
    } else {
        prompt_line(&tr("cfg-api-step-fallback"))?
    };
    let api_key = if api_key.is_empty() {
        default_api_key
    } else {
        api_key
    };
    if api_key.is_empty() {
        anyhow::bail!("{}", tr("cfg-api-empty"));
    }

    println!("{}", tr("cfg-base-step-title"));
    let recommended_default = default_base_url_for(&plan).to_string();
    let shown_default = if default_base_url.is_empty() {
        recommended_default.clone()
    } else {
        default_base_url.clone()
    };
    let base_url_in: String = if is_tty {
        Input::with_theme(&ColorfulTheme::default())
            .with_prompt(&tr("cfg-base-prompt-pty"))
            .default(shown_default.clone())
            .interact_text()?
    } else {
        let mut bf = FluentArgs::new();
        bf.set("url", shown_default.clone());
        let v = prompt_line(&tr_args("cfg-base-prompt-fallback", &bf))?;
        if v.is_empty() {
            shown_default.clone()
        } else {
            v
        }
    };
    let base_url = {
        let v = if base_url_in.is_empty() {
            shown_default
        } else {
            base_url_in
        };
        if v.is_empty() {
            None
        } else {
            Some(v)
        }
    };

    let security = existing
        .as_ref()
        .map(|c| c.security.clone())
        .unwrap_or_default();

    let routing = existing
        .as_ref()
        .map(|c| c.routing.clone())
        .unwrap_or_default();
    let runtime = existing
        .as_ref()
        .map(|c| c.runtime.clone())
        .unwrap_or_default();

    let cfg = AnyCodeConfig {
        provider: "z.ai".to_string(),
        plan,
        api_key,
        provider_credentials: existing
            .as_ref()
            .map(|c| c.provider_credentials.clone())
            .unwrap_or_default(),
        base_url,
        model,
        temperature: 0.7,
        max_tokens: 8192,
        routing,
        runtime,
        security,
        system_prompt_override: None,
        system_prompt_append: None,
        memory: existing
            .as_ref()
            .map(|c| c.memory.clone())
            .unwrap_or_default(),
        zai_tool_choice_first_turn: existing
            .as_ref()
            .map(|c| c.zai_tool_choice_first_turn)
            .unwrap_or(false),
        skills: existing
            .as_ref()
            .map(|c| c.skills.clone())
            .unwrap_or_default(),
        session: existing
            .as_ref()
            .map(|c| c.session.clone())
            .unwrap_or_default(),
    };

    save_anycode_config(&cfg)?;
    println!();
    println!("{}", tr("cfg-saved"));
    println!("{}", tr("cfg-next-example-title"));
    println!("{}", tr("cfg-next-example-cmd"));
    if offer_wechat_after {
        maybe_offer_wechat_binding().await?;
    }
    Ok(())
}

/// 配置向导（不提示微信；供 `onboard` 聚合命令在向导后再统一走 `wechat`）。
pub(crate) async fn run_config_wizard_without_wechat_prompt() -> anyhow::Result<()> {
    run_config_wizard_inner(false).await
}

async fn maybe_offer_wechat_binding() -> anyhow::Result<()> {
    use console::Term;
    use dialoguer::theme::ColorfulTheme;
    use dialoguer::Confirm;

    let term = Term::stdout();
    if !term.is_term() {
        println!("{}", tr("cfg-wechat-hint-non-tty"));
        return Ok(());
    }
    if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(&tr("cfg-wechat-confirm"))
        .default(true)
        .interact()?
    {
        crate::wechat::run_onboard(None, None, false).await?;
    }
    Ok(())
}

/// 首次安装聚合：确保 workspace → 缺 API 配置则向导（不重复问微信）→ 可选微信扫码与自启。
pub(crate) async fn run_onboard_flow(
    config_file: Option<PathBuf>,
    data_dir: Option<PathBuf>,
    skip_wechat: bool,
    debug: bool,
) -> anyhow::Result<()> {
    crate::workspace::ensure_layout()?;
    let cfg = load_anycode_config_resolved(config_file.clone())?;
    let need_wizard = match &cfg {
        None => true,
        Some(c) => c.api_key.trim().is_empty(),
    };
    if need_wizard {
        run_config_wizard_without_wechat_prompt().await?;
    }
    if skip_wechat {
        println!("{}", tr("cfg-skip-wechat"));
        return Ok(());
    }
    crate::wechat::run_onboard(data_dir, config_file, debug).await
}

pub(crate) async fn load_config(config_file: Option<PathBuf>) -> anyhow::Result<Config> {
    let default_path = resolve_config_path(None)?;
    let cfg = match load_anycode_config_resolved(config_file.clone())? {
        Some(c) => c,
        None => {
            let mut np = FluentArgs::new();
            np.set("path", default_path.display().to_string());
            eprintln!("{}", tr_args("cfg-no-config-warn", &np));
            eprintln!("{}", tr("cfg-no-config-run"));
            AnyCodeConfig {
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
            }
        }
    };

    validate_permission_mode(cfg.security.permission_mode.trim())?;
    let runtime_mode = validate_runtime_mode(cfg.runtime.default_mode.trim())?;
    validate_llm_provider(&cfg.provider)?;

    let config_path = resolve_config_path(config_file.clone())?;
    let base_dir = config_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    let system_prompt_override = match cfg.system_prompt_override.as_deref() {
        None | Some("") => None,
        Some(s) => {
            let v = resolve_system_prompt_field(s.trim(), base_dir)?;
            let t = v.trim();
            if t.is_empty() {
                None
            } else {
                Some(v)
            }
        }
    };
    let system_prompt_append = match cfg.system_prompt_append.as_deref() {
        None | Some("") => None,
        Some(s) => {
            let v = resolve_system_prompt_field(s.trim(), base_dir)?;
            let t = v.trim();
            if t.is_empty() {
                None
            } else {
                Some(v)
            }
        }
    };

    let memory_path = resolve_memory_directory(cfg.memory.path.clone())?;
    let memory_backend = normalize_memory_backend(&cfg.memory.backend)?;

    Ok(Config {
        llm: LLMConfig {
            provider: cfg.provider,
            plan: cfg.plan,
            model: cfg.model,
            api_key: cfg.api_key,
            base_url: cfg.base_url,
            temperature: cfg.temperature,
            max_tokens: cfg.max_tokens,
            provider_credentials: cfg.provider_credentials,
            zai_tool_choice_first_turn: cfg.zai_tool_choice_first_turn,
        },
        memory: MemoryConfig {
            path: memory_path,
            auto_save: cfg.memory.auto_save,
            backend: memory_backend,
        },
        security: SecurityConfig {
            permission_mode: cfg.security.permission_mode.clone(),
            require_approval: cfg.security.require_approval,
            sandbox_mode: cfg.security.sandbox_mode,
            mcp_tool_deny_patterns: cfg.security.mcp_tool_deny_patterns.clone(),
            mcp_tool_deny_rules: cfg.security.mcp_tool_deny_rules.clone(),
            always_allow_rules: cfg.security.always_allow_rules.clone(),
            always_ask_rules: cfg.security.always_ask_rules.clone(),
            defer_mcp_tools: cfg.security.defer_mcp_tools,
            session_skip_interactive_approval: false,
        },
        routing: cfg.routing,
        runtime: RuntimeSettings {
            default_mode: runtime_mode,
            features: FeatureRegistry::from_enabled(cfg.runtime.enabled_features),
            model_routes: cfg.runtime.model_routes,
            workspace_project_label: None,
            workspace_channel_profile: None,
        },
        prompt: RuntimePromptConfig {
            system_prompt_override,
            system_prompt_append,
            skills_section: None,
            workspace_section: None,
            channel_section: None,
            workflow_section: None,
            goal_section: None,
            prompt_fragments: vec![],
        },
        skills: cfg.skills.into(),
        session: cfg.session.into(),
    })
}

/// 加载配置并套用全局 CLI 覆盖（如 `--ignore-approval`）。
pub(crate) async fn load_config_for_session(
    config_file: Option<PathBuf>,
    ignore_approval: bool,
) -> anyhow::Result<Config> {
    let mut config = load_config(config_file).await?;
    apply_ignore_approval_cli(&mut config, ignore_approval);
    Ok(config)
}

/// CLI `--ignore-approval` / `--ignore`：仅影响当前进程，不修改配置文件。
fn apply_ignore_approval_cli(config: &mut Config, ignore_approval: bool) {
    config.security.session_skip_interactive_approval = ignore_approval;
    if ignore_approval {
        if config.security.require_approval {
            info!("{}", tr("log-ignore-approval-session"));
        }
        config.security.require_approval = false;
    }
}

/// 是否应为敏感工具 / Claude `alwaysAsk` 注册交互式 `ApprovalCallback`（无 `approval_override` 时）。
pub(crate) fn security_wants_interactive_approval_callback(config: &Config) -> bool {
    !config.security.session_skip_interactive_approval
        && (config.security.require_approval || !config.security.always_ask_rules.is_empty())
}

/// 微信桥进程无终端，无法完成工具交互审批；强制关闭 `require_approval`（不写回配置文件）。
pub(crate) fn apply_wechat_bridge_no_tool_approval(config: &mut Config) {
    if config.security.require_approval {
        info!("{}", tr("log-wechat-bridge-no-approval"));
        config.security.require_approval = false;
    }
}

#[cfg(test)]
mod serde_config_tests {
    use super::*;

    #[test]
    fn auto_compact_threshold_prefers_absolute() {
        let mut s = SessionConfig::default();
        s.auto_compact_min_input_tokens = 50_000;
        s.context_window_tokens = 100_000;
        s.auto_compact_ratio = 0.5;
        assert_eq!(session_auto_compact_threshold(&s, 200_000), 50_000);
    }

    #[test]
    fn should_auto_compact_respects_zero_last_tokens() {
        let mut s = SessionConfig::default();
        s.context_window_auto = false;
        s.context_window_tokens = 128_000;
        assert!(!should_auto_compact_before_send(&s, "z.ai", "glm-5", 0));
        assert!(!should_auto_compact_before_send(
            &s, "z.ai", "glm-5", 100_000
        ));
        assert!(should_auto_compact_before_send(
            &s, "z.ai", "glm-5", 120_000
        ));
    }

    #[test]
    fn auto_resolved_window_claude_triggers_near_200k() {
        let s = SessionConfig::default();
        assert!(should_auto_compact_before_send(
            &s,
            "anthropic",
            "claude-3-5-sonnet-20241022",
            180_000
        ));
        assert!(!should_auto_compact_before_send(
            &s,
            "anthropic",
            "claude-3-5-sonnet-20241022",
            170_000
        ));
    }

    #[test]
    fn deserializes_legacy_json_without_security_or_routing() {
        let j = r#"{
            "provider":"z.ai",
            "plan":"coding",
            "api_key":"k",
            "base_url":null,
            "model":"glm-5",
            "temperature":0.7,
            "max_tokens":8192
        }"#;
        let c: AnyCodeConfig = serde_json::from_str(j).unwrap();
        assert_eq!(c.security.permission_mode, "default");
        assert!(c.security.require_approval);
        assert!(!c.security.sandbox_mode);
        assert!(c.routing.agents.is_empty());
        assert!(c.system_prompt_override.is_none());
        assert!(c.system_prompt_append.is_none());
        assert!(c.session.auto_compact);
        assert!(c.session.context_window_auto);
    }

    #[test]
    fn deserializes_session_block() {
        let j = r#"{
            "provider":"z.ai",
            "plan":"coding",
            "api_key":"k",
            "base_url":null,
            "model":"glm-5",
            "temperature":0.7,
            "max_tokens":8192,
            "session": {
                "auto_compact": false,
                "auto_compact_min_input_tokens": 90000,
                "context_window_tokens": 200000
            }
        }"#;
        let c: AnyCodeConfig = serde_json::from_str(j).unwrap();
        assert!(!c.session.auto_compact);
        assert_eq!(c.session.auto_compact_min_input_tokens, 90_000);
        assert_eq!(c.session.context_window_tokens, 200_000);
    }

    #[test]
    fn deserializes_memory_and_zai_tool_flag() {
        let j = r#"{
            "provider":"z.ai",
            "plan":"coding",
            "api_key":"k",
            "base_url":null,
            "model":"glm-5",
            "temperature":0.7,
            "max_tokens":8192,
            "memory": {
                "backend": "hybrid",
                "path": ".anycode/mem-test",
                "auto_save": false
            },
            "zai_tool_choice_first_turn": true
        }"#;
        let c: AnyCodeConfig = serde_json::from_str(j).unwrap();
        assert_eq!(c.memory.backend, "hybrid");
        assert_eq!(
            c.memory.path.as_ref().and_then(|p| p.to_str()),
            Some(".anycode/mem-test")
        );
        assert!(!c.memory.auto_save);
        assert!(c.zai_tool_choice_first_turn);
    }

    #[test]
    fn deserializes_security_block() {
        let j = r#"{
            "provider":"z.ai",
            "plan":"coding",
            "api_key":"k",
            "base_url":null,
            "model":"glm-5",
            "temperature":0.7,
            "max_tokens":8192,
            "security": {
                "permission_mode": "bypass",
                "require_approval": false,
                "sandbox_mode": true
            }
        }"#;
        let c: AnyCodeConfig = serde_json::from_str(j).unwrap();
        assert_eq!(c.security.permission_mode, "bypass");
        assert!(!c.security.require_approval);
        assert!(c.security.sandbox_mode);
        validate_permission_mode(&c.security.permission_mode).unwrap();
    }

    #[test]
    fn deserializes_mcp_tool_deny_rules() {
        let j = r#"{
            "provider":"z.ai",
            "plan":"coding",
            "api_key":"k",
            "base_url":null,
            "model":"glm-5",
            "temperature":0.7,
            "max_tokens":8192,
            "security": {
                "mcp_tool_deny_rules": ["mcp__slack", "Bash"]
            }
        }"#;
        let c: AnyCodeConfig = serde_json::from_str(j).unwrap();
        assert_eq!(c.security.mcp_tool_deny_rules, vec!["mcp__slack", "Bash"]);
    }

    #[test]
    fn deserializes_mcp_tool_deny_patterns() {
        let j = r#"{
            "provider":"z.ai",
            "plan":"coding",
            "api_key":"k",
            "base_url":null,
            "model":"glm-5",
            "temperature":0.7,
            "max_tokens":8192,
            "security": {
                "mcp_tool_deny_patterns": ["^mcp__secret__.*"]
            }
        }"#;
        let c: AnyCodeConfig = serde_json::from_str(j).unwrap();
        assert_eq!(c.security.mcp_tool_deny_patterns.len(), 1);
        assert_eq!(c.security.mcp_tool_deny_patterns[0], "^mcp__secret__.*");
    }

    #[test]
    fn permission_mode_invalid_is_rejected() {
        assert!(validate_permission_mode("typo").is_err());
        assert!(validate_permission_mode("").is_err());
    }

    #[test]
    fn llm_provider_anthropic_ok() {
        validate_llm_provider("anthropic").unwrap();
        validate_llm_provider("claude").unwrap();
    }

    #[test]
    fn llm_provider_openclaw_catalog_and_kebab() {
        validate_llm_provider("groq").unwrap();
        validate_llm_provider("fireworks").unwrap();
        validate_llm_provider("cloudflare-ai-gateway").unwrap();
        validate_llm_provider("amazon-bedrock").unwrap();
        validate_llm_provider("kimi").unwrap();
    }

    #[test]
    fn llm_provider_invalid_is_rejected() {
        assert!(validate_llm_provider("totally-unknown-vendor-xyz").is_err());
    }

    #[test]
    fn session_model_zai_must_be_catalog() {
        validate_session_model_override("z.ai", "glm-5").unwrap();
        assert!(validate_session_model_override("z.ai", "not-a-catalog-model").is_err());
    }

    #[test]
    fn session_model_openrouter_allows_non_empty_id() {
        validate_session_model_override("openrouter", "anthropic/claude-3.5-sonnet").unwrap();
    }
}
