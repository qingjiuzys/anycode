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
}
