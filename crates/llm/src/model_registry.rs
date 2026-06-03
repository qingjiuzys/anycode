//! Unified model registry: configured models, active capability map, legacy migration.

use crate::capability_catalog::ModelCapability;
use crate::config_file::string_field;
use crate::config_models::{
    ConfiguredModelFile, ModelProfileFile, ModelsConfigFile, SpeechModelsConfig,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// Resolved registry used at runtime and by media clients.
#[derive(Debug, Clone, Default)]
pub struct ResolvedModelRegistry {
    pub items: Vec<ConfiguredModelFile>,
    pub active: HashMap<ModelCapability, String>,
    pub provider_credentials: HashMap<String, String>,
    pub global_provider: Option<String>,
    pub global_model: Option<String>,
    pub global_api_key: String,
    pub global_base_url: Option<String>,
    pub global_plan: Option<String>,
}

impl ResolvedModelRegistry {
    pub fn from_config(cfg: &Value) -> Self {
        let creds: HashMap<String, String> = cfg
            .get("provider_credentials")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        let mut reg = build_registry_from_config(cfg);
        reg.provider_credentials = creds;
        reg.global_provider = string_field(cfg, "provider", "provider");
        reg.global_model = string_field(cfg, "model", "model");
        reg.global_api_key = string_field(cfg, "api_key", "api_key").unwrap_or_default();
        reg.global_base_url = string_field(cfg, "base_url", "base_url");
        reg.global_plan = string_field(cfg, "plan", "plan");
        reg
    }

    pub fn item_by_id(&self, id: &str) -> Option<&ConfiguredModelFile> {
        self.items.iter().find(|m| m.id == id)
    }

    pub fn active_item(&self, cap: ModelCapability) -> Option<&ConfiguredModelFile> {
        let id = self.active.get(&cap)?;
        self.item_by_id(id)
    }

    pub fn resolve_api_key(&self, item: &ConfiguredModelFile) -> Option<String> {
        if let Some(ref k) = item.api_key {
            if !k.trim().is_empty() {
                return Some(k.trim().to_string());
            }
        }
        if let Some(ref r) = item.api_key_ref {
            if let Some(k) = self.provider_credentials.get(r) {
                if !k.trim().is_empty() {
                    return Some(k.trim().to_string());
                }
            }
        }
        let prov = normalize_provider(&item.provider);
        if self
            .global_provider
            .as_deref()
            .map(normalize_provider)
            .as_deref()
            == Some(prov.as_str())
            && !self.global_api_key.trim().is_empty()
        {
            return Some(self.global_api_key.clone());
        }
        self.provider_credentials
            .get(prov.as_str())
            .or_else(|| self.provider_credentials.get(&item.provider))
            .cloned()
            .filter(|s| !s.trim().is_empty())
    }

    pub fn resolve_provider(&self, item: &ConfiguredModelFile) -> String {
        item.provider.trim().to_string()
    }

    pub fn resolve_model(&self, item: &ConfiguredModelFile) -> String {
        item.model.trim().to_string()
    }

    pub fn resolve_base_url(&self, item: &ConfiguredModelFile) -> Option<String> {
        item.base_url
            .as_deref()
            .or(self.global_base_url.as_deref())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    }

    pub fn resolve_plan(&self, item: &ConfiguredModelFile) -> Option<String> {
        item.plan
            .as_deref()
            .or(self.global_plan.as_deref())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    }

    /// Collect all chat-transport provider ids referenced by active chat/vision and routing.
    pub fn chat_providers(&self) -> HashSet<String> {
        let mut out = HashSet::new();
        if let Some(p) = self.global_provider.as_deref() {
            out.insert(normalize_provider(p));
        }
        for cap in [ModelCapability::Chat, ModelCapability::Vision] {
            if let Some(item) = self.active_item(cap) {
                out.insert(normalize_provider(&item.provider));
            }
        }
        out
    }
}

fn normalize_provider(p: &str) -> String {
    crate::normalize_provider_id(p)
}

fn slug_id(provider: &str, model: &str) -> String {
    let p = provider.trim().replace('.', "-").replace('_', "-");
    let m = model.trim().replace('/', "-").replace('.', "-");
    format!("{p}-{m}")
}

fn profile_to_item(
    id: String,
    _provider: &str,
    profile: &ModelProfileFile,
    capabilities: Vec<ModelCapability>,
    source: &str,
    fallback_provider: Option<&str>,
    fallback_model: Option<&str>,
) -> Option<ConfiguredModelFile> {
    let prov = profile
        .provider
        .as_deref()
        .or(fallback_provider)?
        .trim()
        .to_string();
    if prov.is_empty() {
        return None;
    }
    let model = profile
        .model
        .as_deref()
        .or(fallback_model)?
        .trim()
        .to_string();
    if model.is_empty() {
        return None;
    }
    Some(ConfiguredModelFile {
        id,
        display_name: None,
        provider: prov,
        model,
        capabilities,
        api_key: profile.api_key.clone(),
        api_key_ref: None,
        plan: profile.plan.clone(),
        base_url: profile.base_url.clone(),
        temperature: profile.temperature,
        max_tokens: profile.max_tokens,
        extra_headers: None,
        endpoint_overrides: None,
        enabled: true,
        tags: None,
        source: Some(source.to_string()),
    })
}

/// Build registry from config, migrating legacy flat + models.* when items absent.
pub fn build_registry_from_config(cfg: &Value) -> ResolvedModelRegistry {
    let models_raw = cfg.get("models");
    let mut items: Vec<ConfiguredModelFile> = models_raw
        .and_then(|m| m.get("items"))
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let mut active: HashMap<ModelCapability, String> = models_raw
        .and_then(|m| m.get("active"))
        .and_then(|v| serde_json::from_value::<HashMap<String, String>>(v.clone()).ok())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(k, v)| ModelCapability::parse(&k).map(|c| (c, v)))
        .collect();

    let global_provider = string_field(cfg, "provider", "provider");
    let global_model = string_field(cfg, "model", "model");

    let legacy: ModelsConfigFile = models_raw
        .and_then(|v| {
            let mut m: ModelsConfigFile = serde_json::from_value(v.clone()).ok()?;
            m.active = None;
            m.items = None;
            Some(m)
        })
        .unwrap_or_default();

    if items.is_empty() {
        if let (Some(ref gp), Some(ref gm)) = (&global_provider, &global_model) {
            let id = slug_id(gp, gm);
            items.push(ConfiguredModelFile {
                id: id.clone(),
                display_name: Some(format!("{gm} (chat)")),
                provider: gp.clone(),
                model: gm.clone(),
                capabilities: vec![ModelCapability::Chat],
                api_key: string_field(cfg, "api_key", "api_key"),
                api_key_ref: None,
                plan: string_field(cfg, "plan", "plan"),
                base_url: string_field(cfg, "base_url", "base_url"),
                temperature: cfg
                    .get("temperature")
                    .and_then(|v| v.as_f64())
                    .map(|f| f as f32),
                max_tokens: cfg
                    .get("max_tokens")
                    .and_then(|v| v.as_u64())
                    .map(|u| u as u32),
                extra_headers: None,
                endpoint_overrides: None,
                enabled: true,
                tags: None,
                source: Some("migrated_flat".into()),
            });
            active.entry(ModelCapability::Chat).or_insert(id);
        }

        if let Some(ref chat) = legacy.chat {
            if let Some(item) = profile_to_item(
                slug_id(
                    chat.provider
                        .as_deref()
                        .unwrap_or(global_provider.as_deref().unwrap_or("custom")),
                    chat.model
                        .as_deref()
                        .unwrap_or(global_model.as_deref().unwrap_or("model")),
                ),
                global_provider.as_deref().unwrap_or("custom"),
                chat,
                vec![ModelCapability::Chat],
                "migrated_models_chat",
                global_provider.as_deref(),
                global_model.as_deref(),
            ) {
                active
                    .entry(ModelCapability::Chat)
                    .or_insert(item.id.clone());
                if !items.iter().any(|i| i.id == item.id) {
                    items.push(item);
                }
            }
        }

        if let Some(ref emb) = legacy.embedding {
            if let Some(item) = profile_to_item(
                slug_id(
                    emb.provider.as_deref().unwrap_or("openai"),
                    emb.model.as_deref().unwrap_or("embedding"),
                ),
                "openai",
                emb,
                vec![ModelCapability::Embedding],
                "migrated_models_embedding",
                global_provider.as_deref(),
                None,
            ) {
                active
                    .entry(ModelCapability::Embedding)
                    .or_insert(item.id.clone());
                if !items.iter().any(|i| i.id == item.id) {
                    items.push(item);
                }
            }
        }

        if let Some(ref speech) = legacy.speech {
            if let Some(ref stt) = speech.stt {
                if let Some(item) = profile_to_item(
                    slug_id(
                        stt.provider.as_deref().unwrap_or("openai"),
                        stt.model.as_deref().unwrap_or("whisper-1"),
                    ),
                    "openai",
                    stt,
                    vec![ModelCapability::Stt],
                    "migrated_models_stt",
                    global_provider.as_deref(),
                    None,
                ) {
                    active
                        .entry(ModelCapability::Stt)
                        .or_insert(item.id.clone());
                    if !items.iter().any(|i| i.id == item.id) {
                        items.push(item);
                    }
                }
            }
            if let Some(ref tts) = speech.tts {
                if let Some(item) = profile_to_item(
                    slug_id(
                        tts.provider.as_deref().unwrap_or("openai"),
                        tts.model.as_deref().unwrap_or("tts-1"),
                    ),
                    "openai",
                    tts,
                    vec![ModelCapability::Tts],
                    "migrated_models_tts",
                    global_provider.as_deref(),
                    None,
                ) {
                    active
                        .entry(ModelCapability::Tts)
                        .or_insert(item.id.clone());
                    if !items.iter().any(|i| i.id == item.id) {
                        items.push(item);
                    }
                }
            }
        }

        if let Some(ref img) = legacy.image {
            if let Some(item) = profile_to_item(
                slug_id(
                    img.provider.as_deref().unwrap_or("openai"),
                    img.model.as_deref().unwrap_or("dall-e-3"),
                ),
                "openai",
                img,
                vec![ModelCapability::ImageGen],
                "migrated_models_image",
                global_provider.as_deref(),
                None,
            ) {
                active
                    .entry(ModelCapability::ImageGen)
                    .or_insert(item.id.clone());
                if !items.iter().any(|i| i.id == item.id) {
                    items.push(item);
                }
            }
        }

        if let Some(ref vid) = legacy.video {
            if let Some(item) = profile_to_item(
                slug_id(
                    vid.provider.as_deref().unwrap_or("custom"),
                    vid.model.as_deref().unwrap_or("video"),
                ),
                "custom",
                vid,
                vec![ModelCapability::VideoGen],
                "migrated_models_video",
                global_provider.as_deref(),
                None,
            ) {
                active
                    .entry(ModelCapability::VideoGen)
                    .or_insert(item.id.clone());
                if !items.iter().any(|i| i.id == item.id) {
                    items.push(item);
                }
            }
        }
    }

    if active.get(&ModelCapability::Chat).is_none() {
        if let Some(chat_item) = items
            .iter()
            .find(|i| i.enabled && i.capabilities.contains(&ModelCapability::Chat))
        {
            active.insert(ModelCapability::Chat, chat_item.id.clone());
        }
    }

    ResolvedModelRegistry {
        items,
        active,
        provider_credentials: HashMap::new(),
        global_provider,
        global_model,
        global_api_key: String::new(),
        global_base_url: None,
        global_plan: None,
    }
}

