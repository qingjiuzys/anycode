//! Wire LLM client, tool registry, and `AgentRuntime` (shared by TUI, `run`, and long-lived bridges).

mod llm_session;
mod mcp_env;
mod prompt_runtime;
mod runtime;
mod skills_registry;

pub(crate) use runtime::initialize_runtime;

use crate::app_config::{default_base_url_for, Config};
use crate::i18n::tr_args;
use anycode_core::prelude::*;
use anycode_core::{EmbeddingProvider, MemoryPipeline, VectorMemoryBackend};
use anycode_llm::{normalize_provider_id, ModelRouter};
#[cfg(feature = "embedding-local")]
use anycode_memory::FastEmbedEmbeddingProvider;
use anycode_memory::{
    FileMemoryStore, HybridMemoryStore, NoopVectorBackend, OpenAiCompatibleEmbeddingProvider,
    RootReturnMemoryPipeline, SledVectorBackend,
};
use async_trait::async_trait;
use fluent_bundle::FluentArgs;
use llm_session::{effective_provider, resolve_agent_base_url, resolve_profile_api_key};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn compile_tool_name_deny_regexes(patterns: &[String]) -> Vec<regex::Regex> {
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

struct NoopMemoryStore;

#[async_trait]
impl MemoryStore for NoopMemoryStore {
    async fn save(&self, _memory: Memory) -> Result<(), CoreError> {
        Ok(())
    }

    async fn recall(&self, _query: &str, _mem_type: MemoryType) -> Result<Vec<Memory>, CoreError> {
        Ok(vec![])
    }

    async fn update(&self, _id: &str, _memory: Memory) -> Result<(), CoreError> {
        Ok(())
    }

    async fn delete(&self, _id: &str) -> Result<(), CoreError> {
        Ok(())
    }
}

fn sibling_sled_path(file_memory_root: &Path) -> std::path::PathBuf {
    let name = file_memory_root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("memory");
    let parent = file_memory_root.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!("{}.sled", name))
}

/// 热层 Sled 路径（归根通道 `pipeline` backend，与 `hybrid` 的 sibling sled 命名区分）。
fn sibling_pipeline_sled_path(file_memory_root: &Path) -> std::path::PathBuf {
    let name = file_memory_root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("memory");
    let parent = file_memory_root.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!("{}.pipeline.sled", name))
}

fn sibling_pipeline_buffer_wal_path(pipeline_hot_sled: &Path) -> std::path::PathBuf {
    let s = pipeline_hot_sled.to_string_lossy();
    if s.ends_with(".sled") {
        PathBuf::from(format!("{}.buffer.wal", s.strip_suffix(".sled").unwrap()))
    } else {
        pipeline_hot_sled.with_extension("buffer.wal")
    }
}

fn sibling_pipeline_vector_sled_path(pipeline_hot_sled: &Path) -> std::path::PathBuf {
    let s = pipeline_hot_sled.to_string_lossy();
    if s.ends_with(".pipeline.sled") {
        PathBuf::from(s.replace(".pipeline.sled", ".pipeline.vec.sled"))
    } else if s.ends_with(".sled") {
        PathBuf::from(format!("{}.vec.sled", s.strip_suffix(".sled").unwrap()))
    } else {
        pipeline_hot_sled.with_extension("vec.sled")
    }
}

fn open_file_memory_store(path: PathBuf) -> anyhow::Result<FileMemoryStore> {
    FileMemoryStore::new(path).map_err(|e| {
        let mut a = FluentArgs::new();
        a.set("err", e.to_string());
        anyhow::anyhow!("{}", tr_args("err-memory-file-store", &a))
    })
}

