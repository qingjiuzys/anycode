//! Read/migrate/patch helpers for `~/.anycode/config.json` (flat AnyCodeConfig SSOT).

use crate::config_models::{ModelFallbackConfig, ModelProfileFile, ModelsConfigFile};
use crate::model_registry::{
    sync_flat_chat_fields, sync_legacy_models_section, ResolvedModelRegistry,
};
use anyhow::{Context, Result};
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

pub fn default_config_path() -> PathBuf {
    std::env::var("HOME")
        .map(|h| PathBuf::from(h).join(".anycode").join("config.json"))
        .unwrap_or_else(|_| PathBuf::from(".anycode/config.json"))
}

pub fn read_config_value(path: Option<&Path>) -> Result<(PathBuf, Value)> {
    let path = path.map(PathBuf::from).unwrap_or_else(default_config_path);
    if !path.exists() {
        return Ok((path, json!({})));
    }
    let text = std::fs::read_to_string(&path).context("read config.json")?;
    let v: Value = serde_json::from_str(&text).context("parse config.json")?;
    Ok((path, v))
}

pub fn write_config_value(path: &Path, cfg: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("create config directory")?;
    }
    let body = serde_json::to_string_pretty(cfg).context("serialize config.json")?;
    std::fs::write(path, body).context("write config.json")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// Lift legacy nested `llm.*` into flat top-level fields when flat fields are absent.
pub fn migrate_legacy_llm_section(cfg: &mut Value) -> bool {
    let Some(obj) = cfg.as_object_mut() else {
        return false;
    };
    let Some(llm) = obj.get("llm").cloned() else {
        return false;
    };
    let mut changed = false;
    for (key, flat_key) in [
        ("provider", "provider"),
        ("model", "model"),
        ("plan", "plan"),
        ("base_url", "base_url"),
        ("api_key", "api_key"),
        ("temperature", "temperature"),
        ("max_tokens", "max_tokens"),
    ] {
        if obj.get(flat_key).is_none() {
            if let Some(v) = llm.get(key) {
                if !v.is_null() {
                    obj.insert(flat_key.to_string(), v.clone());
                    changed = true;
                }
            }
        }
    }
    if changed {
        obj.remove("llm");
    }
    changed
}

pub fn migrate_and_persist(path: &Path, cfg: &mut Value) -> Result<bool> {
    let changed = migrate_legacy_llm_section(cfg);
    if changed {
        write_config_value(path, cfg)?;
    }
    Ok(changed)
}

