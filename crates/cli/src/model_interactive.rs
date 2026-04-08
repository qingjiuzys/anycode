//! `anycode model` 交互（全量提供方目录 + anyCode 按任务路由）。

use super::{
    default_base_url_for, load_anycode_config_resolved, prompt_api_key_and_base_url, prompt_line,
    prompt_model_for_anthropic, prompt_model_for_google, prompt_model_for_zai, resolve_config_path,
    save_anycode_config_resolved, save_merged_config, validate_llm_provider, ModelProfile,
};
use crate::i18n::{tr, tr_args};
use anycode_llm::{
    normalize_provider_id, transport_for_provider_id, LlmTransport, ProviderCatalogEntry,
    PROVIDER_CATALOG, ROUTING_AGENT_PRESETS, ZAI_AUTH_METHODS,
};
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Input, Password, Select};
use fluent_bundle::FluentArgs;
use std::path::PathBuf;

fn accent_title(line: &str) {
    println!("{}", console::Style::new().cyan().bold().apply_to(line));
}

#[derive(Clone, Copy)]
struct AuthChoice {
    id: &'static str,
    label: &'static str,
    provider: &'static str,
    plan: &'static str,
    default_model: &'static str,
    base_url: &'static str,
    key_envs: &'static [&'static str],
}

const QUICK_AUTH_CHOICES: &[AuthChoice] = &[
    AuthChoice {
        id: "zai-coding",
        label: "z.ai Coding Plan — Global (api.z.ai)",
        provider: "z.ai",
        plan: "coding",
        default_model: "glm-5",
        base_url: "https://api.z.ai/api/coding/paas/v4/chat/completions",
        key_envs: &["ZAI_API_KEY"],
    },
    AuthChoice {
        id: "zai-coding-cn",
        label: "z.ai / 智谱 国内编码套餐 (open.bigmodel.cn)",
        provider: "z.ai",
        plan: "coding_cn",
        default_model: "glm-5",
        base_url: "https://open.bigmodel.cn/api/coding/paas/v4/chat/completions",
        key_envs: &["ZAI_API_KEY"],
    },
    AuthChoice {
        id: "zai-general",
        label: "z.ai General — Global (api.z.ai)",
        provider: "z.ai",
        plan: "general",
        default_model: "glm-5",
        base_url: "https://api.z.ai/api/paas/v4/chat/completions",
        key_envs: &["ZAI_API_KEY"],
    },
    AuthChoice {
        id: "deepseek-api-key",
        label: "DeepSeek API Key",
        provider: "deepseek",
        plan: "general",
        default_model: "deepseek-chat",
        base_url: "https://api.deepseek.com/v1/chat/completions",
        key_envs: &["DEEPSEEK_API_KEY"],
    },
    AuthChoice {
        id: "gemini-api-key",
        label: "Google Gemini API Key",
        provider: "google",
        plan: "general",
        default_model: "gemini-2.5-pro",
        base_url: "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions",
        key_envs: &["GEMINI_API_KEY", "GOOGLE_API_KEY"],
    },
    AuthChoice {
        id: "qwen-api-key",
        label: "Qwen API Key (Global Coding Plan)",
        provider: "qwen",
        plan: "general",
        default_model: "qwen3-coder-plus",
        base_url: "https://coding-intl.dashscope.aliyuncs.com/v1/chat/completions",
        key_envs: &["QWEN_API_KEY", "MODELSTUDIO_API_KEY", "DASHSCOPE_API_KEY"],
    },
    AuthChoice {
        id: "qwen-api-key-cn",
        label: "Qwen API Key (China Coding Plan)",
        provider: "qwen",
        plan: "general",
        default_model: "qwen3-coder-plus",
        base_url: "https://coding.dashscope.aliyuncs.com/v1/chat/completions",
        key_envs: &["QWEN_API_KEY", "MODELSTUDIO_API_KEY", "DASHSCOPE_API_KEY"],
    },
    AuthChoice {
        id: "anthropic-api-key",
        label: "Anthropic API Key",
        provider: "anthropic",
        plan: "general",
        default_model: "claude-sonnet-4-20250514",
        base_url: "https://api.anthropic.com/v1/messages",
        key_envs: &["ANTHROPIC_API_KEY"],
    },
    AuthChoice {
        id: "openai-api-key",
        label: "OpenAI API Key",
        provider: "openai",
        plan: "general",
        default_model: "gpt-4.1",
        base_url: "https://api.openai.com/v1/chat/completions",
        key_envs: &["OPENAI_API_KEY"],
    },
];

