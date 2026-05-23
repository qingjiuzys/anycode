//! Safe partial updates to `~/.anycode/config.json` from the dashboard.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;

fn anycode_config_path() -> PathBuf {
    std::env::var("HOME")
        .map(|h| PathBuf::from(h).join(".anycode").join("config.json"))
        .unwrap_or_else(|_| PathBuf::from(".anycode/config.json"))
}

fn read_config() -> Result<(PathBuf, Value)> {
    let path = anycode_config_path();
    if !path.exists() {
        return Ok((path, json!({})));
    }
    let text = std::fs::read_to_string(&path).context("read config.json")?;
    let v: Value = serde_json::from_str(&text).context("parse config.json")?;
    Ok((path, v))
}

fn write_config(path: &PathBuf, cfg: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("create config directory")?;
    }
    let body = serde_json::to_string_pretty(cfg).context("serialize config")?;
    std::fs::write(path, body).context("write config.json")?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmConfigPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_model: Option<String>,
}

pub fn patch_llm_config(patch: &LlmConfigPatch) -> Result<(PathBuf, Value)> {
    let (path, mut cfg) = read_config()?;
    if !cfg.is_object() {
        cfg = json!({});
    }
    let obj = cfg.as_object_mut().context("config root must be object")?;

    if patch.provider.is_some() || patch.model.is_some() {
        let llm = obj.entry("llm").or_insert(json!({}));
        if let Some(llm_obj) = llm.as_object_mut() {
            if let Some(p) = patch
                .provider
                .as_ref()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
            {
                llm_obj.insert("provider".into(), json!(p));
            }
            if let Some(m) = patch
                .model
                .as_ref()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
            {
                llm_obj.insert("model".into(), json!(m));
            }
        }
    }

    if patch.fallback_provider.is_some() || patch.fallback_model.is_some() {
        let runtime = obj.entry("runtime").or_insert(json!({}));
        if let Some(rt) = runtime.as_object_mut() {
            let fb = rt.entry("model_fallback").or_insert(json!({}));
            if let Some(fb_obj) = fb.as_object_mut() {
                if let Some(p) = patch
                    .fallback_provider
                    .as_ref()
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                {
                    fb_obj.insert("provider".into(), json!(p));
                }
                if let Some(m) = patch
                    .fallback_model
                    .as_ref()
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                {
                    fb_obj.insert("model".into(), json!(m));
                }
            }
        }
    }

    write_config(&path, &cfg)?;
    Ok((path, cfg))
}

pub fn read_llm_fallback(cfg: &Value) -> (Option<String>, Option<String>) {
    let fb = cfg.get("runtime").and_then(|r| r.get("model_fallback"));
    (
        fb.and_then(|f| f.get("provider"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        fb.and_then(|f| f.get("model"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_creates_llm_section_in_temp_home() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", dir.path());
        let (path, cfg) = patch_llm_config(&LlmConfigPatch {
            provider: Some("anthropic".into()),
            model: Some("claude-sonnet".into()),
            ..Default::default()
        })
        .unwrap();
        assert!(path.exists());
        assert_eq!(
            cfg.get("llm")
                .and_then(|l| l.get("provider"))
                .and_then(|v| v.as_str()),
            Some("anthropic")
        );
    }
}
