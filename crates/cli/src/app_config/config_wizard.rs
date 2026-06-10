//! `anycode config` wizard and runtime feature toggles.

use super::prompts::prompt_line;
use super::*;
use crate::i18n::{tr, tr_args};
use anycode_core::{FeatureFlag, RuntimeMode};
use fluent_bundle::FluentArgs;
use std::path::PathBuf;

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

pub(crate) async fn run_config_wizard() -> anyhow::Result<()> {
    run_config_wizard_inner(true).await
}

pub(crate) async fn run_config_wizard_inner(offer_wechat_after: bool) -> anyhow::Result<()> {
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

    let mut cfg = AnyCodeConfig {
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
        agents: existing
            .as_ref()
            .map(|c| c.agents.clone())
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
        mcp: existing.as_ref().map(|c| c.mcp.clone()).unwrap_or_default(),
        notifications: existing
            .as_ref()
            .map(|c| c.notifications.clone())
            .unwrap_or_default(),
        models: existing
            .as_ref()
            .map(|c| c.models.clone())
            .unwrap_or_default(),
    };

    if let Err(e) = crate::setup_memory::apply_to_in_memory_wizard_config(&mut cfg) {
        tracing::warn!(target: "anycode_cli", "memory setup wizard step skipped: {}", e);
    }

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
        crate::channels::wechat::run_onboard(None, None, false).await?;
    }
    Ok(())
}
