//! Model wizard, `save_merged_config`, and `anycode model` subcommands.

use super::{
    is_anthropic_family_provider, is_known_zai_model, is_zai_family_provider,
    load_anycode_config_resolved, resolve_config_path, save_anycode_config_resolved,
    validate_llm_provider, AnyCodeConfig,
};
use crate::cli_args::{ModelAuthCommands, ModelCommands};
use crate::copilot_auth;
use crate::i18n::{tr, tr_args};
use anycode_llm::{
    normalize_provider_id, transport_for_provider_id, LlmTransport, ZAI_MODEL_CATALOG,
};
use fluent_bundle::FluentArgs;
use serde::Serialize;
use std::path::PathBuf;

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
