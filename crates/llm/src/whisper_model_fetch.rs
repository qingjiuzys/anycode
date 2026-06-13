//! Download whisper.cpp ggml weights into `~/.anycode/models/whisper/`.

#![cfg_attr(not(feature = "stt-local"), allow(dead_code))]

use crate::model_cache::whisper_model_path;
use anycode_core::CoreError;
use std::path::{Path, PathBuf};

const WHISPER_HF_REPO: &str = "ggerganov/whisper.cpp";

fn hf_endpoints() -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(ep) = std::env::var("HF_ENDPOINT") {
        let t = ep.trim().to_string();
        if !t.is_empty() {
            out.push(t.trim_end_matches('/').to_string());
        }
    }
    for ep in ["https://huggingface.co", "https://hf-mirror.com"] {
        if !out.iter().any(|e| e == ep) {
            out.push(ep.to_string());
        }
    }
    out
}

pub fn whisper_ggml_filename(model_id: &str) -> String {
    format!("ggml-{model_id}.bin")
}

pub fn whisper_download_url(model_id: &str, hf_base: &str) -> String {
    format!(
        "{}/{WHISPER_HF_REPO}/resolve/main/{}",
        hf_base.trim_end_matches('/'),
        whisper_ggml_filename(model_id)
    )
}

/// Ensure `{model_id}.bin` exists under `~/.anycode/models/whisper/`, downloading on first use.
pub async fn ensure_whisper_model(model_id: &str) -> Result<PathBuf, CoreError> {
    let path = whisper_model_path(model_id);
    if path.exists() {
        return Ok(path);
    }
    let parent = path
        .parent()
        .ok_or_else(|| CoreError::LLMError("invalid whisper model path".into()))?;
    std::fs::create_dir_all(parent).map_err(|e| CoreError::LLMError(e.to_string()))?;

    let filename = whisper_ggml_filename(model_id);
    let mut last_err = String::new();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|e| CoreError::LLMError(format!("whisper download client: {e}")))?;

    for base in hf_endpoints() {
        let url = whisper_download_url(model_id, &base);
        tracing::info!(url = %url, "downloading whisper model (first use)");
        match download_to_path(&client, &url, &path).await {
            Ok(()) => {
                tracing::info!(path = %path.display(), "whisper model ready");
                return Ok(path);
            }
            Err(e) => {
                tracing::warn!(url = %url, error = %e, "whisper model download failed");
                last_err = e;
                let _ = std::fs::remove_file(&path);
            }
        }
    }

    Err(CoreError::LLMError(format!(
        "failed to download whisper model {filename} — {last_err}. \
         Place the file manually at {} or set HF_ENDPOINT to a reachable mirror",
        path.display()
    )))
}

async fn download_to_path(client: &reqwest::Client, url: &str, dest: &Path) -> Result<(), String> {
    let part = dest.with_extension("bin.part");
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {} from {url}", resp.status()));
    }
    let bytes = resp.bytes().await.map_err(|e| format!("read body: {e}"))?;
    if bytes.len() < 1024 {
        return Err(format!("unexpected small download ({})", bytes.len()));
    }
    std::fs::write(&part, &bytes).map_err(|e| format!("write {}: {e}", part.display()))?;
    std::fs::rename(&part, dest).map_err(|e| format!("rename to {}: {e}", dest.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_hf_url() {
        let url = whisper_download_url("tiny", "https://huggingface.co");
        assert!(url.contains("ggml-tiny.bin"));
        assert!(url.contains("ggerganov/whisper.cpp"));
    }

    #[test]
    fn ggml_filename_matches_whisper_cpp() {
        assert_eq!(whisper_ggml_filename("tiny"), "ggml-tiny.bin");
        assert_eq!(whisper_ggml_filename("base.en"), "ggml-base.en.bin");
    }
}
