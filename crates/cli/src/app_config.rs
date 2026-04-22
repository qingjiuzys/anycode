//! 用户配置（`~/.anycode/config.json`）、运行时 `Config` 与 `model` / `config` 子命令逻辑。

use crate::cli_args::{ModelAuthCommands, ModelCommands};
use crate::copilot_auth;
use crate::i18n::{tr, tr_args};
use anycode_agent::{ModelInstructionsConfig, RuntimePromptConfig};
use anycode_core::{FeatureFlag, FeatureRegistry, RuntimeMode};
use anycode_llm::{
    normalize_provider_id, transport_for_provider_id, LlmTransport, GOOGLE_MODEL_CATALOG,
    ZAI_MODEL_CATALOG,
};
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

/// `anycode setup` 场景：精简模型配置（仅 provider/model/key），不进入 routing 菜单。
pub(crate) async fn run_model_onboard_interactive(
    config_file: Option<PathBuf>,
) -> anyhow::Result<()> {
    model_interactive::run_onboard(config_file).await
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
            .with_prompt(tr("wizard-pick-model-prompt"))
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
            .with_prompt(tr("wizard-prompt-model-id"))
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

fn is_google_family_provider(p: &str) -> bool {
    matches!(p.trim(), "google" | "gemini")
}

fn prompt_model_for_google(
    is_tty: bool,
    existing: &Option<AnyCodeConfig>,
) -> anyhow::Result<String> {
    use dialoguer::theme::ColorfulTheme;
    use dialoguer::{Input, Select};

    let catalog_items: Vec<String> = GOOGLE_MODEL_CATALOG
        .iter()
        .map(|e| format!("{} — {}", e.id, e.label))
        .chain(std::iter::once(tr("zai-model-custom")))
        .collect();

    let default_model = existing
        .as_ref()
        .filter(|c| is_google_family_provider(&c.provider))
        .map(|c| c.model.clone())
        .unwrap_or_else(|| "gemini-2.5-pro".to_string());

    let idx = if is_tty {
        let default_i = GOOGLE_MODEL_CATALOG
            .iter()
            .position(|e| e.id == default_model.as_str())
            .unwrap_or(0);
        Select::with_theme(&ColorfulTheme::default())
            .with_prompt(tr("wizard-pick-model-prompt"))
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

    if idx < GOOGLE_MODEL_CATALOG.len() {
        return Ok(GOOGLE_MODEL_CATALOG[idx].id.to_string());
    }

    if is_tty {
        Ok(Input::with_theme(&ColorfulTheme::default())
            .with_prompt(tr("wizard-prompt-model-id"))
            .default(default_model)
            .interact_text()?)
    } else {
        let v = prompt_line(&tr("wizard-model-id-non-tty"))?;
        Ok(if v.is_empty() { default_model } else { v })
    }
}

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
            .with_prompt(tr("wizard-pick-anthropic-prompt"))
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
            .with_prompt(tr("wizard-prompt-model-id"))
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
    prompt_base_url: bool,
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
        // 已有 api_key 时必须允许「空回车」，否则 dialoguer 会反复提示，无法「保留已有」。
        Password::with_theme(&ColorfulTheme::default())
            .with_prompt(tr("wizard-api-key-prompt"))
            .allow_empty_password(!default_api_key.is_empty())
            .interact()?
    } else {
        prompt_line(&tr("wizard-api-key-prompt"))?
    };
    let api_key = if api_key.is_empty() {
        default_api_key
    } else {
        api_key
    }
    .trim()
    .to_string();
    if api_key.is_empty() {
        anyhow::bail!("{}", tr("cfg-api-empty"));
    }

    let base_url = if prompt_base_url {
        accent_line_base_url_prompt();
        let recommended_default = recommended_url.to_string();
        let shown_default = if default_base_url.is_empty() {
            recommended_default.clone()
        } else {
            default_base_url.clone()
        };

        let base_url_in: String = if is_tty {
            Input::with_theme(&ColorfulTheme::default())
                .with_prompt(tr("wizard-base-url-merge-pty"))
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
        normalize_base_url_input(&base_url_in, provider_for_merge, recommended_url)
    } else if !default_base_url.trim().is_empty() {
        normalize_base_url_input(&default_base_url, provider_for_merge, recommended_url)
    } else if !recommended_url.trim().is_empty() {
        normalize_base_url_input(recommended_url, provider_for_merge, recommended_url)
    } else {
        None
    };
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
    provider_for_merge: &str,
    recommended_url: &str,
) -> Option<String> {
    let v = base_url_in.trim();
    if v.is_empty() {
        return None;
    }
    let norm_provider = normalize_provider_id(provider_for_merge);
    let requires_explicit_openai_endpoint = transport_for_provider_id(&norm_provider)
        == LlmTransport::OpenAiChatCompletions
        && norm_provider != "z.ai";
    if v == recommended_url && !requires_explicit_openai_endpoint {
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
        model_instructions: existing
            .as_ref()
            .map(|c| c.model_instructions.clone())
            .unwrap_or_default(),
        status_line: existing
            .as_ref()
            .map(|c| c.status_line.clone())
            .unwrap_or_default(),
        terminal: existing
            .as_ref()
            .map(|c| c.terminal.clone())
            .unwrap_or_default(),
        channels: existing
            .as_ref()
            .map(|c| c.channels.clone())
            .unwrap_or_default(),
        lsp: existing.as_ref().map(|c| c.lsp.clone()).unwrap_or_default(),
        notifications: existing
            .as_ref()
            .map(|c| c.notifications.clone())
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

#[path = "app_config_schema.rs"]
mod schema;
pub(crate) use schema::*;

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
    #[serde(default)]
    model_instructions: ModelInstructionsConfigFile,
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
        "pipeline" | "layered" | "guigen" => Ok("pipeline".to_string()),
        _ => {
            let mut a = FluentArgs::new();
            a.set("b", raw.trim());
            anyhow::bail!("{}", tr_args("err-memory-backend", &a));
        }
    }
}

fn normalize_embedding_provider(raw: Option<&str>) -> anyhow::Result<String> {
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

fn resolve_embedding_local_cache_dir(p: Option<PathBuf>) -> anyhow::Result<Option<PathBuf>> {
    let Some(p) = p.filter(|x| !x.as_os_str().is_empty()) else {
        return Ok(None);
    };
    if p.is_absolute() {
        return Ok(Some(p));
    }
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("{}", tr("err-no-home-memory")))?;
    Ok(Some(home.join(p)))
}

fn merge_memory_pipeline_settings(
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
            model_instructions: ModelInstructionsConfigFile::default(),
            status_line: StatusLineConfigFile::default(),
            terminal: TerminalConfigFile::default(),
            channels: ChannelsConfigFile::default(),
            lsp: LspConfigFile::default(),
            notifications: Default::default(),
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
            .with_prompt(tr("cfg-plan-step-pty"))
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
            .with_prompt(tr("cfg-model-step-pty"))
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
                    .with_prompt(tr("cfg-model-custom-pty"))
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
            .with_prompt(tr("cfg-api-step-pty"))
            .allow_empty_password(default_api_key.is_empty())
            .interact()?
    } else {
        prompt_line(&tr("cfg-api-step-fallback"))?
    };
    let api_key = if api_key.is_empty() {
        default_api_key
    } else {
        api_key
    }
    .trim()
    .to_string();
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
            .with_prompt(tr("cfg-base-prompt-pty"))
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
        model_instructions: existing
            .as_ref()
            .map(|c| c.model_instructions.clone())
            .unwrap_or_default(),
        status_line: existing
            .as_ref()
            .map(|c| c.status_line.clone())
            .unwrap_or_default(),
        terminal: existing
            .as_ref()
            .map(|c| c.terminal.clone())
            .unwrap_or_default(),
        channels: existing
            .as_ref()
            .map(|c| c.channels.clone())
            .unwrap_or_default(),
        lsp: existing.as_ref().map(|c| c.lsp.clone()).unwrap_or_default(),
        notifications: existing
            .as_ref()
            .map(|c| c.notifications.clone())
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
        .with_prompt(tr("cfg-wechat-confirm"))
        .default(true)
        .interact()?
    {
        crate::wechat::run_onboard(None, None, false).await?;
    }
    Ok(())
}

