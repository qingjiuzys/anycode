//! `anycode status` — config, routing, and OpenClaw-style model resolution summary.

use crate::builtin_agents::BUILTIN_AGENT_IDS;
use crate::{bootstrap, slash_commands, workspace};
use anycode_core::{LLMProvider, ModelConfig, RuntimeMode};
use anycode_llm::{
    normalize_provider_id, resolve_chat_model_ref, zai_model_catalog_entries,
    ChatModelResolutionSource,
};
use anycode_tools::iter_cli_tool_help;

fn provider_label(p: &LLMProvider) -> String {
    match p {
        LLMProvider::Anthropic => "anthropic".to_string(),
        LLMProvider::OpenAI => "openai".to_string(),
        LLMProvider::Local => "local".to_string(),
        LLMProvider::Custom(s) => s.clone(),
    }
}

fn format_model_line(cfg: &ModelConfig) -> String {
    format!("{} / {}", provider_label(&cfg.provider), cfg.model)
}

fn primary_chat_resolution(config: &crate::app_config::Config) -> anycode_llm::ChatModelResolution {
    let norm = normalize_provider_id(&config.llm.provider);
    let cat = zai_model_catalog_entries();
    if norm == "z.ai" {
        resolve_chat_model_ref(&config.llm.model, Some(&config.llm.provider), &cat)
    } else {
        resolve_chat_model_ref(&config.llm.model, Some(&config.llm.provider), &[])
    }
}

fn all_modes() -> [RuntimeMode; 6] {
    [
        RuntimeMode::General,
        RuntimeMode::Explore,
        RuntimeMode::Plan,
        RuntimeMode::Code,
        RuntimeMode::Channel,
        RuntimeMode::Goal,
    ]
}

pub(crate) fn print_status(config: &crate::app_config::Config, json: bool) -> anyhow::Result<()> {
    let router = bootstrap::build_preview_model_router(config);
    let primary = primary_chat_resolution(config);

    let mode_lines: Vec<(String, String)> = all_modes()
        .iter()
        .map(|m| {
            (
                m.as_str().to_string(),
                format_model_line(&router.resolve_for_mode(m)),
            )
        })
        .collect();

    let mode_aliases: serde_json::Value = serde_json::to_value(
        &config
            .runtime
            .model_routes
            .mode_aliases
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<std::collections::BTreeMap<_, _>>(),
    )?;

    let agent_aliases: serde_json::Value = serde_json::to_value(
        &config
            .runtime
            .model_routes
            .agent_aliases
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<std::collections::BTreeMap<_, _>>(),
    )?;

    let mode_resolved: serde_json::Map<String, serde_json::Value> = mode_lines
        .iter()
        .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
        .collect();

    if json {
        println!(
            "{}",
            serde_json::json!({
                "provider": config.llm.provider,
                "model": config.llm.model,
                "primary_chat_ref": primary.value,
                "primary_resolution_source": format!("{:?}", primary.source),
                "default_mode": config.runtime.default_mode.as_str(),
                "features": config.runtime.features.enabled(),
                "routing_agents": config.routing.agents.keys().collect::<Vec<_>>(),
                "model_routes_default_alias": config.runtime.model_routes.default_alias,
                "model_routes_mode_aliases": mode_aliases,
                "model_routes_agent_aliases": agent_aliases,
                "model_per_runtime_mode": mode_resolved,
                "agents": BUILTIN_AGENT_IDS,
                "tools": iter_cli_tool_help().map(|(n, _)| n).collect::<Vec<_>>(),
                "workspace_label": config.runtime.workspace_project_label,
                "workspace_channel_profile": config.runtime.workspace_channel_profile,
            })
        );
        return Ok(());
    }

    println!("provider: {}", config.llm.provider);
    println!("model: {}", config.llm.model);
    println!("primary_chat_ref: {} ({:?})", primary.value, primary.source);
    if primary.source == ChatModelResolutionSource::Raw {
        if let Some(r) = primary.reason {
            println!("primary_resolution_note: {:?}", r);
        }
    }
    println!("default_mode: {}", config.runtime.default_mode.as_str());
    if let Some(ref d) = config.runtime.model_routes.default_alias {
        println!("model_routes.default_alias: {}", d);
    }
    if !config.runtime.model_routes.mode_aliases.is_empty() {
        println!("model_routes.mode_aliases:");
        let mut keys: Vec<_> = config.runtime.model_routes.mode_aliases.keys().collect();
        keys.sort();
        for k in keys {
            if let Some(v) = config.runtime.model_routes.mode_aliases.get(k) {
                println!("  {}: {}", k, v);
            }
        }
    }
    if !config.runtime.model_routes.agent_aliases.is_empty() {
        println!("model_routes.agent_aliases:");
        let mut keys: Vec<_> = config.runtime.model_routes.agent_aliases.keys().collect();
        keys.sort();
        for k in keys {
            if let Some(v) = config.runtime.model_routes.agent_aliases.get(k) {
                println!("  {}: {}", k, v);
            }
        }
    }
    println!("model_per_runtime_mode:");
    for (mode, line) in &mode_lines {
        println!("  {}: {}", mode, line);
    }
    if let Some(ref l) = config.runtime.workspace_project_label {
        println!("workspace_label: {}", l);
    }
    if let Some(ref c) = config.runtime.workspace_channel_profile {
        println!("workspace_channel_profile: {}", c);
    }
    println!("features: {}", config.runtime.features.enabled().join(", "));
    println!("{}", workspace::current_workspace_status(5));
    println!(
        "slash_commands: {}",
        slash_commands::help_lines().join(" | ")
    );
    println!("agents: {}", BUILTIN_AGENT_IDS.join(", "));
    println!(
        "tools: {}",
        iter_cli_tool_help()
            .map(|(name, _)| name)
            .collect::<Vec<_>>()
            .join(", ")
    );

    Ok(())
}
