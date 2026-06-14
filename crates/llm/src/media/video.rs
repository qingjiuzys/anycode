//! Video generation via configurable JSON POST endpoint.

use crate::media::http::{bearer_headers, http_client};
use crate::media::MediaProfile;
use anycode_core::CoreError;
use std::time::Duration;

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

    fn status_url(&self, job_id: &str) -> Result<String, CoreError> {
        if let Some(ref ov) = self.profile.endpoint_overrides {
            if let Some(ref status) = ov.status {
                let t = status.trim();
                if !t.is_empty() {
                    if t.contains("{id}") {
                        return Ok(t.replace("{id}", job_id));
                    }
                    return Ok(format!("{}/{}", t.trim_end_matches('/'), job_id));
                }
            }
        }
        let submit = self.endpoint()?;
        Ok(format!("{}/{}", submit.trim_end_matches('/'), job_id))
    }

    fn extract_video_url(v: &serde_json::Value) -> Option<String> {
        let try_str = |s: &str| {
            if s.starts_with("http://") || s.starts_with("https://") {
                Some(s.to_string())
            } else {
                None
            }
        };
        for key in ["video_url", "url", "remixed_from_video_id"] {
            if let Some(u) = v.get(key).and_then(|x| x.as_str()).and_then(try_str) {
                return Some(u);
            }
        }
        for pointer in [
            "/data/0/url",
            "/data/0/video_url",
            "/data/url",
            "/data/video_url",
            "/result/video_url",
            "/result/url",
        ] {
            if let Some(u) = v
                .pointer(pointer)
                .and_then(|x| x.as_str())
                .and_then(try_str)
            {
                return Some(u);
            }
        }
        None
    }

    fn extract_job_id(v: &serde_json::Value) -> Option<String> {
        for key in ["id", "task_id", "job_id"] {
            if let Some(s) = v.get(key).and_then(|x| x.as_str()) {
                if !s.trim().is_empty() {
                    return Some(s.to_string());
                }
            }
        }
        for pointer in ["/data/id", "/data/task_id", "/result/id"] {
            if let Some(s) = v.pointer(pointer).and_then(|x| x.as_str()) {
                if !s.trim().is_empty() {
                    return Some(s.to_string());
                }
            }
        }
        None
    }

    fn task_status(v: &serde_json::Value) -> Option<&str> {
        v.get("status").and_then(|s| s.as_str())
    }

    fn is_terminal_status(status: &str) -> bool {
        matches!(
            status.trim().to_ascii_lowercase().as_str(),
            "completed" | "failed" | "error" | "cancelled" | "canceled"
        )
    }

    async fn poll_until_done(&self, job_id: &str) -> Result<VideoGenResult, CoreError> {
        let url = self.status_url(job_id)?;
        let client = http_client();
        let headers = bearer_headers(&self.profile);
        for attempt in 0..60 {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            let resp = client
                .get(&url)
                .headers(headers.clone())
                .send()
                .await
                .map_err(|e| CoreError::LLMError(e.to_string()))?;
            let status_code = resp.status();
            let text = resp
                .text()
                .await
                .map_err(|e| CoreError::LLMError(e.to_string()))?;
            if !status_code.is_success() {
                return Err(CoreError::LLMError(format!(
                    "video poll failed status={} body={}",
                    status_code, text
                )));
            }
            let v: serde_json::Value =
                serde_json::from_str(&text).map_err(|e| CoreError::LLMError(e.to_string()))?;
            if let Some(st) = Self::task_status(&v) {
                if Self::is_terminal_status(st) {
                    if st.eq_ignore_ascii_case("failed") || st.eq_ignore_ascii_case("error") {
                        let err = v
                            .get("error")
                            .map(|e| e.to_string())
                            .unwrap_or_else(|| "video generation failed".into());
                        return Err(CoreError::LLMError(err));
                    }
                    return Ok(VideoGenResult {
                        url: Self::extract_video_url(&v),
                        job_id: Some(job_id.to_string()),
                        raw: v,
                    });
                }
            }
        }
        Err(CoreError::LLMError(format!(
            "video generation timed out waiting for job {job_id}"
        )))
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
        let direct_url = Self::extract_video_url(&v);
        let job_id = Self::extract_job_id(&v);
        if direct_url.is_some() {
            return Ok(VideoGenResult {
                url: direct_url,
                job_id,
                raw: v,
            });
        }
        if let Some(ref jid) = job_id {
            if Self::task_status(&v)
                .map(Self::is_terminal_status)
                .unwrap_or(false)
            {
                return Ok(VideoGenResult {
                    url: Self::extract_video_url(&v),
                    job_id: Some(jid.clone()),
                    raw: v,
                });
            }
            return self.poll_until_done(jid).await;
        }
        Ok(VideoGenResult {
            url: None,
            job_id,
            raw: v,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability_catalog::ModelCapability;

    #[test]
    fn extract_video_url_prefers_video_url_field() {
        let v = serde_json::json!({
            "status": "completed",
            "video_url": "https://example.com/a.mp4",
            "remixed_from_video_id": "https://example.com/b.mp4"
        });
        assert_eq!(
            VideoGenClient::extract_video_url(&v).as_deref(),
            Some("https://example.com/a.mp4")
        );
    }

    #[test]
    fn extract_video_url_falls_back_to_remixed_from_video_id() {
        let v = serde_json::json!({
            "status": "completed",
            "remixed_from_video_id": "https://example.com/out.mp4"
        });
        assert_eq!(
            VideoGenClient::extract_video_url(&v).as_deref(),
            Some("https://example.com/out.mp4")
        );
    }

    #[test]
    fn extract_video_url_nested_agnes_shape() {
        let v = serde_json::json!({
            "status": "completed",
            "data": { "video_url": "https://example.com/agnes.mp4" }
        });
        assert_eq!(
            VideoGenClient::extract_video_url(&v).as_deref(),
            Some("https://example.com/agnes.mp4")
        );
    }

    #[test]
    fn extract_job_id_from_task_id_field() {
        let v = serde_json::json!({ "task_id": "task_abc", "status": "processing" });
        assert_eq!(
            VideoGenClient::extract_job_id(&v).as_deref(),
            Some("task_abc")
        );
    }

    #[test]
    fn status_url_replaces_id_placeholder() {
        let client = VideoGenClient::new(MediaProfile {
            capability: ModelCapability::VideoGen,
            provider: "custom".into(),
            model: "agnes-video-v2.0".into(),
            api_key: "sk-test".into(),
            base_url: None,
            extra_headers: None,
            endpoint_overrides: Some(crate::config_models::EndpointOverrides {
                submit: Some("https://apihub.agnes-ai.com/v1/videos".into()),
                status: Some("https://apihub.agnes-ai.com/v1/videos/{id}".into()),
                result: None,
            }),
        });
        assert_eq!(
            client.status_url("task_abc").unwrap(),
            "https://apihub.agnes-ai.com/v1/videos/task_abc"
        );
    }
}
