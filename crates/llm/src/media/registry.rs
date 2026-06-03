//! Resolve media profiles from unified model registry + legacy models.* overrides.

use crate::capability_catalog::ModelCapability;
use crate::config_models::EndpointOverrides;
use crate::model_registry::ResolvedModelRegistry;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct MediaProfile {
    pub capability: ModelCapability,
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub extra_headers: Option<std::collections::HashMap<String, String>>,
    pub endpoint_overrides: Option<EndpointOverrides>,
}

#[derive(Debug, Clone)]
pub struct ResolvedMediaProfile {
    pub profile: MediaProfile,
    pub model_id: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct MediaClientRegistry {
    pub stt: Option<ResolvedMediaProfile>,
    pub tts: Option<ResolvedMediaProfile>,
    pub embedding: Option<ResolvedMediaProfile>,
    pub image: Option<ResolvedMediaProfile>,
    pub video: Option<ResolvedMediaProfile>,
}

impl MediaClientRegistry {
    pub fn from_config(cfg: &Value) -> Self {
        Self::from_registry(&ResolvedModelRegistry::from_config(cfg))
    }

    pub fn from_registry(registry: &ResolvedModelRegistry) -> Self {
        let resolve = |cap: ModelCapability| -> Option<ResolvedMediaProfile> {
            let item = registry.active_item(cap)?;
            if !item.enabled {
                return None;
            }
            let api_key = registry.resolve_api_key(item)?;
            if api_key.is_empty() {
                return None;
            }
            let provider = registry.resolve_provider(item);
            let model = registry.resolve_model(item);
            if provider.is_empty() || model.is_empty() {
                return None;
            }
            Some(ResolvedMediaProfile {
                profile: MediaProfile {
                    capability: cap,
                    provider,
                    model,
                    api_key,
                    base_url: registry.resolve_base_url(item),
                    extra_headers: item.extra_headers.clone(),
                    endpoint_overrides: item.endpoint_overrides.clone(),
                },
                model_id: Some(item.id.clone()),
            })
        };

        Self {
            stt: resolve(ModelCapability::Stt),
            tts: resolve(ModelCapability::Tts),
            embedding: resolve(ModelCapability::Embedding),
            image: resolve(ModelCapability::ImageGen),
            video: resolve(ModelCapability::VideoGen),
        }
    }

    pub fn profile_for(&self, cap: ModelCapability) -> Option<&ResolvedMediaProfile> {
        match cap {
            ModelCapability::Stt => self.stt.as_ref(),
            ModelCapability::Tts => self.tts.as_ref(),
            ModelCapability::Embedding => self.embedding.as_ref(),
            ModelCapability::ImageGen => self.image.as_ref(),
            ModelCapability::VideoGen => self.video.as_ref(),
            ModelCapability::Chat | ModelCapability::Vision | ModelCapability::Rerank => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn resolves_stt_from_registry() {
        let cfg = json!({
            "provider": "openai",
            "api_key": "sk-test",
            "models": {
                "active": { "stt": "openai-whisper-1" },
                "items": [{
                    "id": "openai-whisper-1",
                    "provider": "openai",
                    "model": "whisper-1",
                    "capabilities": ["stt"],
                    "enabled": true
                }]
            }
        });
        let reg = MediaClientRegistry::from_config(&cfg);
        let stt = reg.stt.expect("stt");
        assert_eq!(stt.profile.model, "whisper-1");
    }

    #[test]
    fn migrates_legacy_speech() {
        let cfg = json!({
            "provider": "openai",
            "api_key": "sk-test",
            "models": {
                "speech": { "stt": { "model": "whisper-1" } }
            }
        });
        let reg = MediaClientRegistry::from_config(&cfg);
        assert!(reg.stt.is_some());
    }
}
