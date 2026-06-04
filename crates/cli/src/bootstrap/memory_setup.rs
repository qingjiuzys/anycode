//! Memory store / pipeline construction for runtime bootstrap.

use crate::app_config::Config;
use crate::i18n::tr_args;
use anycode_core::prelude::*;
use anycode_core::{EmbeddingProvider, MemoryPipeline, VectorMemoryBackend};
#[cfg(feature = "embedding-local")]
use anycode_memory::FastEmbedEmbeddingProvider;
use anycode_memory::{
    FileMemoryStore, HybridMemoryStore, NoopVectorBackend, OpenAiCompatibleEmbeddingProvider,
    RootReturnMemoryPipeline, SledVectorBackend,
};
use async_trait::async_trait;
use fluent_bundle::FluentArgs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// How this process attaches to configured memory (Sled is single-writer per machine).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MemoryAttachMode {
    /// Channel bridges: open hybrid/pipeline sled when configured.
    Exclusive,
    /// Local REPL/run: same Markdown tree as Exclusive; use `file` when config is hybrid/pipeline.
    Shared,
}

impl MemoryAttachMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Exclusive => "exclusive",
            Self::Shared => "shared",
        }
    }
}

/// Effective store backend after attach policy (`config.memory.backend` may stay `hybrid`).
pub(crate) fn effective_memory_backend(config: &Config, attach: MemoryAttachMode) -> &str {
    match attach {
        MemoryAttachMode::Exclusive => config.memory.backend.as_str(),
        MemoryAttachMode::Shared => match config.memory.backend.as_str() {
            "hybrid" | "pipeline" | "layered" | "guigen" => "file",
            other => other,
        },
    }
}

fn resolve_memory_attach(requested: MemoryAttachMode) -> MemoryAttachMode {
    match std::env::var("ANYCODE_MEMORY_ATTACH")
        .ok()
        .map(|s| s.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("exclusive") => MemoryAttachMode::Exclusive,
        Some("shared") => MemoryAttachMode::Shared,
        _ => requested,
    }
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

pub(crate) fn memory_sled_path_for_diagnostics(file_memory_root: &Path) -> PathBuf {
    sibling_sled_path(file_memory_root)
}

fn sibling_sled_path(file_memory_root: &Path) -> PathBuf {
    let name = file_memory_root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("memory");
    let parent = file_memory_root.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!("{}.sled", name))
}

/// 热层 Sled 路径（归根通道 `pipeline` backend，与 `hybrid` 的 sibling sled 命名区分）。
fn sibling_pipeline_sled_path(file_memory_root: &Path) -> PathBuf {
    let name = file_memory_root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("memory");
    let parent = file_memory_root.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!("{}.pipeline.sled", name))
}

fn sibling_pipeline_buffer_wal_path(pipeline_hot_sled: &Path) -> PathBuf {
    let s = pipeline_hot_sled.to_string_lossy();
    if s.ends_with(".sled") {
        PathBuf::from(format!("{}.buffer.wal", s.strip_suffix(".sled").unwrap()))
    } else {
        pipeline_hot_sled.with_extension("buffer.wal")
    }
}

fn sibling_pipeline_vector_sled_path(pipeline_hot_sled: &Path) -> PathBuf {
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
    attach: MemoryAttachMode,
) -> anyhow::Result<(Arc<dyn MemoryStore>, Option<Arc<dyn MemoryPipeline>>)> {
    let attach = resolve_memory_attach(attach);
    match effective_memory_backend(config, attach) {
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
                    let registry_settings =
                        anycode_llm::read_config_value(None)
                            .ok()
                            .and_then(|(_, cfg_json)| {
                                let reg =
                                    anycode_llm::ResolvedModelRegistry::from_config(&cfg_json);
                                reg.active_item(anycode_llm::ModelCapability::Embedding)
                                    .map(|item| {
                                        (
                                            reg.resolve_model(item),
                                            reg.resolve_base_url(item).unwrap_or_else(|| {
                                                "https://api.openai.com/v1".to_string()
                                            }),
                                            reg.resolve_api_key(item),
                                        )
                                    })
                            });
                    let base_url = config
                        .memory
                        .embedding_base_url
                        .clone()
                        .or_else(|| registry_settings.as_ref().map(|(_, u, _)| u.clone()))
                        .or_else(|| config.llm.base_url.clone())
                        .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
                    let model = config
                        .memory
                        .embedding_model
                        .clone()
                        .or_else(|| registry_settings.as_ref().map(|(m, _, _)| m.clone()))
                        .unwrap_or_else(|| "text-embedding-3-small".to_string());
                    let key = registry_settings
                        .as_ref()
                        .and_then(|(_, _, k)| k.clone())
                        .filter(|s| !s.trim().is_empty())
                        .unwrap_or_else(|| config.llm.api_key.trim().to_string());
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
                        let emb =
                            Arc::new(OpenAiCompatibleEmbeddingProvider::new(base_url, key, model));
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

#[cfg(test)]
fn effective_memory_backend_for_test(configured: &str, attach: MemoryAttachMode) -> &str {
    match attach {
        MemoryAttachMode::Exclusive => configured,
        MemoryAttachMode::Shared => match configured {
            "hybrid" | "pipeline" | "layered" | "guigen" => "file",
            other => other,
        },
    }
}

#[cfg(test)]
mod memory_attach_tests {
    use super::MemoryAttachMode;

    #[test]
    fn shared_attach_maps_sled_backends_to_file() {
        for b in ["hybrid", "pipeline", "layered", "guigen"] {
            assert_eq!(
                super::effective_memory_backend_for_test(b, MemoryAttachMode::Shared),
                "file"
            );
            assert_eq!(
                super::effective_memory_backend_for_test(b, MemoryAttachMode::Exclusive),
                b
            );
        }
        assert_eq!(
            super::effective_memory_backend_for_test("noop", MemoryAttachMode::Shared),
            "noop"
        );
    }
}
