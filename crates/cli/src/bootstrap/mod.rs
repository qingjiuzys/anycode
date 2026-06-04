//! Wire LLM client, tool registry, and `AgentRuntime` (shared by TUI, `run`, and long-lived bridges).

mod agents;
mod llm_session;
mod llm_stack;
mod mcp_env;
mod memory_setup;
mod model_resolve;
mod prompt_runtime;
mod runtime;
mod security_setup;
mod skills_registry;
mod tools_setup;

pub(crate) use memory_setup::{
    build_memory_layer, effective_memory_backend, memory_sled_path_for_diagnostics,
    MemoryAttachMode,
};
pub(crate) use runtime::initialize_runtime;

use crate::app_config::Config;
use anycode_core::prelude::*;
use anycode_llm::ModelRouter;
use model_resolve::resolve_model_profile;
use std::collections::HashMap;
use std::sync::Arc;

pub(crate) fn compile_tool_name_deny_regexes(patterns: &[String]) -> Vec<regex::Regex> {
    use crate::i18n::tr_args;
    use fluent_bundle::FluentArgs;

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
                    let mut a = FluentArgs::new();
                    a.set("pat", t.to_string());
                    a.set("err", e.to_string());
                    tracing::warn!(
                        target: "anycode_cli",
                        "{}",
                        tr_args("log-ignore-deny-pattern", &a)
                    );
                    None
                }
            }
        })
        .collect()
}

/// Default LLM config + per-agent overrides (before summary/workspace-assistant/goal fill-ins).
pub(crate) fn build_model_routing_parts(
    config: &Config,
) -> (ModelConfig, HashMap<AgentType, ModelConfig>) {
    let default_base_url = model_resolve::default_base_url_for_config(config);

    let default_model_config = ModelConfig {
        provider: LLMProvider::Custom(config.llm.provider.clone()),
        model: config.llm.model.clone(),
        base_url: default_base_url.clone(),
        temperature: Some(config.llm.temperature),
        max_tokens: Some(config.llm.max_tokens),
        api_key: None,
    };

    let mut model_overrides: HashMap<AgentType, ModelConfig> = HashMap::new();
    for (agent_type, profile) in config.routing.agents.iter() {
        model_overrides.insert(
            AgentType::new(agent_type.clone()),
            resolve_model_profile(config, profile),
        );
    }

    (default_model_config, model_overrides)
}

pub(crate) fn build_failover_policy(config: &Config) -> Option<anycode_agent::FailoverPolicy> {
    let fb = config.runtime.model_fallback.as_ref()?;
    let provider = fb.provider.as_deref()?.trim();
    let model = fb.model.as_deref()?.trim();
    if provider.is_empty() || model.is_empty() {
        return None;
    }
    let profile = crate::app_config::ModelProfile {
        provider: Some(provider.to_string()),
        model: Some(model.to_string()),
        ..Default::default()
    };
    Some(anycode_agent::FailoverPolicy {
        fallback: resolve_model_profile(config, &profile),
        trigger: fb.on,
    })
}

/// Same routing snapshot as runtime (before optional agent fill-ins). For `status` / diagnostics.
pub(crate) fn build_preview_model_router(config: &Config) -> ModelRouter {
    let (default_model_config, model_overrides) = build_model_routing_parts(config);
    ModelRouter::new(
        default_model_config,
        model_overrides,
        config.runtime.model_routes.clone(),
    )
}
