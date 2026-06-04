//! Model catalog service: static presets + cached remote refresh.

use crate::{
    capability_catalog::ModelCapability,
    deepseek_catalog::{catalog_entry_for_id, DEEPSEEK_MODEL_CATALOG, DEEPSEEK_OPENAI_API_ROOT},
    google_catalog::GOOGLE_MODEL_CATALOG,
    provider_catalog::PROVIDER_CATALOG,
    providers::zai::ZAI_MODEL_CATALOG,
    ROUTING_AGENT_PRESETS, ZAI_AUTH_METHODS,
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn catalog_cache_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".anycode")
        .join("catalog-cache")
}

fn cache_path(provider: &str) -> PathBuf {
    catalog_cache_dir().join(format!("{provider}.json"))
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CatalogRefreshMeta {
    pub last_refreshed_at: Option<String>,
    pub source: String,
    pub offline_cache_used: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CachedProviderCatalog {
    pub provider: String,
    pub models: Vec<CatalogModelEntry>,
    pub meta: CatalogRefreshMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogModelEntry {
    pub id: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

pub fn builtin_catalog_models(provider: &str) -> Vec<CatalogModelEntry> {
    let id = provider.trim().to_ascii_lowercase();
    if matches!(id.as_str(), "z.ai" | "zai" | "bigmodel" | "glm") {
        return ZAI_MODEL_CATALOG
            .iter()
            .map(|m| CatalogModelEntry {
                id: m.api_name.to_string(),
                label: m.display_name.to_string(),
                description: Some(m.description.to_string()),
                capabilities: vec![ModelCapability::Chat.as_str().to_string()],
            })
            .collect();
    }
    if matches!(id.as_str(), "google" | "gemini") {
        return GOOGLE_MODEL_CATALOG
            .iter()
            .map(|m| CatalogModelEntry {
                id: m.id.to_string(),
                label: m.label.to_string(),
                description: None,
                capabilities: vec![
                    ModelCapability::Chat.as_str().to_string(),
                    ModelCapability::Vision.as_str().to_string(),
                ],
            })
            .collect();
    }
    if matches!(id.as_str(), "deepseek" | "deep-seek" | "deep_seek") {
        return DEEPSEEK_MODEL_CATALOG
            .iter()
            .map(|m| CatalogModelEntry {
                id: m.id.to_string(),
                label: m.label.to_string(),
                description: Some(m.description.to_string()),
                capabilities: vec![ModelCapability::Chat.as_str().to_string()],
            })
            .collect();
    }
    vec![]
}

fn is_deepseek_provider(provider: &str) -> bool {
    matches!(
        provider.trim().to_ascii_lowercase().as_str(),
        "deepseek" | "deep-seek" | "deep_seek"
    )
}

/// Merge official GET /models results with static V4 + legacy alias metadata.
#[must_use]
pub fn merge_deepseek_catalog(
    remote: Vec<CatalogModelEntry>,
    builtin: Vec<CatalogModelEntry>,
) -> Vec<CatalogModelEntry> {
    let mut order: Vec<String> = Vec::new();
    let mut by_id: HashMap<String, CatalogModelEntry> = HashMap::new();

    for m in remote {
        let enriched = catalog_entry_for_id(&m.id).map(|e| CatalogModelEntry {
            id: e.id.to_string(),
            label: e.label.to_string(),
            description: Some(e.description.to_string()),
            capabilities: m.capabilities.clone(),
        });
        let entry = enriched.unwrap_or(m);
        if !by_id.contains_key(&entry.id) {
            order.push(entry.id.clone());
        }
        by_id.insert(entry.id.clone(), entry);
    }

    for m in builtin {
        if !by_id.contains_key(&m.id) {
            order.push(m.id.clone());
            by_id.insert(m.id.clone(), m);
        }
    }

    let rank = |id: &str| -> u8 {
        match id {
            "deepseek-v4-pro" => 0,
            "deepseek-v4-flash" => 1,
            "deepseek-chat" => 2,
            "deepseek-reasoner" => 3,
            _ => 4,
        }
    };
    order.sort_by(|a, b| rank(a).cmp(&rank(b)).then_with(|| a.cmp(b)));
    order
        .into_iter()
        .filter_map(|id| by_id.remove(&id))
        .collect()
}

/// Strip `/chat/completions` so `…/v1/models` resolves correctly.
pub fn normalize_openai_api_base(base: &str) -> String {
    let mut b = base.trim().trim_end_matches('/').to_string();
    for suffix in ["/chat/completions", "/completions"] {
        if b.ends_with(suffix) {
            b.truncate(b.len() - suffix.len());
            b = b.trim_end_matches('/').to_string();
            break;
        }
    }
    b
}

fn default_openai_api_base(provider: &str) -> &'static str {
    match provider.trim().to_ascii_lowercase().as_str() {
        "deepseek" | "deep-seek" | "deep_seek" => DEEPSEEK_OPENAI_API_ROOT,
        _ => "https://api.openai.com/v1",
    }
}

fn openai_models_url(base: &str) -> String {
    let b = normalize_openai_api_base(base);
    format!("{b}/models")
}

async fn fetch_openai_compatible_models(
    provider: &str,
    base_url: Option<&str>,
    api_key: Option<&str>,
) -> Result<Vec<CatalogModelEntry>> {
    let base = base_url
        .filter(|s| !s.trim().is_empty())
        .map(str::trim)
        .map(normalize_openai_api_base)
        .unwrap_or_else(|| default_openai_api_base(provider).to_string());
    let url = openai_models_url(&base);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    let mut req = client.get(&url);
    if let Some(key) = api_key.filter(|s| !s.trim().is_empty()) {
        req = req.bearer_auth(key.trim());
    }
    let resp = req.send().await.context("fetch models")?;
    if !resp.status().is_success() {
        anyhow::bail!("models list status {}", resp.status());
    }
    let body: Value = resp.json().await.context("parse models json")?;
    let mut out = Vec::new();
    if let Some(data) = body.get("data").and_then(|d| d.as_array()) {
        for item in data {
            if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                out.push(CatalogModelEntry {
                    id: id.to_string(),
                    label: id.to_string(),
                    description: None,
                    capabilities: infer_capabilities(provider, id),
                });
            }
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

fn infer_capabilities(provider: &str, model_id: &str) -> Vec<String> {
    let mid = model_id.to_ascii_lowercase();
    let mut caps = Vec::new();
    if mid.contains("whisper") || mid.contains("transcribe") {
        caps.push(ModelCapability::Stt.as_str().to_string());
    } else if mid.contains("tts") || mid.contains("speech") {
        caps.push(ModelCapability::Tts.as_str().to_string());
    } else if mid.contains("embed") {
        caps.push(ModelCapability::Embedding.as_str().to_string());
    } else if mid.contains("dall-e") || mid.contains("image") {
        caps.push(ModelCapability::ImageGen.as_str().to_string());
    } else if mid.contains("video") {
        caps.push(ModelCapability::VideoGen.as_str().to_string());
    } else {
        caps.push(ModelCapability::Chat.as_str().to_string());
        if mid.contains("vision")
            || mid.contains("gemini")
            || mid.contains("gpt-4")
            || mid.contains("claude-3")
        {
            caps.push(ModelCapability::Vision.as_str().to_string());
        }
    }
    let _ = provider;
    caps
}

pub fn load_cached_catalog(provider: &str) -> Option<CachedProviderCatalog> {
    let path = cache_path(provider);
    if !path.exists() {
        return None;
    }
    let text = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn save_cached_catalog(catalog: &CachedProviderCatalog) -> Result<()> {
    let dir = catalog_cache_dir();
    std::fs::create_dir_all(&dir).context("create catalog cache dir")?;
    let path = cache_path(&catalog.provider);
    let body = serde_json::to_string_pretty(catalog).context("serialize catalog cache")?;
    std::fs::write(path, body).context("write catalog cache")?;
    Ok(())
}

pub async fn refresh_provider_catalog(
    provider: &str,
    base_url: Option<&str>,
    api_key: Option<&str>,
) -> Result<CachedProviderCatalog> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string();

    let builtin = builtin_catalog_models(provider);

    if is_deepseek_provider(provider) {
        let (merged, source) =
            match fetch_openai_compatible_models(provider, base_url, api_key).await {
                Ok(remote) => (
                    merge_deepseek_catalog(remote, builtin.clone()),
                    "remote+builtin",
                ),
                Err(e) => {
                    tracing::warn!(
                        target: "anycode_llm",
                        "deepseek catalog remote refresh failed: {e}; using builtin"
                    );
                    (builtin.clone(), "builtin")
                }
            };
        let catalog = CachedProviderCatalog {
            provider: provider.to_string(),
            models: merged,
            meta: CatalogRefreshMeta {
                last_refreshed_at: Some(now),
                source: source.into(),
                offline_cache_used: false,
                refresh_error: None,
            },
        };
        save_cached_catalog(&catalog)?;
        return Ok(catalog);
    }

    if !builtin.is_empty() {
        let catalog = CachedProviderCatalog {
            provider: provider.to_string(),
            models: builtin,
            meta: CatalogRefreshMeta {
                last_refreshed_at: Some(now),
                source: "builtin".into(),
                offline_cache_used: false,
                refresh_error: None,
            },
        };
        let _ = save_cached_catalog(&catalog);
        return Ok(catalog);
    }

    match fetch_openai_compatible_models(provider, base_url, api_key).await {
        Ok(models) => {
            let catalog = CachedProviderCatalog {
                provider: provider.to_string(),
                models,
                meta: CatalogRefreshMeta {
                    last_refreshed_at: Some(now),
                    source: "remote".into(),
                    offline_cache_used: false,
                    refresh_error: None,
                },
            };
            save_cached_catalog(&catalog)?;
            Ok(catalog)
        }
        Err(e) => {
            if let Some(cached) = load_cached_catalog(provider) {
                let mut c = cached;
                c.meta.offline_cache_used = true;
                c.meta.refresh_error = Some(e.to_string());
                Ok(c)
            } else {
                Err(e)
            }
        }
    }
}

pub fn aggregate_catalog_view() -> Value {
    let providers: Vec<Value> = PROVIDER_CATALOG
        .iter()
        .map(|e| {
            json!({
                "id": e.id,
                "label": e.label,
                "hint": e.hint,
                "transport": format!("{:?}", e.transport),
                "suggested_openai_base": e.suggested_openai_base,
                "placeholder_only": e.placeholder_only,
            })
        })
        .collect();

    let zai_models: Vec<Value> = ZAI_MODEL_CATALOG
        .iter()
        .map(|m| json!({ "id": m.api_name, "label": m.display_name, "description": m.description }))
        .collect();

    let google_models: Vec<Value> = GOOGLE_MODEL_CATALOG
        .iter()
        .map(|m| json!({ "id": m.id, "label": m.label }))
        .collect();

    let deepseek_models: Vec<Value> =
        merge_deepseek_catalog(vec![], builtin_catalog_models("deepseek"))
            .into_iter()
            .map(|m| {
                json!({
                    "id": m.id,
                    "label": m.label,
                    "description": m.description,
                    "capabilities": m.capabilities,
                })
            })
            .collect();

    let zai_auth: Vec<Value> = ZAI_AUTH_METHODS
        .iter()
        .map(|m| json!({ "label": m.label, "hint": m.hint, "plan": m.plan }))
        .collect();

    let routing_presets: Vec<Value> = ROUTING_AGENT_PRESETS
        .iter()
        .map(|(id, hint)| json!({ "id": id, "hint": hint }))
        .collect();

    let capabilities: Vec<Value> = ModelCapability::all()
        .iter()
        .map(|c| json!({ "id": c.as_str(), "label": c.as_str() }))
        .collect();

    let mut cache_meta: HashMap<String, CatalogRefreshMeta> = HashMap::new();
    let mut provider_models: HashMap<String, Vec<CatalogModelEntry>> = HashMap::new();
    for p in PROVIDER_CATALOG.iter() {
        if let Some(c) = load_cached_catalog(p.id) {
            cache_meta.insert(p.id.to_string(), c.meta.clone());
            if !c.models.is_empty() {
                let models = if is_deepseek_provider(p.id) {
                    merge_deepseek_catalog(c.models.clone(), builtin_catalog_models(p.id))
                } else {
                    c.models.clone()
                };
                provider_models.insert(p.id.to_string(), models);
            }
        } else {
            let builtin = builtin_catalog_models(p.id);
            if !builtin.is_empty() {
                let models = if is_deepseek_provider(p.id) {
                    merge_deepseek_catalog(vec![], builtin)
                } else {
                    builtin
                };
                provider_models.insert(p.id.to_string(), models);
            }
        }
    }

    json!({
        "providers": providers,
        "zai_models": zai_models,
        "google_models": google_models,
        "deepseek_models": deepseek_models,
        "provider_models": provider_models,
        "zai_auth_methods": zai_auth,
        "routing_agent_presets": routing_presets,
        "capabilities": capabilities,
        "cache_meta": cache_meta,
    })
}

pub fn cached_models_for_provider(provider: &str) -> Vec<CatalogModelEntry> {
    load_cached_catalog(provider)
        .map(|c| c.models)
        .unwrap_or_else(|| builtin_catalog_models(provider))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_zai_models_non_empty() {
        let models = builtin_catalog_models("z.ai");
        assert!(!models.is_empty());
    }

    #[test]
    fn builtin_deepseek_models_non_empty() {
        let models = builtin_catalog_models("deepseek");
        assert!(models.len() >= 2);
        assert!(models.iter().any(|m| m.id == "deepseek-v4-pro"));
    }

    #[test]
    fn normalize_strips_chat_completions_suffix() {
        assert_eq!(
            normalize_openai_api_base("https://api.deepseek.com/v1/chat/completions"),
            "https://api.deepseek.com/v1"
        );
        assert_eq!(
            openai_models_url("https://api.deepseek.com/chat/completions"),
            "https://api.deepseek.com/models"
        );
    }

    #[test]
    fn merge_deepseek_includes_legacy_aliases() {
        let remote = vec![CatalogModelEntry {
            id: "deepseek-v4-pro".into(),
            label: "deepseek-v4-pro".into(),
            description: None,
            capabilities: vec!["chat".into()],
        }];
        let builtin = builtin_catalog_models("deepseek");
        let merged = merge_deepseek_catalog(remote, builtin);
        assert!(merged.iter().any(|m| m.id == "deepseek-v4-flash"));
        assert!(merged.iter().any(|m| m.id == "deepseek-chat"));
    }
}
