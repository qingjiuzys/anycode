//! Persist inline vision payloads for web-chat stdin protocol.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionImagePayload {
    pub mime_type: String,
    pub data_base64: String,
}

const MAX_IMAGES: usize = 3;
const MAX_IMAGE_BYTES: usize = 4 * 1024 * 1024;

pub fn validate_vision_payloads(images: &[VisionImagePayload]) -> Result<()> {
    if images.len() > MAX_IMAGES {
        bail!("at most {MAX_IMAGES} vision images per message");
    }
    use base64::Engine;
    for (i, img) in images.iter().enumerate() {
        if img.mime_type.trim().is_empty() {
            bail!("vision image {i}: mime_type is required");
        }
        if img.data_base64.trim().is_empty() {
            bail!("vision image {i}: data_base64 is required");
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(img.data_base64.trim())
            .map_err(|e| anyhow::anyhow!("vision image {i}: invalid base64: {e}"))?;
        if bytes.len() > MAX_IMAGE_BYTES {
            bail!(
                "vision image {i}: exceeds {} MB limit",
                MAX_IMAGE_BYTES / (1024 * 1024)
            );
        }
    }
    Ok(())
}

pub fn write_vision_payload(
    session_id: &str,
    images: &[VisionImagePayload],
) -> Result<Option<PathBuf>> {
    if images.is_empty() {
        return Ok(None);
    }
    validate_vision_payloads(images)?;
    let dir = crate::cancel_ipc::dashboard_state_dir().join("vision-payload");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{session_id}-{}.json", Uuid::new_v4().simple()));
    std::fs::write(&path, serde_json::to_vec(images)?)?;
    Ok(Some(path))
}

pub fn vision_file_line(path: &Path) -> String {
    format!("@anycode/vision-file:{}\n", path.display())
}

/// Convert API payloads to core [`VisionImage`] values for embedded chat metadata.
pub fn to_core_images(images: &[VisionImagePayload]) -> Vec<anycode_core::VisionImage> {
    images
        .iter()
        .map(|img| anycode_core::VisionImage::new(img.mime_type.clone(), img.data_base64.clone()))
        .collect()
}

/// Heuristic when registry capabilities omit vision (legacy / catalog gap).
/// Keep in sync with `dashboard-ui` `modelLikelySupportsVision`.
fn model_likely_supports_vision(model_id: &str) -> bool {
    let mid = model_id.trim().to_lowercase();
    if mid.is_empty() {
        return false;
    }
    if mid.contains("whisper")
        || mid.contains("embed")
        || mid.contains("dall-e")
        || mid.contains("tts")
        || mid.contains("speech")
    {
        return false;
    }
    mid.contains("vision")
        || mid.contains("gemini")
        || mid.contains("gpt-4")
        || mid.contains("gpt-4o")
        || mid.contains("gpt-5")
        || mid.contains("claude-3")
        || mid.contains("claude-4")
        || mid.contains("claude-sonnet")
        || mid.contains("claude-opus")
        || mid.contains("claude-haiku")
        || mid.contains("qwen-vl")
        || mid.contains("qwen2-vl")
        || mid.contains("qwen3-vl")
        || mid.contains("deepseek-vl")
        || mid.contains("llava")
        || mid.contains("kimi")
        || mid.contains("moonshot")
        || mid.contains("glm-4v")
        || mid.contains("yi-vision")
        || mid.contains("-vl-")
        || mid.contains("vl-")
        || mid == "agnes-chat"
}

/// Whether the active chat model advertises vision / multimodal input.
pub fn active_chat_supports_vision() -> anyhow::Result<bool> {
    use anycode_llm::capability_catalog::ModelCapability;
    use anycode_llm::ResolvedModelRegistry;
    let (_, cfg) = crate::config_patch::read_config_value(None)?;
    let registry = ResolvedModelRegistry::from_config(&cfg);
    Ok(registry
        .active_item(ModelCapability::Chat)
        .is_some_and(|item| {
            item.capabilities.contains(&ModelCapability::Vision)
                || model_likely_supports_vision(&item.model)
        }))
}

/// Whether Apple OCR (or equivalent local helper) can extract text from images.
/// Chat may be text-only; OCR is a delegated capability slot.
pub fn ocr_fallback_available() -> bool {
    use anycode_llm::media::apple_media::{self, NO_EXTRA_PATHS};
    if !apple_media::apple_media_available() {
        return false;
    }
    apple_media::query_capabilities(NO_EXTRA_PATHS)
        .map(|c| c.ocr)
        .unwrap_or(false)
}

/// Accept images when chat has vision **or** OCR fallback can serve the brain.
pub fn can_accept_images_for_chat() -> anyhow::Result<bool> {
    Ok(active_chat_supports_vision()? || ocr_fallback_available())
}

/// How attached images should be delivered to the chat brain.
#[derive(Debug, Clone)]
pub enum VisionDelivery {
    /// Pass images through as multimodal content (chat supports vision).
    Native,
    /// OCR text to append to the user prompt; do not send raw images to chat.
    OcrText(String),
}

/// Resolve image delivery for the active chat model.
/// Text-only brains (e.g. DeepSeek Flash) get OCR text instead of a hard reject.
pub fn resolve_vision_delivery(images: &[VisionImagePayload]) -> Result<VisionDelivery> {
    if images.is_empty() {
        return Ok(VisionDelivery::Native);
    }
    validate_vision_payloads(images)?;
    if active_chat_supports_vision()? {
        return Ok(VisionDelivery::Native);
    }
    if !ocr_fallback_available() {
        bail!(
            "Active chat model does not support vision, and OCR is unavailable. \
             Enable Apple OCR (Desktop) or switch chat to a vision-capable model."
        );
    }
    let text = ocr_images_to_text(images)?;
    if text.trim().is_empty() {
        bail!("OCR ran but extracted no text from the attached images");
    }
    Ok(VisionDelivery::OcrText(text))
}

/// Run local OCR on each image and return a prompt appendix for the chat brain.
pub fn ocr_images_to_text(images: &[VisionImagePayload]) -> Result<String> {
    use anycode_llm::media::apple_media::{self, NO_EXTRA_PATHS};
    use base64::Engine;
    validate_vision_payloads(images)?;
    let mut parts = Vec::with_capacity(images.len());
    for (i, img) in images.iter().enumerate() {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(img.data_base64.trim())
            .map_err(|e| anyhow::anyhow!("vision image {i}: invalid base64: {e}"))?;
        let text = apple_media::ocr_image_bytes(NO_EXTRA_PATHS, &img.mime_type, &bytes, None)
            .filter(|t| !t.trim().is_empty())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "OCR failed or returned empty text for image {} (mime={})",
                    i + 1,
                    img.mime_type
                )
            })?;
        parts.push(format!("[image {}]\n{}", i + 1, text.trim()));
    }
    Ok(parts.join("\n\n"))
}

