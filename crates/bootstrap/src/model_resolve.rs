//! Unified [`ModelProfile`] → [`ModelConfig`] resolution for routing and failover.

use anycode_config::{default_base_url_for, Config, ModelProfile};
use anycode_core::prelude::*;
use anycode_llm::{apply_anycode_cloud_model_config, normalize_provider_id};

use super::llm_session::{effective_provider, resolve_agent_base_url, resolve_profile_api_key};

pub fn default_base_url_for_config(config: &Config) -> Option<String> {
    let g_norm = normalize_provider_id(&config.llm.provider);
    if g_norm == "z.ai" {
        config
            .llm
            .base_url
            .clone()
            .or_else(|| Some(default_base_url_for(config.llm.plan.as_str()).to_string()))
    } else {
        config.llm.base_url.clone()
    }
}

pub fn resolve_model_profile(
    config: &Config,
    profile: &ModelProfile,
) -> anyhow::Result<ModelConfig> {
    let default_base_url = default_base_url_for_config(config);
    let eff_p = effective_provider(&config.llm.provider, Some(profile));
    let resolved_model = profile
        .model
        .clone()
        .unwrap_or_else(|| config.llm.model.clone());
    let resolved_temperature = profile.temperature.or(Some(config.llm.temperature));
    let resolved_max_tokens = profile.max_tokens.or(Some(config.llm.max_tokens));
    let resolved_base_url = resolve_agent_base_url(config, profile, &default_base_url);
    let api_key = resolve_profile_api_key(config, profile, &eff_p);
    apply_anycode_cloud_model_config(ModelConfig {
        provider: LLMProvider::Custom(eff_p),
        model: resolved_model,
        base_url: resolved_base_url,
        temperature: resolved_temperature,
        max_tokens: resolved_max_tokens,
        api_key,
        prompt_cache: config.llm.prompt_cache,
        thinking_enabled: config.llm.thinking_enabled,
        reasoning_effort: config.llm.reasoning_effort.clone(),
        ..Default::default()
    })
    .map_err(|e| anyhow::anyhow!(e.to_string()))
}
