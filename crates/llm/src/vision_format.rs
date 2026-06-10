//! Map user messages with vision metadata to provider-specific multimodal payloads.

use anycode_core::{vision_images_from_metadata, Message, MessageRole};
use serde_json::{json, Value};

pub fn openai_user_content(msg: &Message, text: &str) -> Value {
    let images = vision_images_from_metadata(&msg.metadata);
    if images.is_empty() {
        return json!(text);
    }
    let mut parts: Vec<Value> = Vec::new();
    if !text.is_empty() {
        parts.push(json!({ "type": "text", "text": text }));
    }
    for img in images {
        parts.push(json!({
            "type": "image_url",
            "image_url": {
                "url": format!("data:{};base64,{}", img.mime_type, img.data_base64)
            }
        }));
    }
    if parts.is_empty() {
        json!(text)
    } else {
        Value::Array(parts)
    }
}

#[allow(dead_code)]
pub fn anthropic_user_blocks(msg: &Message, text: &str) -> Vec<Value> {
    let mut blocks: Vec<Value> = Vec::new();
    if !text.is_empty() {
        blocks.push(json!({ "type": "text", "text": text }));
    }
    for img in vision_images_from_metadata(&msg.metadata) {
        blocks.push(json!({
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": img.mime_type,
                "data": img.data_base64
            }
        }));
    }
    if blocks.is_empty() && msg.role == MessageRole::User {
        blocks.push(json!({ "type": "text", "text": text }));
    }
    blocks
}