fn has_non_empty_secret(v: &str) -> bool {
    !v.trim().is_empty()
}

fn has_usable_model_config(cfg: &AnyCodeConfig) -> bool {
    if cfg.provider.trim().is_empty() || cfg.model.trim().is_empty() {
        return false;
    }
    if validate_llm_provider(&cfg.provider).is_err() {
        return false;
    }
    if has_non_empty_secret(&cfg.api_key) {
        return true;
    }
    cfg.provider_credentials
        .values()
        .any(|v| has_non_empty_secret(v))
}

/// 首次安装聚合：先模型配置，再选择 channel（wechat/telegram/discord）。
pub(crate) async fn run_onboard_flow(
    config_file: Option<PathBuf>,
    data_dir: Option<PathBuf>,
    channel: Option<String>,
    debug: bool,
) -> anyhow::Result<()> {
    crate::workspace::ensure_layout()?;
    {
        let term = console::Term::stdout();
        if term.is_term() {
            // Keep it minimal so it doesn't dominate the setup UX.
            // 单段输出：`println!` 会在整段末尾再补一个 `\n`，格式里不要在最后一行前多加 `\n`，否则会出现「中间空一行」。
            println!(
                "\n    _              ____          __\n   / \\   _ __  _  / ___|___   __/ _| ___\n  / _ \\ | '_ \\| | | |   / _ \\ / _` |/ _ \\\n / ___ \\| | | | |_| |__| (_) | (_| |  __/\n/_/   \\_\\_| |_|\\__, |\\____\\___/ \\__,_|\\___|"
            );
        }
    }
    let existing = load_anycode_config_resolved(config_file.clone())?;
    let already_configured = existing.as_ref().is_some_and(has_usable_model_config);
    let mut reconfigure_model = !already_configured;
    if already_configured {
        println!("检测到已存在可用模型配置，将默认跳过模型配置。");
        println!("如需单独重配，可运行：anycode model");
        let term = console::Term::stdout();
        if term.is_term() {
            use dialoguer::{theme::ColorfulTheme, Select};
            let options = [
                "跳过（使用现有配置，推荐）",
                "现在重配模型（进入 anycode model 简化向导）",
                "退出 setup",
            ];
            let idx = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("模型配置")
                .items(options)
                .default(0)
                .interact()?;
            match idx {
                0 => reconfigure_model = false,
                1 => reconfigure_model = true,
                _ => anyhow::bail!("setup cancelled"),
            }
        } else {
            reconfigure_model = false;
        }
    }
    if reconfigure_model {
        run_model_onboard_interactive(config_file.clone()).await?;
    }

    #[cfg(feature = "embedding-local")]
    {
        if let Err(e) = crate::memory_embedding_setup::run_optional(config_file.clone()) {
            tracing::warn!(target: "anycode_cli", "memory embedding setup skipped: {}", e);
        }
    }

    let selected = match channel
        .as_deref()
        .map(|s| s.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("wechat") => "wechat",
        Some("telegram") => "telegram",
        Some("discord") => "discord",
        Some(other) => {
            anyhow::bail!("unsupported setup channel: {other} (expected wechat/telegram/discord)")
        }
        None => {
            use dialoguer::{theme::ColorfulTheme, Select};
            let term = console::Term::stdout();
            if !term.is_term() {
                println!("setup 未指定 channel，默认选择 wechat（可用 --channel 覆盖）");
                "wechat"
            } else {
                let options = ["wechat", "telegram", "discord"];
                let idx = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("选择要接入的 channel")
                    .items(options)
                    .default(0)
                    .interact()?;
                options[idx]
            }
        }
    };

    match selected {
        "wechat" => crate::wechat::run_onboard(data_dir, config_file, debug).await,
        "telegram" => crate::tg::run_telegram_setup().await,
        "discord" => crate::discord_channel::run_discord_setup().await,
        _ => unreachable!(),
    }
}