pub fn string_field(cfg: &Value, flat: &str, nested: &str) -> Option<String> {
    cfg.get(flat)
        .or_else(|| cfg.get("llm").and_then(|l| l.get(nested)))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

pub fn read_model_fallback(cfg: &Value) -> ModelFallbackConfig {
    cfg.get("runtime")
        .and_then(|r| r.get("model_fallback"))
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default()
}

pub fn read_models_config(cfg: &Value) -> ModelsConfigFile {
    let registry = ResolvedModelRegistry::from_config(cfg);
    sync_legacy_models_section(&registry)
}

fn merge_models_section(existing: &mut Map<String, Value>, patch: &ModelsConfigFile) -> Result<()> {
    if let Some(active) = patch.active.as_ref() {
        let active_obj = existing.entry("active").or_insert(json!({}));
        if let Some(active_map) = active_obj.as_object_mut() {
            for (k, v) in active {
                if v.trim().is_empty() {
                    active_map.remove(k);
                } else {
                    active_map.insert(k.clone(), json!(v));
                }
            }
        }
    }

    if let Some(items) = patch.items.as_ref() {
        let items_val = existing.entry("items").or_insert(json!([]));
        let items_arr = items_val
            .as_array_mut()
            .context("models.items must be array")?;
        for item in items {
            if let Some(pos) = items_arr.iter().position(|v| {
                v.get("id")
                    .and_then(|id| id.as_str())
                    .is_some_and(|id| id == item.id)
            }) {
                items_arr[pos] = serde_json::to_value(item)?;
            } else {
                items_arr.push(serde_json::to_value(item)?);
            }
        }
    }

    fn merge_profile_section(
        key: &str,
        profile: &ModelProfileFile,
        existing: &mut Map<String, Value>,
    ) {
        let section = existing.entry(key).or_insert(json!({}));
        if let Some(obj) = section.as_object_mut() {
            merge_profile(obj, profile);
        }
    }

    if let Some(ref chat) = patch.chat {
        merge_profile_section("chat", chat, existing);
    }
    if let Some(ref emb) = patch.embedding {
        merge_profile_section("embedding", emb, existing);
    }
    if let Some(ref speech) = patch.speech {
        let speech_obj = existing.entry("speech").or_insert(json!({}));
        if let Some(s) = speech_obj.as_object_mut() {
            if let Some(ref stt) = speech.stt {
                let stt_obj = s.entry("stt").or_insert(json!({}));
                if let Some(obj) = stt_obj.as_object_mut() {
                    merge_profile(obj, stt);
                }
            }
            if let Some(ref tts) = speech.tts {
                let tts_obj = s.entry("tts").or_insert(json!({}));
                if let Some(obj) = tts_obj.as_object_mut() {
                    merge_profile(obj, tts);
                }
            }
        }
    }
    if let Some(ref img) = patch.image {
        merge_profile_section("image", img, existing);
    }
    if let Some(ref vid) = patch.video {
        merge_profile_section("video", vid, existing);
    }

    Ok(())
}

fn apply_registry_sync(cfg: &mut Value) -> Result<()> {
    let registry = ResolvedModelRegistry::from_config(cfg);
    sync_flat_chat_fields(cfg, &registry);
    let legacy = sync_legacy_models_section(&registry);
    let obj = cfg.as_object_mut().context("config root must be object")?;
    let models_val = obj.entry("models").or_insert(json!({}));
    if let Some(models_obj) = models_val.as_object_mut() {
        if let Some(ref active) = legacy.active {
            models_obj.insert(
                "active".into(),
                serde_json::to_value(active).context("serialize active")?,
            );
        }
        if let Some(ref items) = legacy.items {
            models_obj.insert(
                "items".into(),
                serde_json::to_value(items).context("serialize items")?,
            );
        }
        merge_models_section(models_obj, &legacy)?;
    }
    Ok(())
}

fn merge_profile(obj: &mut Map<String, Value>, patch: &ModelProfileFile) {
    if let Some(p) = patch
        .provider
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        obj.insert("provider".into(), json!(p));
    }
    if let Some(m) = patch
        .model
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        obj.insert("model".into(), json!(m));
    }
    if let Some(p) = patch
        .plan
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        obj.insert("plan".into(), json!(p));
    }
    if let Some(u) = patch.base_url.as_ref() {
        if u.trim().is_empty() {
            obj.remove("base_url");
        } else {
            obj.insert("base_url".into(), json!(u.trim()));
        }
    }
    if let Some(k) = patch.api_key.as_ref().filter(|s| !s.trim().is_empty()) {
        obj.insert("api_key".into(), json!(k.trim()));
    }
}

#[derive(Debug, Clone, Default)]
pub struct LlmConfigPatch {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub plan: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub provider_credentials: Option<std::collections::HashMap<String, String>>,
    pub fallback: Option<ModelFallbackConfig>,
    pub routing_agents: Option<std::collections::HashMap<String, ModelProfileFile>>,
    /// Agent keys to remove from routing.agents
    pub routing_agents_delete: Option<Vec<String>>,
    pub models: Option<ModelsConfigFile>,
    /// Replace entire models section (legacy); prefer deep merge via `models`
    pub models_replace: bool,
}