/// Sync registry active chat back to flat top-level fields for CLI backward compat.
pub fn sync_flat_chat_fields(cfg: &mut Value, registry: &ResolvedModelRegistry) {
    let Some(obj) = cfg.as_object_mut() else {
        return;
    };
    if let Some(item) = registry.active_item(ModelCapability::Chat) {
        obj.insert("provider".into(), Value::String(item.provider.clone()));
        obj.insert("model".into(), Value::String(item.model.clone()));
        if let Some(ref p) = item.plan {
            obj.insert("plan".into(), Value::String(p.clone()));
        }
        if let Some(ref u) = item.base_url {
            obj.insert("base_url".into(), Value::String(u.clone()));
        }
        if let Some(ref k) = item.api_key {
            if !k.trim().is_empty() {
                obj.insert("api_key".into(), Value::String(k.trim().to_string()));
            }
        }
        obj.remove("llm");
    }
}

/// Sync registry to legacy models.* profiles for tools still reading old shape.
pub fn sync_legacy_models_section(registry: &ResolvedModelRegistry) -> ModelsConfigFile {
    let mut out = ModelsConfigFile::default();
    out.active = Some(
        registry
            .active
            .iter()
            .map(|(c, id)| (c.as_str().to_string(), id.clone()))
            .collect(),
    );
    out.items = Some(registry.items.clone());

    fn to_profile(item: &ConfiguredModelFile, reg: &ResolvedModelRegistry) -> ModelProfileFile {
        ModelProfileFile {
            provider: Some(item.provider.clone()),
            model: Some(item.model.clone()),
            plan: item.plan.clone().or(reg.global_plan.clone()),
            api_key: reg.resolve_api_key(item),
            base_url: reg.resolve_base_url(item),
            temperature: item.temperature,
            max_tokens: item.max_tokens,
        }
    }

    if let Some(item) = registry.active_item(ModelCapability::Chat) {
        out.chat = Some(to_profile(item, registry));
    }
    if let Some(item) = registry.active_item(ModelCapability::Embedding) {
        out.embedding = Some(to_profile(item, registry));
    }
    if let Some(item) = registry.active_item(ModelCapability::Stt) {
        out.speech
            .get_or_insert_with(SpeechModelsConfig::default)
            .stt = Some(to_profile(item, registry));
    }
    if let Some(item) = registry.active_item(ModelCapability::Tts) {
        out.speech
            .get_or_insert_with(SpeechModelsConfig::default)
            .tts = Some(to_profile(item, registry));
    }
    if let Some(item) = registry.active_item(ModelCapability::ImageGen) {
        out.image = Some(to_profile(item, registry));
    }
    if let Some(item) = registry.active_item(ModelCapability::VideoGen) {
        out.video = Some(to_profile(item, registry));
    }
    out
}

