//! `WebFetch` — 拉取 URL 正文（大小限制）；`prompt` 字段仅作元数据提示，不在工具内二次调用模型。

use crate::services::ToolServices;
use anycode_core::prelude::*;
use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Instant;

pub struct WebFetchTool {
    security_policy: SecurityPolicy,
    services: Arc<ToolServices>,
}

impl WebFetchTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            security_policy: SecurityPolicy::sensitive_mutation(),
            services,
        }
    }
}

#[derive(Deserialize)]
struct WfInput {
    url: String,
    #[serde(default)]
    prompt: String,
}

fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "WebFetch"
    }

    fn description(&self) -> &str {
        "Fetch content from a URL (text/html), with byte limit. Host processes `prompt` separately."
    }

    fn api_tool_description(&self) -> String {
        format!(
            "{}\n\n\
            HTTP(S) fetch for page text; HTML is lightly stripped.\n\
            - Subject to size limits; very large responses are truncated or rejected.\n\
            - `prompt` is **not** sent to a model inside the tool—it is metadata for the host/session only.\n\
            - May require approval depending on security policy.",
            self.description()
        )
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "HTTP(S) URL to fetch" },
                "prompt": { "type": "string", "description": "Host-side prompt hint (not executed inside this tool)" }
            },
            "required": ["url"]
        })
    }

    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Default
    }

    fn security_policy(&self) -> Option<&SecurityPolicy> {
        Some(&self.security_policy)
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let start = Instant::now();
        let wf: WfInput =
            serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;

        let url = url::Url::parse(&wf.url)
            .map_err(|e| CoreError::Other(anyhow::anyhow!("bad url: {}", e)))?;
        if !matches!(url.scheme(), "http" | "https") {
            return Ok(ToolOutput {
                result: serde_json::json!({"error": "only http/https allowed"}),
                error: Some("scheme".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        let resp = self
            .services
            .http
            .get(wf.url.clone())
            .send()
            .await
            .map_err(|e| CoreError::Other(anyhow::anyhow!("fetch failed: {}", e)))?;

        let code = resp.status().as_u16();
        let code_text = resp.status().canonical_reason().unwrap_or("").to_string();
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| CoreError::Other(anyhow::anyhow!("read body: {}", e)))?;

        let max = self.services.max_fetch_bytes as usize;
        let truncated = bytes.len() > max;
        let slice = if truncated { &bytes[..max] } else { &bytes[..] };

        let raw_str = String::from_utf8_lossy(slice);
        let text = if raw_str.contains('<') && raw_str.contains('>') {
            strip_tags(&raw_str)
        } else {
            raw_str.into_owned()
        };

        let result_text = if text.len() > 256_000 {
            text.chars().take(256_000).collect::<String>() + "\n...<truncated>"
        } else {
            text
        };

        Ok(ToolOutput {
            result: serde_json::json!({
                "bytes": bytes.len(),
                "code": code,
                "codeText": code_text,
                "result": result_text,
                "durationMs": start.elapsed().as_millis() as u64,
                "prompt_note": wf.prompt,
                "body_truncated_to_max_fetch": truncated
            }),
            error: if (200..400).contains(&code) {
                None
            } else {
                Some(format!("HTTP {}", code))
            },
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}