pub fn patch_llm_config_value(cfg: &mut Value, patch: &LlmConfigPatch) -> Result<()> {
    let obj = cfg.as_object_mut().context("config root must be object")?;

    if let Some(p) = patch
        .provider
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        obj.insert("provider".into(), json!(p));
        obj.remove("llm");
    }
    if let Some(m) = patch
        .model
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        obj.insert("model".into(), json!(m));
        obj.remove("llm");
    }
    if let Some(p) = patch.plan.as_ref() {
        if p.trim().is_empty() {
            obj.remove("plan");
        } else {
            obj.insert("plan".into(), json!(p.trim()));
        }
    }
    if let Some(u) = patch.base_url.as_ref() {
        if u.trim().is_empty() {
            obj.remove("base_url");
        } else {
            obj.insert("base_url".into(), json!(u.trim()));
        }
    }
    if let Some(k) = patch.api_key.as_ref().filter(|s| !s.trim().is_empty()) {
        obj.insert("api_key".into(), json!(k.trim()));
    }
    if let Some(creds) = patch.provider_credentials.as_ref() {
        let creds_obj = obj.entry("provider_credentials").or_insert(json!({}));
        if let Some(creds_map) = creds_obj.as_object_mut() {
            for (k, v) in creds {
                if v.trim().is_empty() {
                    creds_map.remove(k);
                } else {
                    creds_map.insert(k.clone(), json!(v.trim()));
                }
            }
        }
    }

    if let Some(fb) = patch.fallback.as_ref() {
        let runtime = obj.entry("runtime").or_insert(json!({}));
        if let Some(rt) = runtime.as_object_mut() {
            rt.insert(
                "model_fallback".into(),
                serde_json::to_value(fb).context("serialize model_fallback")?,
            );
        }
    }

    if let Some(agents) = patch.routing_agents.as_ref() {
        let routing = obj.entry("routing").or_insert(json!({}));
        if let Some(r) = routing.as_object_mut() {
            let agents_val = r.entry("agents").or_insert(json!({}));
            if let Some(agents_map) = agents_val.as_object_mut() {
                for (agent, profile) in agents {
                    let entry = agents_map
                        .entry(agent.clone())
                        .or_insert(json!({}))
                        .as_object_mut()
                        .context("routing agent must be object")?;
                    merge_profile(entry, profile);
                }
            }
        }
    }

    if let Some(delete) = patch.routing_agents_delete.as_ref() {
        let routing = obj.entry("routing").or_insert(json!({}));
        if let Some(r) = routing.as_object_mut() {
            if let Some(agents_map) = r.get_mut("agents").and_then(|v| v.as_object_mut()) {
                for key in delete {
                    agents_map.remove(key);
                }
            }
        }
    }

    if let Some(models) = patch.models.as_ref() {
        if patch.models_replace {
            obj.insert(
                "models".into(),
                serde_json::to_value(models).context("serialize models")?,
            );
        } else {
            let models_val = obj.entry("models").or_insert(json!({}));
            let models_obj = models_val
                .as_object_mut()
                .context("models must be object")?;
            merge_models_section(models_obj, models)?;
        }
    }

    apply_registry_sync(cfg)?;

    Ok(())
}

pub fn patch_llm_config(path: Option<&Path>, patch: &LlmConfigPatch) -> Result<(PathBuf, Value)> {
    let (path, mut cfg) = read_config_value(path)?;
    if !cfg.is_object() {
        cfg = json!({});
    }
    migrate_legacy_llm_section(&mut cfg);
    patch_llm_config_value(&mut cfg, patch)?;
    write_config_value(&path, &cfg)?;
    Ok((path, cfg))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_models::{FailoverTrigger, ModelProfileFile, ModelsConfigFile};

    #[test]
    fn migrate_legacy_llm_to_flat() {
        let mut cfg = json!({
            "llm": { "provider": "anthropic", "model": "claude-sonnet" },
            "runtime": {}
        });
        assert!(migrate_legacy_llm_section(&mut cfg));
        assert_eq!(
            cfg.get("provider").and_then(|v| v.as_str()),
            Some("anthropic")
        );
        assert!(cfg.get("llm").is_none());
    }

    #[test]
    fn patch_models_deep_merge_preserves_speech() {
        let mut cfg = json!({
            "provider": "openai",
            "api_key": "sk-test",
            "models": {
                "speech": { "stt": { "model": "whisper-1" }, "tts": { "model": "tts-1" } },
                "embedding": { "model": "text-embedding-3-small" }
            }
        });
        let patch = ModelsConfigFile {
            image: Some(ModelProfileFile {
                model: Some("dall-e-3".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        patch_llm_config_value(
            &mut cfg,
            &LlmConfigPatch {
                models: Some(patch),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            cfg.pointer("/models/speech/stt/model")
                .and_then(|v| v.as_str()),
            Some("whisper-1")
        );
        assert_eq!(
            cfg.pointer("/models/image/model").and_then(|v| v.as_str()),
            Some("dall-e-3")
        );
    }
}
