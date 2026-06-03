//! Shared HTTP helpers for OpenAI-compatible media endpoints.

use crate::media::MediaProfile;
use anycode_core::CoreError;
use reqwest::Client;
use std::time::Duration;

pub(crate) fn openai_base(profile: &MediaProfile) -> Result<String, CoreError> {
    profile
        .base_url
        .clone()
        .or_else(|| default_openai_v1_base(&profile.provider))
        .ok_or_else(|| {
            CoreError::LLMError(format!(
                "media provider `{}` requires base_url",
                profile.provider
            ))
        })
}

fn default_openai_v1_base(provider: &str) -> Option<String> {
    match provider.trim().to_ascii_lowercase().as_str() {
        "openai" => Some("https://api.openai.com/v1".to_string()),
        _ => None,
    }
}

pub(crate) fn http_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .unwrap_or_else(|_| Client::new())
}

pub(crate) fn bearer_headers(profile: &MediaProfile) -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    if let Ok(v) = format!("Bearer {}", profile.api_key.trim()).parse() {
        headers.insert(reqwest::header::AUTHORIZATION, v);
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