pub(crate) fn build_memory_layer(
    config: &Config,
) -> anyhow::Result<(Arc<dyn MemoryStore>, Option<Arc<dyn MemoryPipeline>>)> {
    match config.memory.backend.as_str() {
        "noop" => Ok((Arc::new(NoopMemoryStore), None)),
        "file" => {
            let store = open_file_memory_store(config.memory.path.clone())?;
            Ok((Arc::new(store), None))
        }
        "hybrid" => {
            let sled_path = sibling_sled_path(&config.memory.path);
            let store =
                HybridMemoryStore::new(sled_path, config.memory.path.clone()).map_err(|e| {
                    let mut a = FluentArgs::new();
                    a.set("err", e.to_string());
                    anyhow::anyhow!("{}", tr_args("err-memory-hybrid-store", &a))
                })?;
            Ok((Arc::new(store), None))
        }
        "pipeline" => {
            let sled_path = sibling_pipeline_sled_path(&config.memory.path);
            let buffer_wal = if config.memory.pipeline.buffer_wal_enabled {
                Some(sibling_pipeline_buffer_wal_path(&sled_path))
            } else {
                None
            };
            let legacy = if config.memory.pipeline.merge_legacy_file_recall {
                Some(Arc::new(open_file_memory_store(
                    config.memory.path.clone(),
                )?))
            } else {
                None
            };
            let (vector, embedding): (
                Arc<dyn VectorMemoryBackend>,
                Option<Arc<dyn EmbeddingProvider>>,
            ) = if config.memory.pipeline.embedding_enabled {
                let vec_path = sibling_pipeline_vector_sled_path(&sled_path);
                let v = Arc::new(
                    SledVectorBackend::new(vec_path)
                        .map_err(|e| anyhow::anyhow!("pipeline vector sled: {}", e))?,
                ) as Arc<dyn VectorMemoryBackend>;
                if config.memory.embedding_provider == "local" {
                    #[cfg(feature = "embedding-local")]
                    {
                        if std::env::var_os("HF_ENDPOINT").is_none() {
                            if let Some(ref ep) = config.memory.embedding_hf_endpoint {
                                let t = ep.trim();
                                if !t.is_empty() {
                                    std::env::set_var("HF_ENDPOINT", t);
                                }
                            }
                        }
                        let emb = FastEmbedEmbeddingProvider::try_new(
                            config.memory.embedding_local_cache_dir.clone(),
                            config.memory.embedding_local_model.clone(),
                        )
                        .map_err(|e| anyhow::anyhow!("local embedding init: {}", e))?;
                        (v, Some(Arc::new(emb) as Arc<dyn EmbeddingProvider>))
                    }
                    #[cfg(not(feature = "embedding-local"))]
                    {
                        anyhow::bail!(
                            "memory.pipeline.embedding_provider is \"local\" but this build lacks the `embedding-local` feature. Rebuild with: cargo build -p anycode --features embedding-local"
                        );
                    }
                } else {
                    let base_url = config
                        .memory
                        .embedding_base_url
                        .clone()
                        .or_else(|| config.llm.base_url.clone())
                        .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
                    let model = config
                        .memory
                        .embedding_model
                        .clone()
                        .unwrap_or_else(|| "text-embedding-3-small".to_string());
                    let key = config.llm.api_key.trim();
                    if key.is_empty() {
                        tracing::warn!(
                            target: "anycode_cli",
                            "memory.pipeline.embedding_enabled (http) but llm api_key empty; embeddings disabled"
                        );
                        (
                            Arc::new(NoopVectorBackend) as Arc<dyn VectorMemoryBackend>,
                            None,
                        )
                    } else {
                        let emb = Arc::new(OpenAiCompatibleEmbeddingProvider::new(
                            base_url,
                            key.to_string(),
                            model,
                        ));
                        (v, Some(emb as Arc<dyn EmbeddingProvider>))
                    }
                }
            } else {
                (
                    Arc::new(NoopVectorBackend) as Arc<dyn VectorMemoryBackend>,
                    None,
                )
            };
            let pipe = Arc::new(RootReturnMemoryPipeline::open(
                config.memory.pipeline.clone(),
                sled_path,
                buffer_wal,
                legacy,
                vector,
                embedding,
            )?);
            let pipeline_iface: Arc<dyn MemoryPipeline> = pipe.clone();
            let store_iface: Arc<dyn MemoryStore> = pipe;
            Ok((store_iface, Some(pipeline_iface)))
        }
        other => {
            let mut a = FluentArgs::new();
            a.set("b", other.to_string());
            anyhow::bail!("{}", tr_args("log-memory-backend-internal", &a))
        }
    }
}

/// Default LLM config + per-agent overrides (before summary/workspace-assistant/goal fill-ins).
pub(crate) fn build_model_routing_parts(
    config: &Config,
) -> (ModelConfig, HashMap<AgentType, ModelConfig>) {
    let g_norm = normalize_provider_id(&config.llm.provider);
    let default_base_url = if g_norm == "z.ai" {
        config
            .llm
            .base_url
            .clone()
            .or_else(|| Some(default_base_url_for(config.llm.plan.as_str()).to_string()))
    } else {
        config.llm.base_url.clone()
    };

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
        let eff_p = effective_provider(&config.llm.provider, Some(profile));
        let resolved_model = profile
            .model
            .clone()
            .unwrap_or_else(|| config.llm.model.clone());
        let resolved_temperature = profile.temperature.or(Some(config.llm.temperature));
        let resolved_max_tokens = profile.max_tokens.or(Some(config.llm.max_tokens));
        let resolved_base_url = resolve_agent_base_url(config, profile, &default_base_url);
        let api_key = resolve_profile_api_key(config, profile, &eff_p);
        model_overrides.insert(
            AgentType::new(agent_type.clone()),
            ModelConfig {
                provider: LLMProvider::Custom(eff_p),
                model: resolved_model,
                base_url: resolved_base_url,
                temperature: resolved_temperature,
                max_tokens: resolved_max_tokens,
                api_key,
            },
        );
    }

    (default_model_config, model_overrides)
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
