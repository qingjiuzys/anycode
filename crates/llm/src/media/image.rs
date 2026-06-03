//! Image generation via OpenAI-compatible `images/generations`.

use crate::media::http::{bearer_headers, http_client, openai_base};
use crate::media::MediaProfile;
use anycode_core::CoreError;

#[derive(Debug, Clone)]
pub struct ImageGenResult {
    pub url: Option<String>,
    pub b64_json: Option<String>,
}

pub struct ImageGenClient {
    profile: MediaProfile,
}

impl ImageGenClient {
    pub fn new(profile: MediaProfile) -> Self {
        Self { profile }
    }

    pub async fn generate(&self, prompt: &str) -> Result<ImageGenResult, CoreError> {
        let base = openai_base(&self.profile)?;
        let url = format!("{}/images/generations", base.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": self.profile.model,
            "prompt": prompt,
            "n": 1,
            "size": "1024x1024"
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
                "image gen failed status={} body={}",
                status, text
            )));
        }
        let v: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| CoreError::LLMError(e.to_string()))?;
        let first = v
            .pointer("/data/0")
            .ok_or_else(|| CoreError::LLMError("image gen: empty data".into()))?;
        Ok(ImageGenResult {
            url: first
                .get("url")
                .and_then(|u| u.as_str())
                .map(str::to_string),
            b64_json: first
                .get("b64_json")
                .and_then(|u| u.as_str())
                .map(str::to_string),
        })
    }
}
