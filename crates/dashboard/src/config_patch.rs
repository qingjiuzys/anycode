//! Safe partial updates to `~/.anycode/config.json` from the dashboard.

pub use anycode_llm::config_file::{
    migrate_legacy_llm_section, patch_llm_config as patch_llm_config_inner, read_config_value,
    read_model_fallback, string_field, write_config_value, LlmConfigPatch as LlmConfigPatchInner,
};
pub use anycode_llm::{ModelFallbackConfig, ModelProfileFile, ModelsConfigFile};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmConfigPatchBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_credentials: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_on: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_agents: Option<HashMap<String, ModelProfileFile>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_agents_delete: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<ModelsConfigFile>,
}

impl From<&LlmConfigPatchBody> for LlmConfigPatchInner {
    fn from(body: &LlmConfigPatchBody) -> Self {
        let fallback = if body.fallback_provider.is_some()
            || body.fallback_model.is_some()
            || body.fallback_on.is_some()
        {
            Some(ModelFallbackConfig {
                provider: body.fallback_provider.clone(),
                model: body.fallback_model.clone(),
                on: body
                    .fallback_on
                    .as_deref()
                    .and_then(anycode_llm::FailoverTrigger::from_str_label)
                    .unwrap_or_default(),
            })
        } else {
            None
        };
        LlmConfigPatchInner {
            provider: body.provider.clone(),
            model: body.model.clone(),
            plan: body.plan.clone(),
            base_url: body.base_url.clone(),
            api_key: body.api_key.clone(),
            provider_credentials: body.provider_credentials.clone(),
            fallback,
            routing_agents: body.routing_agents.clone(),
            routing_agents_delete: body.routing_agents_delete.clone(),
            models: body.models.clone(),
            models_replace: false,
        }
    }
}

// Backward-compatible alias used by existing handlers.
pub type LlmConfigPatch = LlmConfigPatchBody;

pub fn patch_llm_config(patch: &LlmConfigPatchBody) -> Result<(PathBuf, Value)> {
    patch_llm_config_inner(None, &patch.into())
}

pub fn read_config_root() -> Result<(PathBuf, Value)> {
    read_config_value(None)
}

pub fn write_config_root(cfg: &Value) -> Result<PathBuf> {
    let (path, _) = read_config_value(None)?;
    write_config_value(&path, cfg)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_writes_flat_provider() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", dir.path());
        let (_, cfg) = patch_llm_config(&LlmConfigPatchBody {
            provider: Some("anthropic".into()),
            model: Some("claude-sonnet".into()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(
            cfg.get("provider").and_then(|v| v.as_str()),
            Some("anthropic")
        );
        assert!(cfg.get("llm").is_none());
    }
}
