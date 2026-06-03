//! Video generation via configurable JSON POST endpoint.

use crate::media::http::{bearer_headers, http_client};
use crate::media::MediaProfile;
use anycode_core::CoreError;

#[derive(Debug, Clone)]
pub struct VideoGenResult {
    pub url: Option<String>,
    pub job_id: Option<String>,
    pub raw: serde_json::Value,
}

pub struct VideoGenClient {
    profile: MediaProfile,
}

impl VideoGenClient {
    pub fn new(profile: MediaProfile) -> Self {
        Self { profile }
    }

    fn endpoint(&self) -> Result<String, CoreError> {
        if let Some(ref ov) = self.profile.endpoint_overrides {
            if let Some(ref submit) = ov.submit {
                if !submit.trim().is_empty() {
                    return Ok(submit.trim().to_string());
                }
            }
        }
        self.profile
            .base_url
            .clone()
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| {
                CoreError::LLMError(
                    "video generation requires endpoint_overrides.submit or models.video.base_url"
                        .into(),
                )
            })
    }

    pub async fn generate(&self, prompt: &str) -> Result<VideoGenResult, CoreError> {
        let url = self.endpoint()?;
        let body = serde_json::json!({
            "model": self.profile.model,
            "prompt": prompt
        });
        let resp = http_client()
            .post(url)
            .headers(bearer_headers(&self.profile))
            .json(&body)
            .send()
            .await
            .map_err(|e| CoreError::LLMError(e.to_string()))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| CoreError::LLMError(e.to_string()))?;
        if !status.is_success() {
            return Err(CoreError::LLMError(format!(
                "video gen failed status={} body={}",
                status, text
            )));
        }
        let v: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| CoreError::LLMError(e.to_string()))?;
        Ok(VideoGenResult {
            url: v
                .pointer("/data/0/url")
                .or_else(|| v.get("url"))
                .and_then(|u| u.as_str())
                .map(str::to_string),
            job_id: v.get("id").and_then(|u| u.as_str()).map(str::to_string),
            raw: v,
        })
    }
}
