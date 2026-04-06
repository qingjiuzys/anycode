//! `WebSearch` — 可配置端点 + API key；否则使用 DuckDuckGo 即时答案 JSON。

use crate::services::ToolServices;
use anycode_core::prelude::*;
use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Instant;

pub struct WebSearchTool {
    security_policy: SecurityPolicy,
    services: Arc<ToolServices>,
}

impl WebSearchTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            security_policy: SecurityPolicy::sensitive_mutation(),
            services,
        }
    }
}

#[derive(Deserialize)]
struct WsInput {
    query: String,
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "WebSearch"
    }

    fn description(&self) -> &str {
        "Search the web. Uses ANYCODE_WEB_SEARCH_URL + ANYCODE_WEB_SEARCH_API_KEY if set; otherwise DuckDuckGo instant answer API."
    }

    fn api_tool_description(&self) -> String {
        format!(
            "{}\n\n\
            Returns concise web search snippets / instant answers.\n\
            - When ANYCODE_WEB_SEARCH_URL and ANYCODE_WEB_SEARCH_API_KEY are set, calls that provider; else falls back to DuckDuckGo instant answer JSON.\n\
            - Not a substitute for fetching full pages—use WebFetch when you need article body text.\n\
            - Network access may require approval.",
            self.description()
        )
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Search query" }
            },
            "required": ["query"]
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
        let ws: WsInput =
            serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;

        if let (Some(url), Some(key)) = (
            self.services.web_search_endpoint.as_ref(),
            self.services.web_search_api_key.as_ref(),
        ) {
            let v = self
                .services
                .http
                .post(url)
                .header("Authorization", format!("Bearer {}", key))
                .json(&serde_json::json!({ "query": ws.query }))
                .send()
                .await
                .map_err(|e| CoreError::Other(anyhow::anyhow!("search request: {}", e)))?
                .text()
                .await
                .map_err(|e| CoreError::Other(anyhow::anyhow!("search body: {}", e)))?;
            return Ok(ToolOutput {
                result: serde_json::json!({ "provider": "custom", "raw": v }),
                error: None,
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        let enc = urlencoding::encode(&ws.query);
        let ddg = format!(
            "https://api.duckduckgo.com/?q={}&format=json&no_html=1&skip_disambig=1",
            enc
        );
        let txt = self
            .services
            .http
            .get(&ddg)
            .send()
            .await
            .map_err(|e| CoreError::Other(anyhow::anyhow!("ddg: {}", e)))?
            .text()
            .await
            .map_err(|e| CoreError::Other(anyhow::anyhow!("ddg body: {}", e)))?;

        Ok(ToolOutput {
            result: serde_json::json!({
                "provider": "duckduckgo",
                "raw": txt
            }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}
