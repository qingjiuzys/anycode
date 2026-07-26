//! Shared composition root: wires LLM, tools, security, and [`AgentRuntime`].

mod agents;
mod browser_mcp;
mod hosts;
mod llm_session;
mod llm_stack;
mod mcp_env;
mod memory_setup;
mod model_resolve;
mod prompt_runtime;
mod runtime;
mod runtimes;
mod security_setup;
mod skills_registry;
mod tools_setup;
mod workbench;

pub use memory_setup::{
    build_memory_layer, effective_memory_backend, memory_sled_path_for_diagnostics,
    MemoryAttachMode,
};
pub use prompt_runtime::{augment_prompt_runtime, build_runtime_prompt_config};
pub use runtime::{initialize_runtime, initialize_runtime_legacy, RuntimeHosts};
pub use runtimes::{detect_runtime_status, RuntimeStatus};
pub use workbench::project_skills::load_project_enabled_skills;
pub use workbench::workbench_ask::WorkbenchAskUserQuestionHost;

use anycode_config::Config;
use anycode_core::prelude::*;
use anycode_llm::{apply_anycode_cloud_model_config, ModelRouter};
use model_resolve::resolve_model_profile;
use std::collections::HashMap;

pub fn compile_tool_name_deny_regexes(patterns: &[String]) -> Vec<regex::Regex> {
    patterns
        .iter()
        .filter_map(|p| {
            let t = p.trim();
            if t.is_empty() {
                return None;
            }
            match regex::Regex::new(t) {
                Ok(re) => Some(re),
                Err(e) => {
                    tracing::warn!(
                        target: "anycode_bootstrap",
                        pattern = %t,
                        error = %e,
                        "ignoring invalid mcp_tool_deny_patterns entry"
                    );
                    None
                }
            }
        })
        .collect()
}

pub fn build_model_routing_parts(
    config: &Config,
) -> anyhow::Result<(ModelConfig, HashMap<AgentType, ModelConfig>)> {
    let default_base_url = model_resolve::default_base_url_for_config(config);

    let default_model_config = apply_anycode_cloud_model_config(ModelConfig {
        provider: LLMProvider::Custom(config.llm.provider.clone()),
        model: config.llm.model.clone(),
        base_url: default_base_url.clone(),
        temperature: Some(config.llm.temperature),
        max_tokens: Some(config.llm.max_tokens),
        api_key: None,
        ..Default::default()
    })
    .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    let mut model_overrides: HashMap<AgentType, ModelConfig> = HashMap::new();
    for (agent_type, profile) in config.routing.agents.iter() {
        model_overrides.insert(
            AgentType::new(agent_type.clone()),
            resolve_model_profile(config, profile)?,
        );
    }

    Ok((default_model_config, model_overrides))
}

pub fn build_failover_policy(config: &Config) -> Option<anycode_agent::FailoverPolicy> {
    let fb = config.runtime.model_fallback.as_ref()?;
    let provider = fb.provider.as_deref()?.trim();
    let model = fb.model.as_deref()?.trim();
    if provider.is_empty() || model.is_empty() {
        return None;
    }
    let profile = anycode_config::ModelProfile {
        provider: Some(provider.to_string()),
        model: Some(model.to_string()),
        ..Default::default()
    };
    Some(anycode_agent::FailoverPolicy {
        fallback: resolve_model_profile(config, &profile).ok()?,
        trigger: fb.on,
    })
}

pub fn build_preview_model_router(config: &Config) -> ModelRouter {
    let (default_model_config, model_overrides) =
        build_model_routing_parts(config).unwrap_or_else(|e| {
            tracing::warn!(error = %e, "model routing fallback to raw config");
            let default_base_url = model_resolve::default_base_url_for_config(config);
            (
                ModelConfig {
                    provider: LLMProvider::Custom(config.llm.provider.clone()),
                    model: config.llm.model.clone(),
                    base_url: default_base_url,
                    temperature: Some(config.llm.temperature),
                    max_tokens: Some(config.llm.max_tokens),
                    api_key: None,
                    ..Default::default()
                },
                HashMap::new(),
            )
        });
    ModelRouter::new(
        default_model_config,
        model_overrides,
        config.runtime.model_routes.clone(),
    )
}
