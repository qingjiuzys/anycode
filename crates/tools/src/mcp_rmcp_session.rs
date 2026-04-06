//! MCP **Streamable HTTP** 客户端（HTTP POST，响应可为 `application/json` 或 `text/event-stream`）。
//! Streamable HTTP MCP 客户端（`rmcp`）；优先使用服务器提供的单一 MCP URL。

use crate::mcp_connected::{McpConnected, McpListedTool};
use crate::mcp_normalization::normalize_name_for_mcp;
use anycode_core::prelude::*;
use async_trait::async_trait;
use http::{HeaderName, HeaderValue};
use rmcp::{
    model::{
        CallToolRequestParams, ClientCapabilities, ClientInfo, Implementation, ReadResourceRequestParams,
    },
    service::{RoleClient, RunningService, ServiceExt},
    transport::{
        streamable_http_client::StreamableHttpClientTransportConfig, AuthClient, AuthorizationManager,
        StreamableHttpClientTransport,
    },
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

fn map_service_err(e: rmcp::service::ServiceError) -> CoreError {
    CoreError::LLMError(e.to_string())
}

pub struct McpRmcpSession {
    pub server_slug: String,
    pub listed_tools: Vec<McpListedTool>,
    client: Arc<Mutex<RunningService<RoleClient, ClientInfo>>>,
}

impl McpRmcpSession {
    async fn from_running(
        running: RunningService<RoleClient, ClientInfo>,
        server_slug: String,
    ) -> Result<Self, CoreError> {
        let tools = running
            .list_all_tools()
            .await
            .map_err(map_service_err)?;

        let mut listed_tools = Vec::new();
        for t in tools {
            let description = t
                .description
                .as_ref()
                .map(|c| c.to_string())
                .unwrap_or_default();
            let input_schema = serde_json::to_value(t.input_schema.as_ref()).unwrap_or_else(|_| {
                json!({"type": "object", "additionalProperties": true})
            });
            listed_tools.push(McpListedTool {
                name: t.name.to_string(),
                description,
                input_schema,
            });
        }

        Ok(Self {
            server_slug,
            listed_tools,
            client: Arc::new(Mutex::new(running)),
        })
    }

    pub async fn connect_streamable_http(
        url: &str,
        server_slug: &str,
        bearer_token: Option<&str>,
        headers: &HashMap<String, String>,
    ) -> Result<Self, CoreError> {
        let server_slug = normalize_name_for_mcp(server_slug);
        let mut config = StreamableHttpClientTransportConfig::with_uri(url.trim());
        if let Some(t) = bearer_token {
            let t = t.trim();
            if !t.is_empty() {
                config = config.auth_header(t);
            }
        }
        for (k, v) in headers {
            let kn = k.trim();
            if kn.is_empty() {
                continue;
            }
            let name = HeaderName::try_from(kn)
                .map_err(|e| CoreError::LLMError(format!("invalid MCP header name {kn}: {e}")))?;
            let value = HeaderValue::try_from(v.as_str()).map_err(|e| {
                CoreError::LLMError(format!("invalid MCP header value for {kn}: {e}"))
            })?;
            config.custom_headers.insert(name, value);
        }
        let transport = StreamableHttpClientTransport::from_config(config);
        let info = ClientInfo::new(
            ClientCapabilities::default(),
            Implementation::new("anycode", env!("CARGO_PKG_VERSION")),
        );
        let running = info
            .serve(transport)
            .await
            .map_err(|e| CoreError::LLMError(e.to_string()))?;

        Self::from_running(running, server_slug).await
    }

    /// 从 JSON 凭证文件连接（含 refresh token 时由 `AuthClient` 在每次请求前刷新并写回文件）。
    ///
    /// `config.auth_header` 必须为 `None`，以便 `AuthClient` 在 `auth_token == None` 时调用
    /// `get_access_token()`（内部按需刷新并持久化）。
    pub async fn connect_streamable_http_oauth(
        url: &str,
        server_slug: &str,
        oauth_credentials_path: &Path,
        headers: &HashMap<String, String>,
    ) -> Result<Self, CoreError> {
        let server_slug = normalize_name_for_mcp(server_slug);
        let store = crate::mcp_oauth_store::JsonFileCredentialStore::new(oauth_credentials_path.to_path_buf());
        let mut manager = AuthorizationManager::new(url.trim())
            .await
            .map_err(|e| CoreError::LLMError(format!("MCP OAuth: {e}")))?;
        manager.set_credential_store(store);
        let ok = manager
            .initialize_from_store()
            .await
            .map_err(|e| CoreError::LLMError(format!("MCP OAuth: {e}")))?;
        if !ok {
            return Err(CoreError::LLMError(format!(
                "no OAuth credentials in {}; run `anycode mcp oauth-login --url ... --credentials-store {}`",
                oauth_credentials_path.display(),
                oauth_credentials_path.display()
            )));
        }
        manager
            .get_access_token()
            .await
            .map_err(|e| CoreError::LLMError(format!("MCP OAuth token: {e}")))?;

        let reqwest_client = reqwest::Client::default();
        let auth_client = AuthClient::new(reqwest_client, manager);

        let mut config = StreamableHttpClientTransportConfig::with_uri(url.trim());
        for (k, v) in headers {
            let kn = k.trim();
            if kn.is_empty() {
                continue;
            }
            let name = HeaderName::try_from(kn)
                .map_err(|e| CoreError::LLMError(format!("invalid MCP header name {kn}: {e}")))?;
            let value = HeaderValue::try_from(v.as_str()).map_err(|e| {
                CoreError::LLMError(format!("invalid MCP header value for {kn}: {e}"))
            })?;
            config.custom_headers.insert(name, value);
        }

        let transport = StreamableHttpClientTransport::with_client(auth_client, config);
        let info = ClientInfo::new(
            ClientCapabilities::default(),
            Implementation::new("anycode", env!("CARGO_PKG_VERSION")),
        );
        let running = info
            .serve(transport)
            .await
            .map_err(|e| CoreError::LLMError(e.to_string()))?;

        Self::from_running(running, server_slug).await
    }
}

#[async_trait]
impl McpConnected for McpRmcpSession {
    fn server_slug(&self) -> &str {
        &self.server_slug
    }

    fn listed_tools(&self) -> &[McpListedTool] {
        &self.listed_tools
    }

    async fn call_tool_named(&self, name: &str, arguments: Value) -> Result<ToolOutput, CoreError> {
        let start = std::time::Instant::now();
        let args_obj = arguments.as_object().cloned().unwrap_or_default();
        let params = CallToolRequestParams::new(name.to_string()).with_arguments(args_obj);
        let g = self.client.lock().await;
        let result = g.call_tool(params).await.map_err(map_service_err)?;
        let err = result.is_error.unwrap_or(false);
        let out =
            serde_json::to_value(&result).unwrap_or_else(|_| json!({ "note": "serialize error" }));
        Ok(ToolOutput {
            result: out,
            error: if err {
                Some("mcp tools/call is_error".into())
            } else {
                None
            },
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn resources_list(&self, _server: Option<&str>) -> Result<Value, CoreError> {
        let g = self.client.lock().await;
        let all = g
            .list_all_resources()
            .await
            .map_err(map_service_err)?;
        serde_json::to_value(&all).map_err(|e| CoreError::LLMError(e.to_string()))
    }

    async fn resources_read(&self, uri: &str) -> Result<Value, CoreError> {
        let g = self.client.lock().await;
        let r = g
            .read_resource(ReadResourceRequestParams::new(uri))
            .await
            .map_err(map_service_err)?;
        serde_json::to_value(&r).map_err(|e| CoreError::LLMError(e.to_string()))
    }
}
