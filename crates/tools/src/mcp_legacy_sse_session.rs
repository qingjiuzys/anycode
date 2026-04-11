//! MCP **legacy HTTP+SSE** 客户端：GET `text/event-stream`，首帧 `event: endpoint`（或 `data` 为 POST URL），
//! 之后 JSON-RPC 经 POST 发送、响应从 SSE `data:` 回传（与 OpenClaw `SSEClientTransport` 同类）。

use crate::mcp_connected::{McpConnected, McpListedTool};
use crate::mcp_normalization::normalize_name_for_mcp;
use anycode_core::prelude::*;
use async_trait::async_trait;
use futures_util::StreamExt;
use serde_json::{json, Value};
use sse_stream::{Sse, SseStream};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{oneshot, Mutex};
use tracing::error;

const MCP_ENDPOINT_WAIT: Duration = Duration::from_secs(30);
const MCP_RPC_WAIT: Duration = Duration::from_secs(120);

fn id_key(id: &Value) -> String {
    serde_json::to_string(id).unwrap_or_default()
}

fn parse_endpoint_data(d: &str) -> Option<String> {
    let t = d.trim();
    if t.starts_with("http://") || t.starts_with("https://") {
        return Some(t.to_string());
    }
    let v: Value = serde_json::from_str(t).ok()?;
    v.get("url").and_then(|x| x.as_str()).map(|s| s.to_string())
}

fn parse_endpoint_from_sse(sse: &Sse) -> Option<String> {
    if sse.event.as_deref() == Some("endpoint") {
        return sse.data.as_ref().and_then(|d| parse_endpoint_data(d));
    }
    sse.data.as_ref().and_then(|d| parse_endpoint_data(d))
}

fn build_reqwest() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(600))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

pub struct McpLegacySseSession {
    _sse_reader: tokio::task::JoinHandle<()>,
    client: reqwest::Client,
    post_url: String,
    headers: HashMap<String, String>,
    bearer_token: Option<String>,
    pending: Arc<Mutex<HashMap<String, oneshot::Sender<Value>>>>,
    next_id: AtomicU64,
    pub server_slug: String,
    pub listed_tools: Vec<McpListedTool>,
}

impl McpLegacySseSession {
    /// 连接 SSE URL，等待 `endpoint`，完成 initialize / tools/list。
    pub async fn connect(
        sse_url: &str,
        server_slug: &str,
        bearer_token: Option<&str>,
        headers: &HashMap<String, String>,
    ) -> Result<Self, CoreError> {
        let server_slug = normalize_name_for_mcp(server_slug);
        let client = build_reqwest();

        let mut req = client
            .get(sse_url.trim())
            .header("Accept", "text/event-stream");
        if let Some(t) = bearer_token.map(str::trim).filter(|s| !s.is_empty()) {
            req = req.header("Authorization", format!("Bearer {}", t));
        }
        for (k, v) in headers {
            let kn = k.trim();
            if kn.is_empty() {
                continue;
            }
            req = req.header(kn, v.as_str());
        }

        let response = req
            .send()
            .await
            .map_err(|e| CoreError::LLMError(format!("MCP legacy SSE GET: {e}")))?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(CoreError::LLMError(format!(
                "MCP legacy SSE GET failed: {status} {body}"
            )));
        }

        let pending: Arc<Mutex<HashMap<String, oneshot::Sender<Value>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let (post_tx, post_rx) = oneshot::channel::<String>();
        let pending_clone = pending.clone();
        let stream = response.bytes_stream();
        let mut sse = SseStream::from_byte_stream(stream);
        let sse_reader = tokio::spawn(async move {
            let mut post_tx = Some(post_tx);
            let mut post_sent = false;
            while let Some(item) = sse.next().await {
                let ev = match item {
                    Ok(s) => s,
                    Err(e) => {
                        error!("MCP legacy SSE parse: {}", e);
                        break;
                    }
                };
                if !post_sent {
                    if let Some(url) = parse_endpoint_from_sse(&ev) {
                        post_sent = true;
                        if let Some(tx) = post_tx.take() {
                            let _ = tx.send(url);
                        }
                    }
                }
                let Some(data) = ev.data else { continue };
                let data = data.trim();
                if data.is_empty() {
                    continue;
                }
                let Ok(msg) = serde_json::from_str::<Value>(data) else {
                    continue;
                };
                if msg.get("jsonrpc").is_none() || msg.get("id").is_none() {
                    continue;
                }
                let key = id_key(msg.get("id").unwrap());
                if let Some(tx) = pending_clone.lock().await.remove(&key) {
                    let _ = tx.send(msg);
                }
            }
        });

        let post_url = tokio::time::timeout(MCP_ENDPOINT_WAIT, post_rx)
            .await
            .map_err(|_| {
                CoreError::LLMError(
                    "MCP legacy SSE: timeout waiting for endpoint event (event: endpoint / data URL)"
                        .into(),
                )
            })?
            .map_err(|_| CoreError::LLMError("MCP legacy SSE: endpoint channel closed".into()))?;

        let mut sess = Self {
            _sse_reader: sse_reader,
            client,
            post_url,
            headers: headers.clone(),
            bearer_token: bearer_token
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            pending,
            next_id: AtomicU64::new(1),
            server_slug,
            listed_tools: Vec::new(),
        };