pub fn upsert_registry_item(items: &mut Vec<ConfiguredModelFile>, item: ConfiguredModelFile) {
    if let Some(existing) = items.iter_mut().find(|i| i.id == item.id) {
        *existing = item;
    } else {
        items.push(item);
    }
}

pub fn remove_registry_item(items: &mut Vec<ConfiguredModelFile>, id: &str) {
    items.retain(|i| i.id != id);
}

pub fn set_active_capability(
    active: &mut HashMap<ModelCapability, String>,
    cap: ModelCapability,
    model_id: &str,
) {
    active.insert(cap, model_id.to_string());
}

/// Facade view of model registry + flat LLM fields for settings APIs.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RegistryView {
    pub config_present: bool,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub plan: Option<String>,
    pub base_url: Option<String>,
    pub api_key: crate::config_models::MaskedSecret,
    pub provider_credentials: serde_json::Value,
    pub model_fallback: Option<serde_json::Value>,
    pub models: serde_json::Value,
    pub routing_agents: Option<serde_json::Value>,
    pub active: HashMap<String, String>,
    pub items: Vec<serde_json::Value>,
}

impl RegistryView {
    pub fn from_config(cfg: &Value) -> Self {
        let registry = ResolvedModelRegistry::from_config(cfg);
        let creds: HashMap<String, String> = cfg
            .get("provider_credentials")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        let masked_creds: serde_json::Map<String, serde_json::Value> = creds
            .keys()
            .map(|k| {
                (
                    k.clone(),
                    serde_json::json!({ "configured": true, "preview": "***" }),
                )
            })
            .collect();
        let active: HashMap<String, String> = registry
            .active
            .iter()
            .map(|(c, id)| (c.as_str().to_string(), id.clone()))
            .collect();
        let items: Vec<serde_json::Value> = registry
            .items
            .iter()
            .map(|item| {
                serde_json::json!({
                    "id": item.id,
                    "display_name": item.display_name,
                    "provider": item.provider,
                    "model": item.model,
                    "capabilities": item.capabilities.iter().map(|c| c.as_str()).collect::<Vec<_>>(),
                    "plan": item.plan,
                    "base_url": item.base_url,
                    "api_key": crate::config_models::MaskedSecret::from_value(item.api_key.as_deref()),
                    "enabled": item.enabled,
                    "source": item.source,
                })
            })
            .collect();
        Self {
            config_present: cfg.is_object() && !cfg.as_object().is_some_and(|o| o.is_empty()),
            provider: string_field(cfg, "provider", "provider"),
            model: string_field(cfg, "model", "model"),
            plan: string_field(cfg, "plan", "plan"),
            base_url: string_field(cfg, "base_url", "base_url"),
            api_key: crate::config_models::MaskedSecret::from_value(
                string_field(cfg, "api_key", "api_key").as_deref(),
            ),
            provider_credentials: serde_json::Value::Object(masked_creds),
            model_fallback: {
                let fb = crate::config_file::read_model_fallback(cfg);
                if fb.provider.is_none() && fb.model.is_none() {
                    None
                } else {
                    serde_json::to_value(fb).ok()
                }
            },
            models: serde_json::to_value(crate::config_file::read_models_config(cfg))
                .unwrap_or(serde_json::Value::Null),
            routing_agents: cfg.get("routing").and_then(|r| r.get("agents")).cloned(),
            active,
            items,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn migrates_flat_chat_to_registry() {
        let cfg = json!({
            "provider": "z.ai",
            "model": "glm-5",
            "api_key": "sk-test",
            "models": {}
        });
        let reg = ResolvedModelRegistry::from_config(&cfg);
        assert!(reg.active_item(ModelCapability::Chat).is_some());
        assert_eq!(
            reg.active_item(ModelCapability::Chat).unwrap().model,
            "glm-5"
        );
    }

    #[test]
    fn migrates_legacy_models_speech() {
        let cfg = json!({
            "provider": "openai",
            "api_key": "sk-test",
            "models": {
                "speech": { "stt": { "model": "whisper-1" } }
            }
        });
        let reg = ResolvedModelRegistry::from_config(&cfg);
        assert!(reg.active_item(ModelCapability::Stt).is_some());
    }
}
