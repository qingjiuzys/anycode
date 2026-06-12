//! Shared HTTP helpers for OpenAI-compatible media endpoints.

use crate::local_media_catalog::local_media_provider_allows_placeholder_key;
use crate::media::MediaProfile;
use anycode_core::CoreError;
use reqwest::Client;
use std::time::Duration;

pub(crate) fn openai_base(profile: &MediaProfile) -> Result<String, CoreError> {
    profile
        .base_url
        .clone()
        .or_else(|| default_media_v1_base(&profile.provider))
        .ok_or_else(|| {
            let hint = docs_hint_for_provider(&profile.provider);
            CoreError::LLMError(format!(
                "media provider `{}` requires base_url{}",
                profile.provider,
                hint.map(|h| format!(" ({h})")).unwrap_or_default()
            ))
        })
}

fn default_media_v1_base(provider: &str) -> Option<String> {
    match provider.trim().to_ascii_lowercase().as_str() {
        "openai" => Some("https://api.openai.com/v1".to_string()),
        "ollama" => Some("http://127.0.0.1:11434/v1".to_string()),
        "whisper_cpp" | "whisper-cpp" | "faster_whisper" => {
            Some("http://127.0.0.1:8080/v1".to_string())
        }
        "piper" => Some("http://127.0.0.1:5000/v1".to_string()),
        _ => None,
    }
}

fn docs_hint_for_provider(provider: &str) -> Option<&'static str> {
    match provider.trim().to_ascii_lowercase().as_str() {
        "whisper_cpp" | "whisper-cpp" => Some("see whisper.cpp server docs"),
        "piper" => Some("see Piper HTTP server docs"),
        "local_whisper" => Some("build with --features stt-local"),
        "local_piper" => Some("build with --features tts-local"),
        "local_fastembed" => Some("build with --features embedding-local"),
        _ => None,
    }
}

/// Resolve API key, allowing placeholder keys for local providers.
pub(crate) fn resolve_api_key(profile: &MediaProfile) -> String {
    let key = profile.api_key.trim();
    if !key.is_empty() {
        return key.to_string();
    }
    if local_media_provider_allows_placeholder_key(&profile.provider) {
        return "local".to_string();
    }
    String::new()
}

pub(crate) fn http_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .unwrap_or_else(|_| Client::new())
}

pub(crate) fn bearer_headers(profile: &MediaProfile) -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    let key = resolve_api_key(profile);
    if !key.is_empty() && key != "local" {
        if let Ok(v) = format!("Bearer {key}").parse() {
            headers.insert(reqwest::header::AUTHORIZATION, v);
        }
    }
    if let Some(ref extra) = profile.extra_headers {
        for (k, v) in extra {
            if let (Ok(name), Ok(val)) = (
                reqwest::header::HeaderName::from_bytes(k.as_bytes()),
                v.parse(),
            ) {
                headers.insert(name, val);
            }
        }
    }
    headers
}

/// Voice parameter for TTS requests (extra_headers `voice`, else model id, else OpenAI default).
pub(crate) fn resolve_tts_voice(profile: &MediaProfile) -> String {
    profile
        .extra_headers
        .as_ref()
        .and_then(|h| h.get("voice"))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            let prov = profile.provider.trim().to_ascii_lowercase();
            if prov == "openai" {
                "alloy".to_string()
            } else {
                profile.model.clone()
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability_catalog::ModelCapability;

    #[test]
    fn default_whisper_cpp_base() {
        let p = MediaProfile {
            capability: ModelCapability::Stt,
            provider: "whisper_cpp".into(),
            model: "tiny".into(),
            api_key: "local".into(),
            base_url: None,
            extra_headers: None,
            endpoint_overrides: None,
        };
        assert_eq!(
            default_media_v1_base(&p.provider).as_deref(),
            Some("http://127.0.0.1:8080/v1")
        );
    }

    #[test]
    fn tts_voice_from_extra_headers() {
        let mut headers = std::collections::HashMap::new();
        headers.insert("voice".into(), "zh_CN-huayan-medium".into());
        let p = MediaProfile {
            capability: ModelCapability::Tts,
            provider: "piper".into(),
            model: "zh_CN-huayan-medium".into(),
            api_key: "local".into(),
            base_url: None,
            extra_headers: Some(headers),
            endpoint_overrides: None,
        };
        assert_eq!(resolve_tts_voice(&p), "zh_CN-huayan-medium");
    }
}