pub(super) async fn run(config_file: Option<PathBuf>) -> anyhow::Result<()> {
    let is_tty = console::Term::stdout().is_term();
    let path = resolve_config_path(config_file.clone())?;

    println!("{}", tr("model-banner"));
    println!("{} {}", tr("model-config-path"), path.display());
    println!();

    loop {
        accent_title(&tr("model-main-menu-title"));
        let hub_idx = if is_tty {
            let items = vec![
                tr("model-menu-global"),
                tr("model-menu-routing"),
                tr("model-menu-exit"),
            ];
            Select::with_theme(&ColorfulTheme::default())
                .with_prompt(&tr("model-pick-prompt"))
                .default(0)
                .items(&items)
                .interact()?
        } else {
            println!("{}", tr("model-menu-fallback-1"));
            println!("{}", tr("model-menu-fallback-2"));
            println!("{}", tr("model-menu-fallback-0"));
            loop {
                let v = prompt_line(&tr("model-pick-number"))?;
                match v.trim() {
                    "1" => break 0,
                    "2" => break 1,
                    "0" => return Ok(()),
                    _ => println!("{}", tr("model-invalid")),
                }
            }
        };

        match hub_idx {
            0 => run_global_provider_flow(config_file.clone(), &path, is_tty, false).await?,
            1 => run_routing_agents_flow(config_file.clone(), &path, is_tty).await?,
            _ => return Ok(()),
        }
        println!();
    }
}

/// setup 场景的精简模型向导：直接进入 provider 配置，不展示 routing 菜单。
pub(super) async fn run_onboard(config_file: Option<PathBuf>) -> anyhow::Result<()> {
    let is_tty = console::Term::stdout().is_term();
    let path = resolve_config_path(config_file.clone())?;
    let existing = load_anycode_config_resolved(config_file.clone())?;

    println!("模型配置（OpenClaw 风格）");
    if let Some(ref c) = existing {
        println!("当前：provider={} model={}", c.provider, c.model);
    }

    let mode_items = vec![
        "快速配置（常用 auth-choice）".to_string(),
        "完整提供商目录（全量 provider）".to_string(),
    ];
    let mode_idx = if is_tty {
        Select::with_theme(&ColorfulTheme::default())
            .with_prompt("请选择配置模式")
            .default(0)
            .items(&mode_items)
            .interact()?
    } else {
        println!("请选择配置模式：");
        for (i, l) in mode_items.iter().enumerate() {
            println!("  {}) {}", i + 1, l);
        }
        loop {
            let v = prompt_line("输入序号")?;
            if let Ok(n) = v.trim().parse::<usize>() {
                if n >= 1 && n <= mode_items.len() {
                    break n - 1;
                }
            }
            println!("{}", tr("model-invalid"));
        }
    };

    if mode_idx == 1 {
        return run_global_provider_flow(config_file, &path, is_tty, true).await;
    }

    let mut items: Vec<String> = QUICK_AUTH_CHOICES
        .iter()
        .map(|c| format!("{}  ({})", c.label, c.id))
        .collect();
    items.push("切换到完整提供商目录（全量）".to_string());

    let idx = if is_tty {
        Select::with_theme(&ColorfulTheme::default())
            .with_prompt("请选择 auth-choice")
            .default(0)
            .items(&items)
            .interact()?
    } else {
        println!("请选择 auth-choice：");
        for (i, l) in items.iter().enumerate() {
            println!("  {}) {}", i + 1, l);
        }
        loop {
            let v = prompt_line("输入序号")?;
            if let Ok(n) = v.trim().parse::<usize>() {
                if n >= 1 && n <= items.len() {
                    break n - 1;
                }
            }
            println!("{}", tr("model-invalid"));
        }
    };

    if idx >= QUICK_AUTH_CHOICES.len() {
        return run_global_provider_flow(config_file, &path, is_tty, true).await;
    }
    apply_quick_auth_choice(
        config_file,
        &path,
        is_tty,
        &existing,
        QUICK_AUTH_CHOICES[idx],
    )
    .await
}

