//! Dashboard / pipe REPL vision injection (`@anycode/vision-file:` line protocol).

use anycode_core::{attach_vision_images, Message, MessageContent, MessageRole, VisionImage};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const VISION_FILE_LINE_PREFIX: &str = "@anycode/vision-file:";

#[must_use]
pub fn parse_vision_file_line(line: &str) -> Option<PathBuf> {
    line.trim()
        .strip_prefix(VISION_FILE_LINE_PREFIX)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}

pub fn load_vision_file(path: &Path) -> anyhow::Result<Vec<VisionImage>> {
    let raw = std::fs::read_to_string(path)?;
    let images: Vec<VisionImage> = serde_json::from_str(&raw)?;
    Ok(images)
}

pub fn remove_vision_file(path: &Path) {
    let _ = std::fs::remove_file(path);
}

pub fn user_message_with_vision(prompt: impl Into<String>, vision: &[VisionImage]) -> Message {
    let mut metadata = HashMap::new();
    if !vision.is_empty() {
        attach_vision_images(&mut metadata, vision);
    }
    Message {
        id: uuid::Uuid::new_v4(),
        role: MessageRole::User,
        content: MessageContent::Text(prompt.into()),
        timestamp: chrono::Utc::now(),
        metadata,
    }
}
