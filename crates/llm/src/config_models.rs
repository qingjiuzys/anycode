//! Shared `config.json` model profile shapes (flat AnyCodeConfig + models.*).

use crate::capability_catalog::ModelCapability;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// When to switch from primary chat model to fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FailoverTrigger {
    #[default]
    Geo,
    RateLimit,
    AnyError,
}

impl FailoverTrigger {
    pub fn from_str_label(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "geo" => Some(Self::Geo),
            "rate_limit" | "ratelimit" | "429" => Some(Self::RateLimit),
            "any_error" | "any" | "all" => Some(Self::AnyError),
            _ => None,
        }
    }
}

/// Chat model fallback stored under `runtime.model_fallback`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ModelFallbackConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default)]
    pub on: FailoverTrigger,
}

/// Per-capability model override (mirrors CLI `ModelProfile`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ModelProfileFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

/// Custom HTTP endpoints (e.g. video submit/status/result).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct EndpointOverrides {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub submit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
}

/// A user-configured model entry in the unified registry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfiguredModelFile {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub provider: String,
    pub model: String,
    #[serde(default)]
    pub capabilities: Vec<ModelCapability>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra_headers: Option<HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint_overrides: Option<EndpointOverrides>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Routing strategy for model selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RoutingStrategy {
    #[default]
    CapabilityFirst,
    AgentOverrides,
    FallbackChain,
}

/// Fallback chain entry under routing.fallbacks.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FallbackChainEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default)]
    pub on: FailoverTrigger,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SpeechModelsConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stt: Option<ModelProfileFile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tts: Option<ModelProfileFile>,
}

/// Multimodal model profiles under top-level `models`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ModelsConfigFile {
    /// capability id -> configured model id
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<ConfiguredModelFile>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat: Option<ModelProfileFile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding: Option<ModelProfileFile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speech: Option<SpeechModelsConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<ModelProfileFile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video: Option<ModelProfileFile>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RoutingAgentsFile {
    #[serde(default)]
    pub agents: HashMap<String, ModelProfileFile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strategy: Option<RoutingStrategy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modes: Option<HashMap<String, ModelProfileFile>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallbacks: Option<Vec<FallbackChainEntry>>,
}

/// Masked view for dashboard GET /llm.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MaskedSecret {
    pub configured: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
}

impl MaskedSecret {
    pub fn from_value(value: Option<&str>) -> Self {
        match value.filter(|s| !s.trim().is_empty()) {
            Some(s) => Self {
                configured: true,
                preview: Some(format!("{}…", s.chars().take(4).collect::<String>())),
            },
            None => Self {
                configured: false,
                preview: None,
            },
        }
    }
}
