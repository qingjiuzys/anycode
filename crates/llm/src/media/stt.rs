//! Speech-to-text via OpenAI-compatible HTTP or on-device whisper.cpp.

use crate::local_media_catalog::is_builtin_local_provider;
use crate::media::apple_media::{is_apple_speech_provider, transcribe_apple_speech};
use crate::media::http::{bearer_headers, http_client, openai_base};
use crate::media::stt_local::{transcribe_pcm, wav_bytes_to_pcm16k};
use crate::media::MediaProfile;
use anycode_core::CoreError;
use reqwest::multipart;

#[derive(Debug, Clone)]
pub struct SttResult {
    pub text: String,
}

pub struct SttClient {
    profile: MediaProfile,
}

impl SttClient {
    pub fn new(profile: MediaProfile) -> Self {
        Self { profile }
    }

    pub async fn transcribe(
        &self,
        audio_bytes: &[u8],
        filename: &str,
    ) -> Result<SttResult, CoreError> {
        if is_apple_speech_provider(&self.profile.provider) {
            let locale = self
                .profile
                .extra_headers
                .as_ref()
                .and_then(|h| h.get("locale"))
                .map(|s| s.as_str())
                .unwrap_or("zh-CN");
            let text = transcribe_apple_speech(audio_bytes, filename, locale).await?;
            return Ok(SttResult { text });
        }
        if is_builtin_local_provider(&self.profile.provider) {
            let pcm = wav_bytes_to_pcm16k(audio_bytes)?;
            let text = transcribe_pcm(&self.profile.model, &pcm).await?;
            return Ok(SttResult { text });
        }
        self.transcribe_http(audio_bytes, filename).await
    }

    async fn transcribe_http(
        &self,
        audio_bytes: &[u8],
        filename: &str,
    ) -> Result<SttResult, CoreError> {
        let base = openai_base(&self.profile)?;
        let url = format!("{}/audio/transcriptions", base.trim_end_matches('/'));
        let part = multipart::Part::bytes(audio_bytes.to_vec())
            .file_name(filename.to_string())
            .mime_str("application/octet-stream")
            .map_err(|e| CoreError::LLMError(e.to_string()))?;
        let form = multipart::Form::new()
            .text("model", self.profile.model.clone())
            .part("file", part);
        let resp = http_client()
            .post(url)
            .headers(bearer_headers(&self.profile))
            .multipart(form)
            .send()
            .await
            .map_err(|e| CoreError::LLMError(e.to_string()))?;
        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| CoreError::LLMError(e.to_string()))?;
        if !status.is_success() {
            return Err(CoreError::LLMError(format!(
                "STT failed status={} body={}",
                status, body
            )));
        }
        let v: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| CoreError::LLMError(e.to_string()))?;
        let text = v
            .get("text")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        Ok(SttResult { text })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability_catalog::ModelCapability;

    #[tokio::test]
    async fn rejects_missing_base_url() {
        let client = SttClient::new(MediaProfile {
            capability: ModelCapability::Stt,
            provider: "custom".into(),
            model: "whisper-1".into(),
            api_key: "sk-x".into(),
            base_url: None,
            extra_headers: None,
            endpoint_overrides: None,
        });
        assert!(client.transcribe(b"abc", "a.wav").await.is_err());
    }

    #[test]
    fn apple_speech_provider_detected() {
        assert!(crate::media::apple_media::is_apple_speech_provider(
            "apple_speech"
        ));
        assert!(!crate::media::apple_media::is_apple_speech_provider(
            "whisper_cpp"
        ));
    }
}