fn lookup_env_first(envs: &[&str]) -> Option<String> {
    for k in envs {
        if let Ok(v) = std::env::var(k) {
            let t = v.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    None
}

async fn apply_quick_auth_choice(
    config_file: Option<PathBuf>,
    path: &std::path::Path,
    is_tty: bool,
    existing: &Option<super::AnyCodeConfig>,
    choice: AuthChoice,
) -> anyhow::Result<()> {
    let existing_key = existing
        .as_ref()
        .filter(|c| normalize_provider_id(&c.provider) == normalize_provider_id(choice.provider))
        .map(|c| c.api_key.clone())
        .unwrap_or_default();
    let default_key = lookup_env_first(choice.key_envs).unwrap_or(existing_key);

    println!("已选择：{} ({})", choice.label, choice.id);
    let key_prompt = format!("{} API Key", choice.provider);
    let api_key = if is_tty {
        let input = Password::with_theme(&ColorfulTheme::default())
            .with_prompt(key_prompt)
            .allow_empty_password(!default_key.trim().is_empty())
            .interact()?;
        if input.trim().is_empty() {
            default_key
        } else {
            input
        }
    } else {
        let input = prompt_line("请输入 API Key（回车保留已有值）")?;
        if input.trim().is_empty() {
            default_key
        } else {
            input
        }
    };
    let api_key = api_key.trim().to_string();
    if api_key.is_empty() {
        anyhow::bail!("API Key 不能为空");
    }

    let model = existing
        .as_ref()
        .filter(|c| normalize_provider_id(&c.provider) == normalize_provider_id(choice.provider))
        .map(|c| c.model.clone())
        .filter(|m| !m.trim().is_empty())
        .unwrap_or_else(|| choice.default_model.to_string());
    let base_url = Some(choice.base_url.to_string());

    save_merged_config(
        config_file,
        existing,
        choice.provider,
        choice.plan,
        &model,
        &api_key,
        base_url,
    )?;
    let mut sa = FluentArgs::new();
    sa.set("path", path.display().to_string());
    println!("✅ {}", tr_args("wizard-saved", &sa));
    Ok(())
}

async fn run_global_provider_flow(
    config_file: Option<PathBuf>,
    path: &std::path::Path,
    is_tty: bool,
    quick_mode: bool,
) -> anyhow::Result<()> {
    let existing = load_anycode_config_resolved(config_file.clone())?;
    if let Some(ref c) = existing {
        let mut a = FluentArgs::new();
        a.set("p", c.provider.clone());
        a.set("l", c.plan.clone());
        a.set("m", c.model.clone());
        println!("{}", tr_args("model-current-global", &a));
        println!();
    }

    'outer: loop {
        accent_title(&tr("model-provider-title"));
        let labels: Vec<String> = PROVIDER_CATALOG
            .iter()
            .map(|e| {
                if quick_mode {
                    return e.label.to_string();
                }
                let hint = e.hint.map(|h| format!(" ({})", h)).unwrap_or_default();
                if e.placeholder_only {
                    let mut a = FluentArgs::new();
                    a.set("label", e.label);
                    a.set("hint", hint);
                    tr_args("model-catalog-placeholder", &a)
                } else {
                    format!("{}{}", e.label, hint)
                }
            })
            .chain(std::iter::once(tr("model-back-menu")))
            .collect();

        let idx = if is_tty {
            Select::with_theme(&ColorfulTheme::default())
                .with_prompt(&tr("model-pick-provider"))
                .default(0)
                .items(&labels)
                .interact()?
        } else {
            println!("{}", tr("model-provider-list"));
            for (i, l) in labels.iter().enumerate() {
                println!("  {}) {}", i + 1, l);
            }
            println!("  0) {}", tr("model-back-menu"));
            loop {
                let v = prompt_line(&tr("model-pick-number"))?;
                if v.trim() == "0" {
                    return Ok(());
                }
                if let Ok(n) = v.trim().parse::<usize>() {
                    if n >= 1 && n <= labels.len() {
                        break n - 1;
                    }
                }
                println!("{}", tr("model-invalid"));
            }
        };

        if idx >= PROVIDER_CATALOG.len() {
            return Ok(());
        }

        let entry = &PROVIDER_CATALOG[idx];
        if entry.placeholder_only {
            let mut ah = FluentArgs::new();
            ah.set("label", entry.label.to_string());
            let hint_str = entry
                .hint
                .map(|s| s.to_string())
                .unwrap_or_else(|| tr("model-placeholder-default-hint"));
            ah.set("hint", hint_str);
            println!("ℹ️ {}", tr_args("model-placeholder-hint", &ah));
            continue 'outer;
        }

        let id = entry.id.to_string();
        if id == "z.ai" {
            accent_title(&tr("model-zai-auth-title"));
            let zai_labels: Vec<String> = ZAI_AUTH_METHODS
                .iter()
                .map(|z| {
                    z.hint
                        .map(|h| format!("{} ({})", z.label, h))
                        .unwrap_or_else(|| z.label.to_string())
                })
                .chain(std::iter::once(tr("model-back")))
                .collect();

            let zi = if is_tty {
                Select::with_theme(&ColorfulTheme::default())
                    .with_prompt(&tr("model-pick-prompt"))
                    .default(0)
                    .items(&zai_labels)
                    .interact()?
            } else {
                for (i, l) in zai_labels.iter().enumerate() {
                    println!("  {}) {}", i + 1, l);
                }
                println!("  0) {}", tr("model-back"));
                loop {
                    let v = prompt_line(&tr("model-pick-number"))?;
                    match v.trim() {
                        "0" => continue 'outer,
                        s => {
                            if let Ok(n) = s.parse::<usize>() {
                                if n >= 1 && n <= zai_labels.len() {
                                    break n - 1;
                                }
                            }
                            println!("{}", tr("model-invalid"));
                        }
                    }
                }
            };

            if zi >= ZAI_AUTH_METHODS.len() {
                continue 'outer;
            }
            let plan = ZAI_AUTH_METHODS[zi].plan.to_string();
            let model = prompt_model_for_zai(is_tty, &existing)?;
            let (api_key, base_url) = prompt_api_key_and_base_url(
                is_tty,
                &existing,
                "z.ai",
                &plan,
                default_base_url_for(ZAI_AUTH_METHODS[zi].plan),
                true,
            )?;
            save_merged_config(
                config_file.clone(),
                &existing,
                "z.ai",
                &plan,
                &model,
                &api_key,
                base_url,
            )?;
            let mut sa = FluentArgs::new();
            sa.set("path", path.display().to_string());
            println!("✅ {}", tr_args("wizard-saved", &sa));
            return Ok(());
        }

        if transport_for_provider_id(&id) == LlmTransport::AnthropicMessages {
            let model = prompt_model_for_anthropic(is_tty, &existing)?;
            let (api_key, base_url) = prompt_api_key_and_base_url(
                is_tty,
                &existing,
                "anthropic",
                "general",
                "https://api.anthropic.com/v1/messages",
                true,
            )?;
            save_merged_config(
                config_file.clone(),
                &existing,
                "anthropic",
                "general",
                &model,
                &api_key,
                base_url,
            )?;
            let mut sa = FluentArgs::new();
            sa.set("path", path.display().to_string());
            println!("✅ {}", tr_args("wizard-saved", &sa));
            return Ok(());
        }

        if id == "google" {
            let model = prompt_model_for_google(is_tty, &existing)?;
            let default_url = entry.suggested_openai_base.unwrap_or("");
            let (api_key, base_url) = prompt_api_key_and_base_url(
                is_tty,
                &existing,
                "google",
                "general",
                default_url,
                true,
            )?;
            save_merged_config(
                config_file.clone(),
                &existing,
                "google",
                "general",
                &model,
                &api_key,
                base_url,
            )?;
            let mut sa = FluentArgs::new();
            sa.set("path", path.display().to_string());
            println!("✅ {}", tr_args("wizard-saved", &sa));
            return Ok(());
        }

        if transport_for_provider_id(&id) == LlmTransport::BedrockConverse {
            let model: String = if is_tty {
                Input::with_theme(&ColorfulTheme::default())
                    .with_prompt(&tr("wizard-prompt-model-id"))
                    .interact_text()?
            } else {
                prompt_line(&format!("{} ", tr("wizard-prompt-model-id")))?
            };
            let base_url: Option<String> = if is_tty {
                let raw: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt(&tr("wizard-bedrock-endpoint-prompt"))
                    .allow_empty(true)
                    .interact_text()?;
                let t = raw.trim();
                if t.is_empty() {
                    None
                } else {
                    Some(t.to_string())
                }
            } else {
                let v = prompt_line(&format!("{} ", tr("wizard-bedrock-endpoint-prompt")))?;
                let t = v.trim();
                if t.is_empty() {
                    None
                } else {
                    Some(t.to_string())
                }
            };
            save_merged_config(
                config_file.clone(),
                &existing,
                "amazon_bedrock",
                "general",
                &model,
                "",
                base_url,
            )?;
            let mut sa = FluentArgs::new();
            sa.set("path", path.display().to_string());
            println!("✅ {}", tr_args("wizard-saved", &sa));
            return Ok(());
        }

        if transport_for_provider_id(&id) == LlmTransport::GithubCopilot {
            let model: String = if is_tty {
                Input::with_theme(&ColorfulTheme::default())
                    .with_prompt(&tr("wizard-copilot-model-prompt"))
                    .interact_text()?
            } else {
                prompt_line(&format!("{} ", tr("wizard-copilot-model-prompt")))?
            };
            let (api_key, base_url) = prompt_api_key_and_base_url(
                is_tty,
                &existing,
                "github_copilot",
                "general",
                "https://api.individual.githubcopilot.com",
                true,
            )?;
            save_merged_config(
                config_file.clone(),
                &existing,
                "github_copilot",
                "general",
                &model,
                &api_key,
                base_url,
            )?;
            let mut sa = FluentArgs::new();
            sa.set("path", path.display().to_string());
            println!("✅ {}", tr_args("wizard-saved", &sa));
            return Ok(());
        }

        let default_url = entry.suggested_openai_base.unwrap_or("");
        let default_model = existing
            .as_ref()
            .filter(|c| normalize_provider_id(&c.provider) == id)
            .map(|c| c.model.clone())
            .unwrap_or_else(|| provider_default_model(&id).to_string());
        let model: String = if is_tty {
            Input::with_theme(&ColorfulTheme::default())
                .with_prompt(&tr("wizard-prompt-model-id"))
                .default(default_model.clone())
                .interact_text()?
        } else {
            let v = prompt_line(&format!("{} ", tr("wizard-prompt-model-id")))?;
            if v.trim().is_empty() {
                default_model.clone()
            } else {
                v
            }
        };
        let prompt_base_url = if quick_mode && !default_url.is_empty() {
            if is_tty {
                use dialoguer::Confirm;
                !Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt(format!("使用推荐 endpoint：{} ？", default_url))
                    .default(true)
                    .interact()?
            } else {
                false
            }
        } else {
            true
        };
        let (api_key, base_url) = prompt_api_key_and_base_url(
            is_tty,
            &existing,
            &id,
            "general",
            default_url,
            prompt_base_url,
        )?;
        save_merged_config(
            config_file.clone(),
            &existing,
            &id,
            "general",
            &model,
            &api_key,
            base_url,
        )?;
        let mut sa = FluentArgs::new();
        sa.set("path", path.display().to_string());
        println!("✅ {}", tr_args("wizard-saved", &sa));
        return Ok(());
    }
}

fn provider_default_model(provider: &str) -> &'static str {
    match provider {
        "deepseek" => "deepseek-chat",
        "anthropic" => "claude-sonnet-4-20250514",
        "google" => "gemini-2.5-pro",
        "openai" => "gpt-4.1",
        "qwen" => "qwen3.5-plus",
        "moonshot" | "kimi_code" => "kimi-k2-0711-preview",
        "groq" => "llama-3.3-70b-versatile",
        _ => "gpt-4o",
    }
}