/// Resolve the model instructions file path from env var `ANYCODE_MODEL_INSTRUCTIONS_FILE`.
fn resolve_model_instructions_file_from_env() -> Option<PathBuf> {
    std::env::var("ANYCODE_MODEL_INSTRUCTIONS_FILE")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
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
                model_instructions: ModelInstructionsConfigFile::default(),
                status_line: StatusLineConfigFile::default(),
                terminal: TerminalConfigFile::default(),
                channels: ChannelsConfigFile::default(),
                lsp: LspConfigFile::default(),
                notifications: Default::default(),
            }
        }
    };

    validate_permission_mode(cfg.security.permission_mode.trim())?;
    let runtime_mode = validate_runtime_mode(cfg.runtime.default_mode.trim())?;
    validate_llm_provider(&cfg.provider)?;
    validate_notifications(&cfg.notifications)?;

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

    let embedding_provider =
        normalize_embedding_provider(cfg.memory.pipeline.embedding_provider.as_deref())?;
    let mut pipeline = merge_memory_pipeline_settings(&cfg.memory.pipeline);
    if embedding_provider == "local" {
        pipeline.embedding_enabled = true;
    }

    // Resolve model instructions file from env var (e.g., AGENTS.md)
    let model_instructions_file = resolve_model_instructions_file_from_env();

    let mut lsp_runtime: LspRuntime = cfg.lsp.clone().into();
    if let Some(ref p) = lsp_runtime.workspace_root {
        if p.as_os_str().is_empty() {
            lsp_runtime.workspace_root = None;
        } else {
            let full = if p.is_absolute() {
                p.clone()
            } else {
                base_dir.join(p)
            };
            lsp_runtime.workspace_root = std::fs::canonicalize(&full).ok().or(Some(full));
        }
    }

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
            pipeline,
            embedding_model: cfg
                .memory
                .pipeline
                .embedding_model
                .clone()
                .filter(|s| !s.trim().is_empty()),
            embedding_base_url: cfg
                .memory
                .pipeline
                .embedding_base_url
                .clone()
                .filter(|s| !s.trim().is_empty()),
            embedding_provider,
            embedding_local_cache_dir: resolve_embedding_local_cache_dir(
                cfg.memory.pipeline.embedding_local_cache_dir.clone(),
            )?,
            embedding_local_model: cfg
                .memory
                .pipeline
                .embedding_local_model
                .clone()
                .filter(|s| !s.trim().is_empty()),
            embedding_hf_endpoint: cfg
                .memory
                .pipeline
                .embedding_hf_endpoint
                .clone()
                .filter(|s| !s.trim().is_empty()),
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
            skills_section_by_agent: std::collections::HashMap::new(),
            workspace_section: None,
            channel_section: None,
            workflow_section: None,
            goal_section: None,
            prompt_fragments: vec![],
            model_instructions: cfg.model_instructions.into(),
            model_instructions_file,
            model_instructions_content: None,
        },
        skills: cfg.skills.into(),
        session: cfg.session.into(),
        status_line: cfg.status_line.into(),
        terminal: cfg.terminal.into(),
        channels: cfg.channels.into(),
        lsp: lsp_runtime,
        notifications: cfg.notifications,
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
    fn deserializes_notifications_block() {
        let j = r#"{
            "provider":"z.ai",
            "plan":"coding",
            "api_key":"k",
            "base_url":null,
            "model":"glm-5",
            "temperature":0.7,
            "max_tokens":8192,
            "notifications": {
                "http_url": "http://127.0.0.1:9/hook",
                "http_headers": { "Authorization": "Bearer ${HOOKS_TOKEN}" },
                "shell_command": "cat",
                "max_body_bytes": 1024,
                "tool_deny_prefixes": ["mcp__"]
            }
        }"#;
        let c: AnyCodeConfig = serde_json::from_str(j).unwrap();
        validate_notifications(&c.notifications).unwrap();
        assert!(c.notifications.is_configured());
        assert_eq!(
            c.notifications.http_url.as_deref(),
            Some("http://127.0.0.1:9/hook")
        );
        assert_eq!(c.notifications.max_body_bytes, 1024);
        assert_eq!(
            c.notifications.tool_deny_prefixes,
            vec!["mcp__".to_string()]
        );
    }

    #[test]
    fn notifications_validation_rejects_non_http_url_scheme() {
        let mut s = anycode_core::SessionNotificationSettings::default();
        s.http_url = Some("ftp://example.com/hook".to_string());
        s.max_body_bytes = 4096;
        let e = validate_notifications(&s).unwrap_err();
        let m = e.to_string();
        assert!(
            m.contains("notifications") && m.contains("http"),
            "unexpected message: {m}"
        );
    }

    #[test]
    fn notifications_validation_rejects_max_body_out_of_range() {
        let mut s = anycode_core::SessionNotificationSettings::default();
        s.max_body_bytes = 100;
        let e = validate_notifications(&s).unwrap_err();
        assert!(e.to_string().contains("max_body_bytes"), "{}", e);
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
    fn deserializes_memory_pipeline_embedding_local_fields() {
        let j = r#"{
            "provider":"z.ai",
            "plan":"coding",
            "api_key":"k",
            "base_url":null,
            "model":"glm-5",
            "temperature":0.7,
            "max_tokens":8192,
            "memory": {
                "backend": "pipeline",
                "pipeline": {
                    "embedding_provider": "local",
                    "embedding_local_model": "BGESmallZHV15",
                    "embedding_hf_endpoint": "https://hf-mirror.com"
                }
            }
        }"#;
        let c: AnyCodeConfig = serde_json::from_str(j).unwrap();
        assert_eq!(c.memory.backend, "pipeline");
        assert_eq!(
            c.memory.pipeline.embedding_provider.as_deref(),
            Some("local")
        );
        assert_eq!(
            c.memory.pipeline.embedding_local_model.as_deref(),
            Some("BGESmallZHV15")
        );
        assert_eq!(
            c.memory.pipeline.embedding_hf_endpoint.as_deref(),
            Some("https://hf-mirror.com")
        );
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
        validate_session_model_override("z.ai", "glm-5.1").unwrap();
        assert!(validate_session_model_override("z.ai", "not-a-catalog-model").is_err());
    }

    #[test]
    fn session_model_openrouter_allows_non_empty_id() {
        validate_session_model_override("openrouter", "anthropic/claude-3.5-sonnet").unwrap();
    }

    #[test]
    fn session_model_qualified_zai_suffix_must_be_catalog() {
        validate_session_model_override("openrouter", "z.ai/glm-5").unwrap();
        assert!(validate_session_model_override("openrouter", "z.ai/not-in-catalog").is_err());
    }

    #[test]
    fn deserializes_status_line_block() {
        let j = r#"{
            "provider":"z.ai",
            "plan":"coding",
            "api_key":"k",
            "base_url":null,
            "model":"glm-5",
            "temperature":0.7,
            "max_tokens":8192,
            "statusLine": {
                "command": "~/.anycode/statusline.sh",
                "timeout_ms": 4000,
                "padding": 3,
                "show_builtin": true
            }
        }"#;
        let c: AnyCodeConfig = serde_json::from_str(j).unwrap();
        assert_eq!(
            c.status_line.command.as_deref(),
            Some("~/.anycode/statusline.sh")
        );
        assert_eq!(c.status_line.timeout_ms, Some(4000));
        assert_eq!(c.status_line.padding, Some(3));
        assert!(c.status_line.show_builtin);
    }

    #[test]
    fn status_line_runtime_trims_blank_command_to_none() {
        let f = StatusLineConfigFile {
            command: Some("  \n\t  ".to_string()),
            timeout_ms: None,
            padding: None,
            show_builtin: false,
        };
        let r: StatusLineRuntime = f.into();
        assert!(r.command.is_none());
        assert_eq!(r.timeout_ms, 5000);
    }

    #[test]
    fn deserializes_lsp_block() {
        let j = r#"{
            "provider":"z.ai",
            "plan":"coding",
            "api_key":"k",
            "base_url":null,
            "model":"glm-5",
            "temperature":0.7,
            "max_tokens":8192,
            "lsp": {
                "enabled": true,
                "command": "rust-analyzer",
                "workspace_root": "./myproj",
                "read_timeout_ms": 120000
            }
        }"#;
        let c: AnyCodeConfig = serde_json::from_str(j).unwrap();
        assert!(c.lsp.enabled);
        assert_eq!(c.lsp.command.as_deref(), Some("rust-analyzer"));
        assert_eq!(
            c.lsp.workspace_root.as_deref(),
            Some(std::path::Path::new("./myproj"))
        );
        assert_eq!(c.lsp.read_timeout_ms, Some(120_000));
    }
}
