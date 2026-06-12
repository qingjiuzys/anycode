//! Embeddings via OpenAI-compatible HTTP or on-device FastEmbed.

use crate::local_media_catalog::is_builtin_local_provider;
use crate::media::http::{bearer_headers, http_client, openai_base};
use crate::media::MediaProfile;
use anycode_core::CoreError;

#[cfg(feature = "embedding-local")]
use anycode_memory::FastEmbedEmbeddingProvider;
#[cfg(feature = "embedding-local")]
use std::path::PathBuf;
#[cfg(feature = "embedding-local")]
use std::sync::{Arc, Mutex, OnceLock};

pub struct EmbeddingClient {
    profile: MediaProfile,
    #[cfg(feature = "embedding-local")]
    local: Option<Arc<FastEmbedEmbeddingProvider>>,
}

impl EmbeddingClient {
    pub fn new(profile: MediaProfile) -> Self {
        #[cfg(feature = "embedding-local")]
        let local = if is_builtin_local_provider(&profile.provider) {
            FastEmbedEmbeddingProvider::try_new(None, Some(profile.model.clone()))
                .ok()
                .map(Arc::new)
        } else {
            None
        };
        Self {
            profile,
            #[cfg(feature = "embedding-local")]
            local,
        }
    }

    pub async fn embed(&self, input: &str) -> Result<Vec<f32>, CoreError> {
        if is_builtin_local_provider(&self.profile.provider) {
            return self.embed_local(input).await;
        }
        self.embed_http(input).await
    }

    #[cfg(feature = "embedding-local")]
    async fn embed_local(&self, input: &str) -> Result<Vec<f32>, CoreError> {
        let provider = self.local.as_ref().ok_or_else(|| {
            CoreError::LLMError(
                "local_fastembed requires build with --features embedding-local (or media-local)"
                    .into(),
            )
        })?;
        provider.embed_one(input).await
    }

    #[cfg(not(feature = "embedding-local"))]
    async fn embed_local(&self, _input: &str) -> Result<Vec<f32>, CoreError> {
        Err(CoreError::LLMError(
            "local_fastembed requires build with --features embedding-local (or media-local)"
                .into(),
        ))
    }

    async fn embed_http(&self, input: &str) -> Result<Vec<f32>, CoreError> {
        let base = openai_base(&self.profile)?;
        let url = format!("{}/embeddings", base.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": self.profile.model,
            "input": input
        });
        let resp = http_client()
            .post(url)
            .headers(bearer_headers(&self.profile))
            .json(&body)
            .send()
            .await
            .map_err(|e| CoreError::LLMError(e.to_string()))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| CoreError::LLMError(e.to_string()))?;
        if !status.is_success() {
            return Err(CoreError::LLMError(format!(
                "embedding failed status={} body={}",
                status, text
            )));
        }
        let v: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| CoreError::LLMError(e.to_string()))?;
        let arr = v
            .pointer("/data/0/embedding")
            .and_then(|e| e.as_array())
            .ok_or_else(|| CoreError::LLMError("embedding: missing vector".into()))?;
        Ok(arr
            .iter()
            .filter_map(|x| x.as_f64().map(|f| f as f32))
            .collect())
    }
}

/// Shared process-wide FastEmbed instance for memory + media (same model id).
#[cfg(feature = "embedding-local")]
pub fn shared_fastembed(
    cache_dir: Option<PathBuf>,
    model_id: Option<String>,
) -> Result<Arc<FastEmbedEmbeddingProvider>, CoreError> {
    static CACHE: OnceLock<Mutex<Option<(String, Arc<FastEmbedEmbeddingProvider>)>>> =
        OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(None));
    let key = format!(
        "{}|{}",
        cache_dir
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
        model_id.clone().unwrap_or_default()
    );
    let mut guard = cache.lock().expect("fastembed cache mutex");
    if let Some((k, prov)) = guard.as_ref() {
        if k == &key {
            return Ok(prov.clone());
        }
    }
    let prov = Arc::new(
        FastEmbedEmbeddingProvider::try_new(cache_dir, model_id)
            .map_err(|e| CoreError::LLMError(format!("fastembed init: {e}")))?,
    );
    *guard = Some((key, prov.clone()));
    Ok(prov)
}
