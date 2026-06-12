use anyhow::{bail, Result};
use serde_json::{json, Value};

/// Memory strategy presets shared by CLI `anycode setup` and Dashboard setup wizard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemorySetupPreset {
    Noop,
    SimpleFile,
    HybridBackend,
    PipelineNoEmbedding,
    PipelineHttp {
        embedding_base_url: String,
        embedding_model: String,
    },
    #[cfg(feature = "embedding-local")]
    PipelineLocalOnnx {
        model_id: String,
        hf_endpoint: Option<String>,
    },
}

impl MemorySetupPreset {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Noop => "noop",
            Self::SimpleFile => "simple_file",
            Self::HybridBackend => "hybrid",
            Self::PipelineNoEmbedding => "pipeline_no_embedding",
            Self::PipelineHttp { .. } => "pipeline_http",
            #[cfg(feature = "embedding-local")]
            Self::PipelineLocalOnnx { .. } => "pipeline_local",
        }
    }
}

pub fn memory_preset_from_label(label: &str) -> Result<MemorySetupPreset> {
    match label.trim().to_ascii_lowercase().as_str() {
        "noop" | "off" | "none" => Ok(MemorySetupPreset::Noop),
        "simple_file" | "file" | "simple" => Ok(MemorySetupPreset::SimpleFile),
        "hybrid" | "markdown" | "smart" => Ok(MemorySetupPreset::HybridBackend),
        "pipeline_no_embedding" => Ok(MemorySetupPreset::PipelineNoEmbedding),
        other if other.starts_with("pipeline_http:") => {
            let rest = other.trim_start_matches("pipeline_http:");
            let (url, model) = rest
                .split_once('|')
                .ok_or_else(|| anyhow::anyhow!("pipeline_http preset needs url|model"))?;
            Ok(MemorySetupPreset::PipelineHttp {
                embedding_base_url: url.to_string(),
                embedding_model: model.to_string(),
            })
        }
        #[cfg(feature = "embedding-local")]
        other if other.starts_with("pipeline_local:") => {
            let model_id = other.trim_start_matches("pipeline_local:").to_string();
            Ok(MemorySetupPreset::PipelineLocalOnnx {
                model_id,
                hf_endpoint: Some("https://hf-mirror.com".to_string()),
            })
        }
        _ => bail!("unknown memory preset: {label}"),
    }
}

fn memory_obj(cfg: &mut Value) -> &mut Value {
    if !cfg.get("memory").is_some_and(|m| m.is_object()) {
        cfg["memory"] = json!({});
    }
    cfg.get_mut("memory").expect("memory object")
}

fn pipeline_obj(memory: &mut Value) -> &mut Value {
    if !memory.get("pipeline").is_some_and(|p| p.is_object()) {
        memory["pipeline"] = json!({});
    }
    memory.get_mut("pipeline").expect("pipeline object")
}

fn clear_pipeline_embedding_local_fields(pipeline: &mut Value) {
    if let Some(obj) = pipeline.as_object_mut() {
        obj.remove("embedding_local_model");
        obj.remove("embedding_hf_endpoint");
        obj.remove("embedding_local_cache_dir");
    }
}

/// Patch the `memory` section of a full config JSON value.
pub fn apply_memory_preset(cfg: &mut Value, preset: MemorySetupPreset) {
    let path = cfg.get("memory").and_then(|m| m.get("path")).cloned();
    let auto_save = cfg
        .get("memory")
        .and_then(|m| m.get("auto_save"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let memory = memory_obj(cfg);
    match preset {
        MemorySetupPreset::Noop => {
            memory["backend"] = json!("noop");
        }
        MemorySetupPreset::SimpleFile => {
            *memory = json!({
                "backend": "file",
                "auto_save": auto_save,
                "pipeline": {}
            });
            if let Some(p) = path {
                memory["path"] = p;
            }
        }
        MemorySetupPreset::HybridBackend => {
            memory["backend"] = json!("hybrid");
            memory["pipeline"] = json!({});
            if let Some(p) = path {
                memory["path"] = p;
            }
            memory["auto_save"] = json!(auto_save);
        }
        MemorySetupPreset::PipelineNoEmbedding => {
            memory["backend"] = json!("pipeline");
            let pipeline = pipeline_obj(memory);
            pipeline["embedding_enabled"] = json!(false);
            pipeline.as_object_mut().map(|o| {
                o.remove("embedding_model");
                o.remove("embedding_base_url");
                o.remove("embedding_provider");
            });
            clear_pipeline_embedding_local_fields(pipeline);
            if let Some(p) = path {
                memory["path"] = p;
            }
            memory["auto_save"] = json!(auto_save);
        }
        MemorySetupPreset::PipelineHttp {
            embedding_base_url,
            embedding_model,
        } => {
            memory["backend"] = json!("pipeline");
            let pipeline = pipeline_obj(memory);
            pipeline["embedding_enabled"] = json!(true);
            pipeline["embedding_provider"] = json!("http");
            pipeline["embedding_base_url"] = json!(embedding_base_url);
            pipeline["embedding_model"] = json!(embedding_model);
            clear_pipeline_embedding_local_fields(pipeline);
            if let Some(p) = path {
                memory["path"] = p;
            }
            memory["auto_save"] = json!(auto_save);
        }
        #[cfg(feature = "embedding-local")]
        MemorySetupPreset::PipelineLocalOnnx {
            model_id,
            hf_endpoint,
        } => {
            memory["backend"] = json!("pipeline");
            let pipeline = pipeline_obj(memory);
            pipeline["embedding_enabled"] = json!(true);
            pipeline["embedding_provider"] = json!("local");
            pipeline["embedding_local_model"] = json!(model_id);
            if let Some(ep) = hf_endpoint {
                pipeline["embedding_hf_endpoint"] = json!(ep);
            }
            pipeline.as_object_mut().map(|o| {
                o.remove("embedding_base_url");
                o.remove("embedding_model");
            });
            if let Some(p) = path {
                memory["path"] = p;
            }
            memory["auto_save"] = json!(auto_save);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn preset_noop() {
        let mut cfg = json!({ "memory": { "backend": "file" } });
        apply_memory_preset(&mut cfg, MemorySetupPreset::Noop);
        assert_eq!(cfg["memory"]["backend"], "noop");
    }

    #[test]
    fn preset_hybrid() {
        let mut cfg = json!({});
        apply_memory_preset(&mut cfg, MemorySetupPreset::HybridBackend);
        assert_eq!(cfg["memory"]["backend"], "hybrid");
    }

    #[test]
    fn preset_http() {
        let mut cfg = json!({});
        apply_memory_preset(
            &mut cfg,
            MemorySetupPreset::PipelineHttp {
                embedding_base_url: "https://api.example/v1".into(),
                embedding_model: "text-embedding-3-small".into(),
            },
        );
        assert_eq!(cfg["memory"]["backend"], "pipeline");
        assert_eq!(
            cfg["memory"]["pipeline"]["embedding_base_url"],
            "https://api.example/v1"
        );
    }
}
