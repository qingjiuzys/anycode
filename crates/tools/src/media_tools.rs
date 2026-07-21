//! Multimodal generation tools (STT/TTS/image/video) backed by injected `MediaClientRegistry`.

use crate::services::ToolServices;
use anycode_core::prelude::*;
use anycode_llm::{
    media::{ImageGenClient, MediaClientRegistry, SttClient, TtsClient, VideoGenClient},
    ModelCapability,
};
use async_trait::async_trait;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

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
            "properties": {
                "prompt": { "type": "string" },
                "path": {
                    "type": "string",
                    "description": "Optional output path (relative to cwd or absolute). Defaults to generated/image-<ts>.png"
                }
            },
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
        let wd = input.working_directory.as_deref().unwrap_or(".");
        let path_hint = input.input.get("path").and_then(|v| v.as_str());
        let (path, bytes) = persist_image_bytes(wd, path_hint, &out.url, &out.b64_json).await?;
        let path_str = path.display().to_string();
        Ok(ToolOutput {
            result: json!({
                "path": path_str,
                "url": out.url,
                "bytes": bytes,
                "artifacts": [{
                    "path": path_str,
                    "kind": "image",
                    "mime": "image/png",
                    "title": path.file_name().and_then(|s| s.to_str()).unwrap_or("image.png"),
                    "inline": true,
                    "bytes": bytes
                }]
            }),
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
        "Generate a video from a text prompt using models.video (Agnes or other configured provider)"
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "prompt": { "type": "string" },
                "path": {
                    "type": "string",
                    "description": "Optional local output path when a downloadable URL is returned"
                }
            },
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
        let wd = input.working_directory.as_deref().unwrap_or(".");
        let path_hint = input.input.get("path").and_then(|v| v.as_str());
        let mut path_str: Option<String> = None;
        let mut bytes: Option<u64> = None;
        let mut artifacts = Vec::new();
        if let Some(url) = out.url.as_deref() {
            if let Ok((path, len)) = persist_remote_media(wd, path_hint, url, "mp4").await {
                path_str = Some(path.display().to_string());
                bytes = Some(len);
                artifacts.push(json!({
                    "path": path.display().to_string(),
                    "kind": "video",
                    "mime": "video/mp4",
                    "title": path.file_name().and_then(|s| s.to_str()).unwrap_or("video.mp4"),
                    "inline": true,
                    "bytes": len
                }));
            }
        }
        let hint = out
            .url
            .as_ref()
            .map(|url| format!("Video ready. Share this URL with the user: {url}"))
            .or_else(|| {
                out.job_id.as_ref().map(|id| {
                    format!("Video job submitted (job_id={id}). Poll may still be in progress.")
                })
            });
        Ok(ToolOutput {
            result: json!({
                "path": path_str,
                "url": out.url,
                "job_id": out.job_id,
                "bytes": bytes,
                "hint": hint,
                "artifacts": artifacts,
                "raw": out.raw
            }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

fn millis_stamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn resolve_output_path(wd: &str, hint: Option<&str>, default_name: &str) -> PathBuf {
    if let Some(h) = hint.map(str::trim).filter(|s| !s.is_empty()) {
        let p = Path::new(h);
        if p.is_absolute() {
            return p.to_path_buf();
        }
        return Path::new(wd).join(p);
    }
    Path::new(wd).join("generated").join(default_name)
}

async fn persist_image_bytes(
    wd: &str,
    path_hint: Option<&str>,
    url: &Option<String>,
    b64_json: &Option<String>,
) -> Result<(PathBuf, u64), CoreError> {
    let path = resolve_output_path(wd, path_hint, &format!("image-{}.png", millis_stamp()));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            CoreError::IoError(std::io::Error::new(
                e.kind(),
                format!("create image dir: {e}"),
            ))
        })?;
    }
    let bytes = if let Some(b64) = b64_json.as_deref() {
        base64_decode(b64)?
    } else if let Some(u) = url.as_deref() {
        download_bytes(u).await?
    } else {
        return Err(CoreError::LLMError(
            "image gen returned neither b64_json nor url".into(),
        ));
    };
    let len = bytes.len() as u64;
    std::fs::write(&path, &bytes).map_err(CoreError::IoError)?;
    Ok((path, len))
}

async fn persist_remote_media(
    wd: &str,
    path_hint: Option<&str>,
    url: &str,
    ext: &str,
) -> Result<(PathBuf, u64), CoreError> {
    let path = resolve_output_path(wd, path_hint, &format!("video-{}.{ext}", millis_stamp()));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            CoreError::IoError(std::io::Error::new(
                e.kind(),
                format!("create media dir: {e}"),
            ))
        })?;
    }
    let bytes = download_bytes(url).await?;
    let len = bytes.len() as u64;
    std::fs::write(&path, &bytes).map_err(CoreError::IoError)?;
    Ok((path, len))
}

async fn download_bytes(url: &str) -> Result<Vec<u8>, CoreError> {
    let resp = reqwest::Client::new()
        .get(url)
        .send()
        .await
        .map_err(|e| CoreError::LLMError(format!("download media: {e}")))?;
    if !resp.status().is_success() {
        return Err(CoreError::LLMError(format!(
            "download media status={}",
            resp.status()
        )));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| CoreError::LLMError(format!("download media body: {e}")))?;
    Ok(bytes.to_vec())
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
