//! 本地向量：ONNX Runtime（经 [fastembed]，`all-MiniLM-L6-v2`），无 HTTP。
//!
//! 需在 `Cargo.toml` 启用 `anycode-memory` 的 `embedding-local` feature。

use crate::MemoryError;
use anycode_core::prelude::*;
use anycode_core::EmbeddingProvider;
use async_trait::async_trait;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// 与 OpenAI HTTP 嵌入并列的本地实现：首次运行会从 Hugging Face 拉取 ONNX（可配置缓存目录）。
pub struct FastEmbedEmbeddingProvider {
    inner: Arc<Mutex<TextEmbedding>>,
}

impl FastEmbedEmbeddingProvider {
    /// `cache_dir` 为 `None` 时使用 fastembed 默认缓存目录（通常为 `~/.cache/fastembed`）。
    ///
    /// `model_id` 为 fastembed 枚举名（与 `EmbeddingModel` 的 `Debug` 一致，不区分大小写），如 `AllMiniLML6V2`、`BGESmallZHV15`；`None` 等价于 `AllMiniLML6V2`。
    pub fn try_new(
        cache_dir: Option<PathBuf>,
        model_id: Option<String>,
    ) -> Result<Self, MemoryError> {
        let model = match model_id
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            Some(s) => s
                .parse::<EmbeddingModel>()
                .map_err(|e| MemoryError::EmbeddingInit(e.to_string()))?,
            None => EmbeddingModel::AllMiniLML6V2,
        };
        let mut opts = InitOptions::new(model).with_show_download_progress(false);
        if let Some(p) = cache_dir {
            opts = opts.with_cache_dir(p);
        }
        let model =
            TextEmbedding::try_new(opts).map_err(|e| MemoryError::EmbeddingInit(e.to_string()))?;
        Ok(Self {
            inner: Arc::new(Mutex::new(model)),
        })
    }
}

#[async_trait]
impl EmbeddingProvider for FastEmbedEmbeddingProvider {
    async fn embed_one(&self, text: &str) -> Result<Vec<f32>, CoreError> {
        let text = text.to_string();
        let inner = self.inner.clone();
        let vecs = tokio::task::spawn_blocking(move || {
            let mut rt = inner.lock().expect("fastembed mutex poisoned");
            rt.embed(vec![text], None)
        })
        .await
        .map_err(|e| CoreError::Other(anyhow::anyhow!("embed join: {}", e)))?
        .map_err(|e| CoreError::Other(anyhow::anyhow!("fastembed: {}", e)))?;
        vecs.into_iter()
            .next()
            .ok_or_else(|| CoreError::Other(anyhow::anyhow!("empty embedding")))
    }
}

#[cfg(all(test, feature = "embedding-local"))]
mod tests {
    use super::EmbeddingModel;

    #[test]
    fn embedding_model_from_str_case_insensitive() {
        let a: EmbeddingModel = "allminilmL6v2".parse().expect("parse");
        let b: EmbeddingModel = "AllMiniLML6V2".parse().expect("parse");
        assert_eq!(format!("{:?}", a), format!("{:?}", b));
    }

    #[test]
    fn embedding_model_invalid() {
        assert!("NotARealModel".parse::<EmbeddingModel>().is_err());
    }
}
