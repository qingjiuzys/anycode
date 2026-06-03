//! Text-to-speech via OpenAI-compatible `audio/speech`.

use crate::media::http::{bearer_headers, http_client, openai_base};
use crate::media::MediaProfile;
use anycode_core::CoreError;

#[derive(Debug, Clone)]
pub struct TtsResult {
    pub audio_bytes: Vec<u8>,
    pub content_type: String,
}

pub struct TtsClient {
    profile: MediaProfile,
}

impl TtsClient {
    pub fn new(profile: MediaProfile) -> Self {
        Self { profile }
    }

    pub async fn synthesize(&self, text: &str) -> Result<TtsResult, CoreError> {
        let base = openai_base(&self.profile)?;
        let url = format!("{}/audio/speech", base.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": self.profile.model,
            "input": text,
            "voice": "alloy"
        });
        let resp = http_client()
            .post(url)
            .headers(bearer_headers(&self.profile))
            .json(&body)
            .send()
            .await
            .map_err(|e| CoreError::LLMError(e.to_string()))?;
        let status = resp.status();
        if !status.is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(CoreError::LLMError(format!(
                "TTS failed status={} body={}",
                status, err
            )));
        }
        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("audio/mpeg")
            .to_string();
        let audio_bytes = resp
            .bytes()
            .await
            .map_err(|e| CoreError::LLMError(e.to_string()))?
            .to_vec();
        Ok(TtsResult {
            audio_bytes,
            content_type,
        })
    }
}