        let init_resp = sess
            .rpc(
                "initialize",
                json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": { "name": "anycode", "version": env!("CARGO_PKG_VERSION") }
                }),
            )
            .await?;
        if init_resp.get("error").is_some() {
            return Err(CoreError::LLMError(format!(
                "MCP legacy SSE initialize error: {}",
                init_resp.get("error").unwrap_or(&json!({}))
            )));
        }

        sess.rpc("notifications/initialized", json!({})).await?;

        let list_resp = sess.rpc("tools/list", json!({})).await?;
        if list_resp.get("error").is_some() {
            return Err(CoreError::LLMError(format!(
                "MCP legacy SSE tools/list error: {}",
                list_resp.get("error").unwrap_or(&json!({}))
            )));
        }

        sess.listed_tools = parse_tools_list(&list_resp)?;
        sess.next_id = AtomicU64::new(10);

        Ok(sess)
    }

    async fn rpc(&self, method: &str, params: Value) -> Result<Value, CoreError> {
        let notification = method.starts_with("notifications/");
        let id = if notification {
            None
        } else {
            Some(self.next_id.fetch_add(1, Ordering::SeqCst))
        };

        let request = if let Some(id) = id {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": method,
                "params": params
            })
        } else {
            json!({
                "jsonrpc": "2.0",
                "method": method,
                "params": params
            })
        };

        let rx = if let Some(id) = request.get("id") {
            let (tx, rx) = oneshot::channel();
            self.pending.lock().await.insert(id_key(id), tx);
            Some(rx)
        } else {
            None
        };

        let mut req = self
            .client
            .post(&self.post_url)
            .header("Content-Type", "application/json");
        if let Some(ref t) = self.bearer_token {
            req = req.header("Authorization", format!("Bearer {}", t));
        }
        for (k, v) in &self.headers {
            req = req.header(k, v);
        }
        let resp = req
            .json(&request)
            .send()
            .await
            .map_err(|e| CoreError::LLMError(format!("MCP legacy SSE POST: {e}")))?;

        let status = resp.status();
        if !status.is_success() && status != reqwest::StatusCode::ACCEPTED {
            let body = resp.text().await.unwrap_or_default();
            if let Some(id) = request.get("id") {
                let _ = self.pending.lock().await.remove(&id_key(id));
            }
            return Err(CoreError::LLMError(format!(
                "MCP legacy SSE POST: {status} {body}"
            )));
        }

        let body = resp.bytes().await.unwrap_or_default();
        if !body.is_empty() {
            if let Ok(v) = serde_json::from_slice::<Value>(&body) {
                if v.get("id").is_some() {
                    if let Some(id) = request.get("id") {
                        let _ = self.pending.lock().await.remove(&id_key(id));
                    }
                    if let Some(rx) = rx {
                        drop(rx);
                    }
                    return Ok(v);
                }
            }
        }

        if let Some(rx) = rx {
            let id = request.get("id").unwrap();
            let key = id_key(id);
            match tokio::time::timeout(MCP_RPC_WAIT, rx).await {
                Ok(Ok(v)) => return Ok(v),
                Ok(Err(_)) => {
                    let _ = self.pending.lock().await.remove(&key);
                    return Err(CoreError::LLMError(
                        "MCP legacy SSE: response channel closed".into(),
                    ));
                }
                Err(_) => {
                    let _ = self.pending.lock().await.remove(&key);
                    return Err(CoreError::LLMError(format!(
                        "MCP legacy SSE: timeout waiting for response id={key}"
                    )));
                }
            }
        }
        Ok(json!({}))
    }
}

fn parse_tools_list(resp: &Value) -> Result<Vec<McpListedTool>, CoreError> {
    let tools_val = resp
        .get("result")
        .and_then(|r| r.get("tools"))
        .ok_or_else(|| CoreError::LLMError("MCP tools/list: missing result.tools".into()))?;
    let arr = tools_val
        .as_array()
        .ok_or_else(|| CoreError::LLMError("MCP tools/list: tools not array".into()))?;
    let mut out = Vec::new();
    for t in arr {
        let name = t
            .get("name")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if name.is_empty() {
            continue;
        }
        let description = t
            .get("description")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let input_schema = t
            .get("inputSchema")
            .cloned()
            .or_else(|| t.get("input_schema").cloned())
            .unwrap_or_else(|| json!({"type": "object", "additionalProperties": true}));
        out.push(McpListedTool {
            name,
            description,
            input_schema,
        });
    }
    Ok(out)
}

#[async_trait]
impl McpConnected for McpLegacySseSession {
    fn server_slug(&self) -> &str {
        &self.server_slug
    }

    fn listed_tools(&self) -> &[McpListedTool] {
        &self.listed_tools
    }

    async fn call_tool_named(&self, name: &str, arguments: Value) -> Result<ToolOutput, CoreError> {
        let start = std::time::Instant::now();
        let args_obj = arguments.as_object().cloned().unwrap_or_default();
        let resp = self
            .rpc("tools/call", json!({ "name": name, "arguments": args_obj }))
            .await?;
        if let Some(err) = resp.get("error") {
            return Ok(ToolOutput {
                result: json!({ "mcp_error": err }),
                error: Some("mcp tools/call failed".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
        Ok(ToolOutput {
            result: resp.get("result").cloned().unwrap_or(resp),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn resources_list(&self, server: Option<&str>) -> Result<Value, CoreError> {
        let mut params = json!({});
        if let Some(s) = server {
            params["server"] = json!(s);
        }
        let resp = self.rpc("resources/list", params).await?;
        if let Some(err) = resp.get("error") {
            return Err(CoreError::LLMError(format!("resources/list: {}", err)));
        }
        Ok(resp.get("result").cloned().unwrap_or(resp))
    }

    async fn resources_read(&self, uri: &str) -> Result<Value, CoreError> {
        let resp = self.rpc("resources/read", json!({ "uri": uri })).await?;
        if let Some(err) = resp.get("error") {
            return Err(CoreError::LLMError(format!("resources/read: {}", err)));
        }
        Ok(resp.get("result").cloned().unwrap_or(resp))
    }
}
