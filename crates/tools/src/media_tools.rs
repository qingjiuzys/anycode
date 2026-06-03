//! Multimodal generation tools (STT/TTS/image/video) backed by injected `MediaClientRegistry`.

use crate::services::ToolServices;
use anycode_core::prelude::*;
use anycode_llm::{
    media::{ImageGenClient, MediaClientRegistry, SttClient, TtsClient, VideoGenClient},
    ModelCapability,
};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use std::time::Instant;

fn resolve_registry(services: &ToolServices) -> Result<MediaClientRegistry, CoreError> {
    services
        .media_registry()
        .map_err(|e| CoreError::ConfigError(e))
}

macro_rules! media_tool_boilerplate {
    () => {
        fn permission_mode(&self) -> PermissionMode {
            PermissionMode::Auto
        }

        fn security_policy(&self) -> Option<&SecurityPolicy> {
            None
        }
    };
}

macro_rules! media_tool_struct {
    ($name:ident) => {
        pub struct $name {
            services: Arc<ToolServices>,
        }

        impl $name {
            pub fn new(services: Arc<ToolServices>) -> Self {
                Self { services }
            }
        }
    };
}

media_tool_struct!(SpeechToTextTool);
media_tool_struct!(TextToSpeechTool);
media_tool_struct!(GenerateImageTool);
media_tool_struct!(GenerateVideoTool);

#[async_trait]
impl Tool for SpeechToTextTool {
    fn name(&self) -> &str {
        "SpeechToText"
    }

    fn description(&self) -> &str {
        "Transcribe audio bytes (base64) to text using models.speech.stt"
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "audio_base64": { "type": "string" },
                "filename": { "type": "string", "default": "audio.wav" }
            },
            "required": ["audio_base64"]
        })
    }

    media_tool_boilerplate!();

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let start = Instant::now();
        let b64 = input
            .input
            .get("audio_base64")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::ConfigError("audio_base64 required".into()))?;
        let filename = input
            .input
            .get("filename")
            .and_then(|v| v.as_str())
            .unwrap_or("audio.wav");
        let bytes = base64_decode(b64)?;
        let reg = resolve_registry(&self.services)?;
        let prof = reg
            .profile_for(ModelCapability::Stt)
            .ok_or_else(|| CoreError::ConfigError("models.speech.stt not configured".into()))?;
        let client = SttClient::new(prof.profile.clone());
        let out = client.transcribe(&bytes, filename).await?;
        Ok(ToolOutput {
            result: json!({ "text": out.text }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[async_trait]
impl Tool for TextToSpeechTool {
    fn name(&self) -> &str {
        "TextToSpeech"
    }

    fn description(&self) -> &str {
        "Synthesize speech from text using models.speech.tts"
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": { "text": { "type": "string" } },
            "required": ["text"]
        })
    }

    media_tool_boilerplate!();

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let start = Instant::now();
        let text = input
            .input
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::ConfigError("text required".into()))?;
        let reg = resolve_registry(&self.services)?;
        let prof = reg
            .profile_for(ModelCapability::Tts)
            .ok_or_else(|| CoreError::ConfigError("models.speech.tts not configured".into()))?;
        let client = TtsClient::new(prof.profile.clone());
        let out = client.synthesize(text).await?;
        Ok(ToolOutput {
            result: json!({
                "content_type": out.content_type,
                "audio_base64": base64_encode(&out.audio_bytes),
            }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[async_trait]
impl Tool for GenerateImageTool {
    fn name(&self) -> &str {
        "GenerateImage"
    }

    fn description(&self) -> &str {
        "Generate an image from a text prompt using models.image"
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": { "prompt": { "type": "string" } },
            "required": ["prompt"]
        })
    }

    media_tool_boilerplate!();

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let start = Instant::now();
        let prompt = input
            .input
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::ConfigError("prompt required".into()))?;
        let reg = resolve_registry(&self.services)?;
        let prof = reg
            .profile_for(ModelCapability::ImageGen)
            .ok_or_else(|| CoreError::ConfigError("models.image not configured".into()))?;
        let client = ImageGenClient::new(prof.profile.clone());
        let out = client.generate(prompt).await?;
        Ok(ToolOutput {
            result: json!({ "url": out.url, "b64_json": out.b64_json }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[async_trait]
impl Tool for GenerateVideoTool {
    fn name(&self) -> &str {
        "GenerateVideo"
    }

    fn description(&self) -> &str {
        "Generate a video from a text prompt using models.video (requires base_url)"
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": { "prompt": { "type": "string" } },
            "required": ["prompt"]
        })
    }

    media_tool_boilerplate!();

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let start = Instant::now();
        let prompt = input
            .input
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::ConfigError("prompt required".into()))?;
        let reg = resolve_registry(&self.services)?;
        let prof = reg
            .profile_for(ModelCapability::VideoGen)
            .ok_or_else(|| CoreError::ConfigError("models.video not configured".into()))?;
        let client = VideoGenClient::new(prof.profile.clone());
        let out = client.generate(prompt).await?;
        Ok(ToolOutput {
            result: json!({ "url": out.url, "job_id": out.job_id, "raw": out.raw }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

fn base64_decode(s: &str) -> Result<Vec<u8>, CoreError> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(s.trim())
        .map_err(|e| CoreError::ConfigError(format!("invalid base64: {e}")))
}

fn base64_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}