pub fn append_ocr_to_prompt(prompt: &str, ocr_text: &str) -> String {
    let mut out = prompt.to_string();
    if !out.is_empty() {
        out.push_str("\n\n");
    }
    out.push_str(
        "--- OCR from attached images (chat model is text-only; OCR capability used) ---\n",
    );
    out.push_str(ocr_text.trim());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    #[test]
    fn rejects_oversized_payload() {
        let huge = "a".repeat(MAX_IMAGE_BYTES + 1);
        use base64::engine::general_purpose::STANDARD;
        let encoded = STANDARD.encode(huge.as_bytes());
        let err = validate_vision_payloads(&[VisionImagePayload {
            mime_type: "image/png".into(),
            data_base64: encoded,
        }])
        .unwrap_err();
        assert!(err.to_string().contains("limit"));
    }

    #[test]
    fn to_core_images_maps_payloads() {
        let core = to_core_images(&[VisionImagePayload {
            mime_type: "image/jpeg".into(),
            data_base64: "abc123".into(),
        }]);
        assert_eq!(core.len(), 1);
        assert_eq!(core[0].mime_type, "image/jpeg");
        assert_eq!(core[0].data_base64, "abc123");
    }

    #[test]
    fn model_likely_supports_vision_matches_common_multimodal_ids() {
        assert!(model_likely_supports_vision("gemini-2.0-flash"));
        assert!(model_likely_supports_vision("agnes-chat"));
        assert!(model_likely_supports_vision("llava"));
        assert!(!model_likely_supports_vision("deepseek-v4-flash"));
        assert!(!model_likely_supports_vision("text-embedding-3-small"));
    }

    #[test]
    fn append_ocr_to_prompt_labels_capability_path() {
        let out = append_ocr_to_prompt("请看图", "[image 1]\n你好");
        assert!(out.contains("请看图"));
        assert!(out.contains("OCR from attached images"));
        assert!(out.contains("你好"));
    }
}
