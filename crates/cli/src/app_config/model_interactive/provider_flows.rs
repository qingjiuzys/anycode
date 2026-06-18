//! Global provider catalog + quick auth-choice flows.

use super::accent_title;
use crate::app_config::model_wizard::save_merged_config;
use crate::app_config::prompts::{
    prompt_api_key_and_base_url, prompt_line, prompt_model_for_anthropic, prompt_model_for_google,
    prompt_model_for_zai,
};
use crate::app_config::{default_base_url_for, load_anycode_config_resolved, AnyCodeConfig};
use crate::i18n::{tr, tr_args};
use anycode_llm::{
    normalize_provider_id, transport_for_provider_id, LlmTransport, PROVIDER_CATALOG,
    ZAI_AUTH_METHODS,
};
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Input, Password, Select};
use fluent_bundle::FluentArgs;
use std::path::PathBuf;

pub(super) use anycode_setup::{QuickAuthChoice as AuthChoice, QUICK_AUTH_CHOICES};

/// [`run_global_provider_flow`] 结束方式：setup 下从提供方列表返回不可结束整个 `run_onboard`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GlobalProviderFlowResult {
    /// 已保存配置，或 `anycode model` 全局流中从提供方列表返回主菜单。
    Finished,
    /// 仅 `anycode setup` 精简向导：用户在全量提供方列表中选 Back，应回到「快速/完整」模式选择。
    SetupBackToModePicker,
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

pub(super) async fn apply_quick_auth_choice(
    config_file: Option<PathBuf>,
    path: &std::path::Path,
    is_tty: bool,
    existing: &Option<AnyCodeConfig>,
    choice: AuthChoice,
) -> anyhow::Result<()> {
    if choice.device_auth {
        println!("已选择：{} ({})", choice.label, choice.id);
        println!("请运行 `anycode auth login` 并在浏览器完成设备关联。");
        if crate::commands::cloud_auth::read_cloud_session().is_none() {
            crate::commands::cloud_auth::run_auth_login().await?;
        }
        save_merged_config(
            config_file,
            existing,
            choice.provider,
            choice.plan,
            choice.default_model,
            "",
            None,
        )?;
        let mut sa = FluentArgs::new();
        sa.set("path", path.display().to_string());
        println!("✅ {}", tr_args("wizard-saved", &sa));
        return Ok(());
    }

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

pub(super) async fn run_global_provider_flow(
    config_file: Option<PathBuf>,
    path: &std::path::Path,
    is_tty: bool,
    quick_mode: bool,
    setup_onboard: bool,
) -> anyhow::Result<GlobalProviderFlowResult> {
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
                .with_prompt(tr("model-pick-provider"))
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
                    return Ok(if setup_onboard {
                        GlobalProviderFlowResult::SetupBackToModePicker
                    } else {
                        GlobalProviderFlowResult::Finished
                    });
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
            return Ok(if setup_onboard {
                GlobalProviderFlowResult::SetupBackToModePicker
            } else {
                GlobalProviderFlowResult::Finished
            });
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
                    .with_prompt(tr("model-pick-prompt"))
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
            return Ok(GlobalProviderFlowResult::Finished);
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
            return Ok(GlobalProviderFlowResult::Finished);
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
            return Ok(GlobalProviderFlowResult::Finished);
        }

        if transport_for_provider_id(&id) == LlmTransport::BedrockConverse {
            let model: String = if is_tty {
                Input::with_theme(&ColorfulTheme::default())
                    .with_prompt(tr("wizard-prompt-model-id"))
                    .interact_text()?
            } else {
                prompt_line(&format!("{} ", tr("wizard-prompt-model-id")))?
            };
            let base_url: Option<String> = if is_tty {
                let raw: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt(tr("wizard-bedrock-endpoint-prompt"))
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
            return Ok(GlobalProviderFlowResult::Finished);
        }

        if id == "anycode_cloud" {
            println!("anyCode Cloud 使用设备登录，无需 API Key。");
            if crate::commands::cloud_auth::read_cloud_session().is_none() {
                crate::commands::cloud_auth::run_auth_login().await?;
            }
            let model = existing
                .as_ref()
                .filter(|c| normalize_provider_id(&c.provider) == "anycode_cloud")
                .map(|c| c.model.clone())
                .filter(|m| !m.trim().is_empty())
                .unwrap_or_else(|| "agnes-chat".to_string());
            save_merged_config(
                config_file.clone(),
                &existing,
                "anycode_cloud",
                "cloud",
                &model,
                "",
                None,
            )?;
            let mut sa = FluentArgs::new();
            sa.set("path", path.display().to_string());
            println!("✅ {}", tr_args("wizard-saved", &sa));
            return Ok(GlobalProviderFlowResult::Finished);
        }

        if transport_for_provider_id(&id) == LlmTransport::GithubCopilot {
            let model: String = if is_tty {
                Input::with_theme(&ColorfulTheme::default())
                    .with_prompt(tr("wizard-copilot-model-prompt"))
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
            return Ok(GlobalProviderFlowResult::Finished);
        }

        let default_url = entry.suggested_openai_base.unwrap_or("");
        let default_model = existing
            .as_ref()
            .filter(|c| normalize_provider_id(&c.provider) == id)
            .map(|c| c.model.clone())
            .unwrap_or_else(|| provider_default_model(&id).to_string());
        let model: String = if is_tty {
            Input::with_theme(&ColorfulTheme::default())
                .with_prompt(tr("wizard-prompt-model-id"))
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
        return Ok(GlobalProviderFlowResult::Finished);
    }
}

pub(super) fn provider_default_model(provider: &str) -> &'static str {
    match provider {
        "anycode_cloud" => "agnes-chat",
        "deepseek" => "deepseek-v4-pro",
        "anthropic" => "claude-sonnet-4-20250514",
        "google" => "gemini-2.5-pro",
        "openai" => "gpt-4.1",
        "qwen" => "qwen3.5-plus",
        "moonshot" | "kimi_code" => "kimi-k2-0711-preview",
        "groq" => "llama-3.3-70b-versatile",
        _ => "gpt-4o",
    }
}
