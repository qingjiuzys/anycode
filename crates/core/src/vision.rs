//! Vision / multimodal user attachments for LLM requests.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// User `Message.metadata` key: JSON array of [`VisionImage`].
pub const ANYCODE_VISION_IMAGES_METADATA_KEY: &str = "anycode_vision_images";

/// Inline image for chat-completions / Anthropic vision APIs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VisionImage {
    pub mime_type: String,
    pub data_base64: String,
}

impl VisionImage {
    #[must_use]
    pub fn new(mime_type: impl Into<String>, data_base64: impl Into<String>) -> Self {
        Self {
            mime_type: mime_type.into(),
            data_base64: data_base64.into(),
        }
    }
}

#[must_use]
pub fn vision_images_from_metadata(metadata: &HashMap<String, Value>) -> Vec<VisionImage> {
    metadata
        .get(ANYCODE_VISION_IMAGES_METADATA_KEY)
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default()
}

pub fn attach_vision_images(metadata: &mut HashMap<String, Value>, images: &[VisionImage]) {
    if images.is_empty() {
        metadata.remove(ANYCODE_VISION_IMAGES_METADATA_KEY);
    } else if let Ok(v) = serde_json::to_value(images) {
        metadata.insert(ANYCODE_VISION_IMAGES_METADATA_KEY.to_string(), v);
    }
}
