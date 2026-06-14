//! Text-to-speech via OpenAI-compatible HTTP or on-device Piper.

use crate::local_media_catalog::is_builtin_local_provider;
use crate::media::apple_media::{is_apple_tts_provider, synthesize_apple_tts};
use crate::media::http::{bearer_headers, http_client, openai_base, resolve_tts_voice};
use crate::media::tts_local::synthesize_local;
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
        if is_apple_tts_provider(&self.profile.provider) {
            let voice = resolve_tts_voice(&self.profile);
            let locale = self
                .profile
                .extra_headers
                .as_ref()
                .and_then(|h| h.get("locale"))
                .map(|s| s.as_str())
                .unwrap_or("zh-CN");
            let audio_bytes = synthesize_apple_tts(text, Some(&voice), locale).await?;
            return Ok(TtsResult {
                audio_bytes,
                content_type: "audio/wav".to_string(),
            });
        }
        if is_builtin_local_provider(&self.profile.provider) {
            let voice = resolve_tts_voice(&self.profile);
            let audio_bytes = synthesize_local(&voice, text).await?;
            return Ok(TtsResult {
                audio_bytes,
                content_type: "audio/wav".to_string(),
            });
        }
        let base = openai_base(&self.profile)?;
        let url = format!("{}/audio/speech", base.trim_end_matches('/'));
        let voice = resolve_tts_voice(&self.profile);
        let body = serde_json::json!({
            "model": self.profile.model,
            "input": text,
            "voice": voice
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
