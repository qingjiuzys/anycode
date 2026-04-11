//! OpenAI 兼容 `/v1/embeddings` HTTP 客户端。

use anycode_core::prelude::*;
use anycode_core::EmbeddingProvider;
use async_trait::async_trait;
use serde::Deserialize;

#[derive(Deserialize)]
struct EmbeddingsResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

fn normalize_base(base: &str) -> String {
    base.trim_end_matches('/').to_string()
}

pub struct OpenAiCompatibleEmbeddingProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl OpenAiCompatibleEmbeddingProvider {
    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("reqwest client"),
            base_url: normalize_base(&base_url.into()),
            api_key: api_key.into(),
            model: model.into(),
        }
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAiCompatibleEmbeddingProvider {
    async fn embed_one(&self, text: &str) -> Result<Vec<f32>, CoreError> {
        let url = format!("{}/embeddings", self.base_url);
        let body = serde_json::json!({
            "model": self.model,
            "input": text,
        });
        let res = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| CoreError::Other(anyhow::anyhow!("embed http: {}", e)))?;
        if !res.status().is_success() {
            let t = res.text().await.unwrap_or_default();
            return Err(CoreError::Other(anyhow::anyhow!(
                "embeddings status: {}",
                t
            )));
        }
        let parsed: EmbeddingsResponse = res
            .json()
            .await
            .map_err(|e| CoreError::Other(anyhow::anyhow!("embeddings json: {}", e)))?;
        parsed
            .data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .ok_or_else(|| CoreError::Other(anyhow::anyhow!("empty embedding data")))
    }
}
