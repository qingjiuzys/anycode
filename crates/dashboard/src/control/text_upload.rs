//! Persist text reference uploads for web-chat stdin protocol.

use anyhow::{bail, Result};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextFilePayload {
    pub filename: String,
    pub content: String,
}

const MAX_FILES: usize = 3;
const MAX_FILE_BYTES: usize = 1024 * 1024;
const ALLOWED_EXTENSIONS: &[&str] = &["txt", "md", "json", "csv", "log", "pdf"];

pub fn validate_text_payloads(files: &[TextFilePayload]) -> Result<()> {
    if files.len() > MAX_FILES {
        bail!("at most {MAX_FILES} text files per message");
    }
    for (i, f) in files.iter().enumerate() {
        let name = f.filename.trim();
        if name.is_empty() {
            bail!("text file {i}: filename is required");
        }
        let ext = Path::new(name)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();
        if !ALLOWED_EXTENSIONS.contains(&ext.as_str()) {
            bail!("text file {i}: unsupported type (allowed: .txt .md .json .csv .log .pdf)");
        }
        let text = normalize_upload_content(&f.filename, &f.content)?;
        if text.trim().is_empty() {
            bail!("text file {i}: content is empty");
        }
        if text.as_bytes().len() > MAX_FILE_BYTES {
            bail!("text file {i}: exceeds 1 MB limit");
        }
    }
    Ok(())
}

pub fn write_text_payloads(session_id: &str, files: &[TextFilePayload]) -> Result<Vec<PathBuf>> {
    if files.is_empty() {
        return Ok(vec![]);
    }
    validate_text_payloads(files)?;
    let dir = crate::cancel_ipc::dashboard_state_dir()
        .join("uploads")
        .join(session_id);
    std::fs::create_dir_all(&dir)?;
    let mut paths = Vec::with_capacity(files.len());
    for f in files {
        let safe_name = sanitize_filename(&f.filename);
        let path = dir.join(format!("{}-{}", Uuid::new_v4().simple(), safe_name));
        let text = normalize_upload_content(&f.filename, &f.content)?;
        std::fs::write(&path, text)?;
        paths.push(path);
    }
    Ok(paths)
}

pub fn text_file_line(path: &Path) -> String {
    format!("@anycode/text-file:{}\n", path.display())
}

fn normalize_upload_content(filename: &str, content: &str) -> Result<String> {
    let ext = Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    if ext != "pdf" {
        return Ok(content.to_string());
    }
    let raw = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, content.trim())
        .map_err(|e| anyhow::anyhow!("pdf file: invalid base64 payload: {e}"))?;
    pdf_extract::extract_text_from_mem(&raw)
        .map_err(|e| anyhow::anyhow!("pdf text extraction failed: {e}"))
}

fn sanitize_filename(name: &str) -> String {
    let base = Path::new(name)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("upload.txt");
    base.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsupported_extension() {
        let err = validate_text_payloads(&[TextFilePayload {
            filename: "evil.exe".into(),
            content: "x".into(),
        }])
        .unwrap_err();
        assert!(err.to_string().contains("unsupported"));
    }

    #[test]
    fn rejects_oversized_content() {
        let err = validate_text_payloads(&[TextFilePayload {
            filename: "big.txt".into(),
            content: "x".repeat(MAX_FILE_BYTES + 1),
        }])
        .unwrap_err();
        assert!(err.to_string().contains("1 MB"));
    }
}