async fn run_routing_agents_flow(
    config_file: Option<PathBuf>,
    path: &std::path::Path,
    is_tty: bool,
) -> anyhow::Result<()> {
    let mut existing = load_anycode_config_resolved(config_file.clone())?.ok_or_else(|| {
        let mut a = FluentArgs::new();
        a.set("path", path.display().to_string());
        anyhow::anyhow!("{}", tr_args("wizard-no-config", &a))
    })?;

    accent_title(&tr("model-routing-title"));
    let preset_labels: Vec<String> = ROUTING_AGENT_PRESETS
        .iter()
        .map(|(id, desc)| format!("{} — {}", id, desc))
        .chain(std::iter::once(tr("model-custom-agent")))
        .chain(std::iter::once(tr("model-back-menu")))
        .collect();

    let pi = if is_tty {
        Select::with_theme(&ColorfulTheme::default())
            .with_prompt(&tr("model-pick-agent-type"))
            .default(0)
            .items(&preset_labels)
            .interact()?
    } else {
        for (i, l) in preset_labels.iter().enumerate() {
            println!("  {}) {}", i + 1, l);
        }
        println!("  0) {}", tr("model-back"));
        loop {
            let v = prompt_line(&tr("model-enter-number"))?;
            if v.trim() == "0" {
                return Ok(());
            }
            if let Ok(n) = v.trim().parse::<usize>() {
                if n >= 1 && n <= preset_labels.len() {
                    break n - 1;
                }
            }
            println!("{}", tr("model-invalid"));
        }
    };

    if pi >= preset_labels.len() - 1 {
        return Ok(());
    }

    let agent_key = if pi < ROUTING_AGENT_PRESETS.len() {
        ROUTING_AGENT_PRESETS[pi].0.to_string()
    } else if is_tty {
        Input::with_theme(&ColorfulTheme::default())
            .with_prompt("agent_type")
            .interact_text()?
    } else {
        prompt_line("agent_type: ")?
    };

    let mut ek = FluentArgs::new();
    ek.set("key", agent_key.clone());
    println!("{}", tr_args("model-edit-routing", &ek));

    let cur = existing
        .routing
        .agents
        .get(&agent_key)
        .cloned()
        .unwrap_or_default();

    let def_p = existing.provider.clone();
    let mut kp = FluentArgs::new();
    kp.set("p", def_p.clone());
    println!("{}", tr_args("model-keep-global", &kp));

    let routable: Vec<&ProviderCatalogEntry> = PROVIDER_CATALOG
        .iter()
        .filter(|e| !e.placeholder_only)
        .collect();
    let mut prov_labels: Vec<String> = routable
        .iter()
        .map(|e| format!("{} — {}", e.id, e.label))
        .collect();
    prov_labels.push("(inherit global provider)".to_string());
    prov_labels.push("(type custom provider id)".to_string());

    let p_idx = if is_tty {
        let def_i = cur
            .provider
            .as_deref()
            .map(normalize_provider_id)
            .and_then(|id| routable.iter().position(|e| e.id == id))
            .unwrap_or(prov_labels.len() - 2);
        Select::with_theme(&ColorfulTheme::default())
            .with_prompt("provider for this agent (OpenClaw catalog)")
            .default(def_i.min(prov_labels.len().saturating_sub(1)))
            .items(&prov_labels)
            .interact()?
    } else {
        for (i, l) in prov_labels.iter().enumerate() {
            println!("  {}) {}", i + 1, l);
        }
        loop {
            let v = prompt_line(&tr("model-pick-number"))?;
            if let Ok(n) = v.trim().parse::<usize>() {
                if n >= 1 && n <= prov_labels.len() {
                    break n - 1;
                }
            }
            println!("{}", tr("model-invalid"));
        }
    };

    let provider = if p_idx < routable.len() {
        Some(routable[p_idx].id.to_string())
    } else if p_idx == routable.len() {
        None
    } else {
        let raw = if is_tty {
            Input::with_theme(&ColorfulTheme::default())
                .with_prompt("provider id")
                .interact_text()?
        } else {
            prompt_line("provider id: ")?
        };
        let t = raw.trim();
        if t.is_empty() {
            None
        } else {
            let n = normalize_provider_id(t);
            validate_llm_provider(&n)?;
            Some(n)
        }
    };

    let model = if provider.as_deref() == Some("z.ai") || provider.as_deref() == Some("google") {
        let synthetic = Some(existing.clone());
        let m = if provider.as_deref() == Some("z.ai") {
            prompt_model_for_zai(is_tty, &synthetic)?
        } else {
            prompt_model_for_google(is_tty, &synthetic)?
        };
        Some(m)
    } else if provider.is_some() {
        let def_m = cur.model.clone().unwrap_or_else(|| {
            provider_default_model(provider.as_deref().unwrap_or("openai")).to_string()
        });
        let model_in = if is_tty {
            Input::with_theme(&ColorfulTheme::default())
                .with_prompt("model")
                .default(def_m.clone())
                .allow_empty(true)
                .interact_text()?
        } else {
            prompt_line(&tr("model-prompt-model-skip"))?
        };
        if model_in.trim().is_empty() {
            None
        } else {
            Some(model_in.trim().to_string())
        }
    } else {
        let model_in = if is_tty {
            Input::with_theme(&ColorfulTheme::default())
                .with_prompt("model")
                .default(cur.model.clone().unwrap_or_default())
                .allow_empty(true)
                .interact_text()?
        } else {
            prompt_line(&tr("model-prompt-model-skip"))?
        };
        if model_in.trim().is_empty() {
            None
        } else {
            Some(model_in.trim().to_string())
        }
    };

    let plan = if provider.as_deref() == Some("z.ai") {
        let plan_labels: Vec<String> = ZAI_AUTH_METHODS
            .iter()
            .map(|z| {
                z.hint
                    .map(|h| format!("{} — {} ({})", z.label, h, z.plan))
                    .unwrap_or_else(|| format!("{} ({})", z.label, z.plan))
            })
            .collect();
        let zi = if is_tty {
            Select::with_theme(&ColorfulTheme::default())
                .with_prompt("z.ai endpoint / plan")
                .default(0)
                .items(&plan_labels)
                .interact()?
        } else {
            for (i, l) in plan_labels.iter().enumerate() {
                println!("  {}) {}", i + 1, l);
            }
            loop {
                let v = prompt_line(&tr("model-pick-number"))?;
                if let Ok(n) = v.trim().parse::<usize>() {
                    if n >= 1 && n <= plan_labels.len() {
                        break n - 1;
                    }
                }
                println!("{}", tr("model-invalid"));
            }
        };
        Some(ZAI_AUTH_METHODS[zi].plan.to_string())
    } else {
        let plan_in = if is_tty {
            Input::with_theme(&ColorfulTheme::default())
                .with_prompt("plan（optional; z.ai: coding|general|coding_cn|general_cn）")
                .default(cur.plan.clone().unwrap_or_default())
                .allow_empty(true)
                .interact_text()?
        } else {
            prompt_line(&tr("model-prompt-plan-skip"))?
        };
        if plan_in.trim().is_empty() {
            None
        } else {
            Some(plan_in.trim().to_string())
        }
    };

    let ak_in = if is_tty {
        Input::with_theme(&ColorfulTheme::default())
            .with_prompt(&tr("model-prompt-api-key-profile"))
            .default(cur.api_key.clone().unwrap_or_default())
            .allow_empty(true)
            .interact_text()?
    } else {
        prompt_line(&tr("model-prompt-api-key-skip"))?
    };
    let api_key = if ak_in.trim().is_empty() {
        None
    } else {
        Some(ak_in.trim().to_string())
    };

    let bu_in = if is_tty {
        Input::with_theme(&ColorfulTheme::default())
            .with_prompt("base_url")
            .default(cur.base_url.clone().unwrap_or_default())
            .allow_empty(true)
            .interact_text()?
    } else {
        prompt_line(&tr("model-prompt-base-url-skip"))?
    };
    let base_url = if bu_in.trim().is_empty() {
        None
    } else {
        Some(bu_in.trim().to_string())
    };

    let profile = ModelProfile {
        provider,
        api_key,
        plan,
        model,
        temperature: cur.temperature,
        max_tokens: cur.max_tokens,
        base_url,
    };

    let empty_profile = profile.provider.is_none()
        && profile.api_key.is_none()
        && profile.plan.is_none()
        && profile.model.is_none()
        && profile.base_url.is_none()
        && profile.temperature.is_none()
        && profile.max_tokens.is_none();

    if empty_profile {
        existing.routing.agents.remove(&agent_key);
    } else {
        existing.routing.agents.insert(agent_key, profile);
    }

    save_anycode_config_resolved(config_file, &existing)?;
    let mut ru = FluentArgs::new();
    ru.set("path", path.display().to_string());
    println!("{}", tr_args("model-routing-updated", &ru));
    Ok(())
}
