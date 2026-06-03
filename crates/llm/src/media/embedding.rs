//! Embeddings via OpenAI-compatible `embeddings`.

use crate::media::http::{bearer_headers, http_client, openai_base};
use crate::media::MediaProfile;
use anycode_core::CoreError;

pub struct EmbeddingClient {
    profile: MediaProfile,
}

impl EmbeddingClient {
    pub fn new(profile: MediaProfile) -> Self {
        Self { profile }
    }

    pub async fn embed(&self, input: &str) -> Result<Vec<f32>, CoreError> {
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
