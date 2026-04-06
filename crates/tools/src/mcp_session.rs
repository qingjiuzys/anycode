//! 长驻 MCP stdio 连接：`initialize`、`tools/list`、`tools/call`、`resources/list`、`resources/read`（JSON-RPC 单行、串行锁）。

pub use crate::mcp_connected::McpListedTool;
use crate::mcp_normalization::normalize_name_for_mcp;
use anycode_core::prelude::*;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::Mutex;
use tokio::time::timeout;

const READ_TIMEOUT: Duration = Duration::from_secs(120);

pub struct McpIo {
    pub stdin: ChildStdin,
    pub reader: BufReader<tokio::process::ChildStdout>,
    pub child: Child,
}

pub struct McpStdioSession {
    io: Mutex<McpIo>,
    next_id: AtomicU64,
    pub server_slug: String,
    pub listed_tools: Vec<McpListedTool>,
}

async fn write_line(stdin: &mut ChildStdin, v: &Value) -> Result<(), CoreError> {
    let s = serde_json::to_string(v).map_err(|e| CoreError::LLMError(e.to_string()))?;
    stdin
        .write_all(s.as_bytes())
        .await
        .map_err(|e| CoreError::LLMError(e.to_string()))?;
    stdin
        .write_all(b"\n")
        .await
        .map_err(|e| CoreError::LLMError(e.to_string()))?;
    Ok(())
}

async fn read_until_id(
    reader: &mut BufReader<tokio::process::ChildStdout>,
    want_id: u64,
) -> Result<Value, CoreError> {
    let mut line = String::new();
    for _ in 0..1024 {
        line.clear();
        timeout(READ_TIMEOUT, reader.read_line(&mut line))
            .await
            .map_err(|_| CoreError::LLMError("MCP read timeout".into()))?
            .map_err(|e| CoreError::LLMError(e.to_string()))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let v: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if v.get("id") == Some(&json!(want_id)) {
            return Ok(v);
        }
    }
    Err(CoreError::LLMError(
        "MCP: no JSON-RPC response with matching id".into(),
    ))
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

impl McpStdioSession {
    /// 启动子进程、完成 handshake 并拉取 `tools/list`。
    /// `server_slug` 会经 [`normalize_name_for_mcp`] 规范化后再存入（与 Claude Code `buildMcpToolName` 前缀一致）。
    pub async fn connect(command_shell: &str, server_slug: &str) -> Result<Self, CoreError> {
        let server_slug = normalize_name_for_mcp(server_slug);
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(command_shell)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| CoreError::LLMError(format!("MCP spawn failed: {}", e)))?;

        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| CoreError::LLMError("MCP stdin missing".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| CoreError::LLMError("MCP stdout missing".into()))?;
        let mut reader = BufReader::new(stdout);

        write_line(
            &mut stdin,
            &json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": { "name": "anycode", "version": "0.1" }
                }
            }),
        )
        .await?;

        let init_resp = read_until_id(&mut reader, 1).await?;
        if init_resp.get("error").is_some() {
            return Err(CoreError::LLMError(format!(
                "MCP initialize error: {}",
                init_resp.get("error").unwrap_or(&json!({}))
            )));
        }

        write_line(
            &mut stdin,
            &json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized",
                "params": {}
            }),
        )
        .await?;

        write_line(
            &mut stdin,
            &json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/list",
                "params": {}
            }),
        )
        .await?;

        let list_resp = read_until_id(&mut reader, 2).await?;
        if list_resp.get("error").is_some() {
            return Err(CoreError::LLMError(format!(
                "MCP tools/list error: {}",
                list_resp.get("error").unwrap_or(&json!({}))
            )));
        }

        let listed_tools = parse_tools_list(&list_resp)?;

        let io = McpIo {
            stdin,
            reader,
            child,
        };

        Ok(Self {
            io: Mutex::new(io),
            next_id: AtomicU64::new(10),
            server_slug,
            listed_tools,
        })
    }

    async fn rpc(&self, method: &str, params: Value) -> Result<Value, CoreError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let mut io = self.io.lock().await;
        write_line(
            &mut io.stdin,
            &json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }),
        )
        .await?;
        read_until_id(&mut io.reader, id).await
    }

    pub async fn call_tool_named(&self, name: &str, arguments: Value) -> Result<ToolOutput, CoreError> {
        let start = std::time::Instant::now();
        let resp = self
            .rpc(
                "tools/call",
                json!({ "name": name, "arguments": arguments }),
            )
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

    pub async fn resources_list(&self, server: Option<&str>) -> Result<Value, CoreError> {
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

    pub async fn resources_read(&self, uri: &str) -> Result<Value, CoreError> {
        let resp = self
            .rpc("resources/read", json!({ "uri": uri }))
            .await?;
        if let Some(err) = resp.get("error") {
            return Err(CoreError::LLMError(format!("resources/read: {}", err)));
        }
        Ok(resp.get("result").cloned().unwrap_or(resp))
    }
}

#[async_trait::async_trait]
impl crate::mcp_connected::McpConnected for McpStdioSession {
    fn server_slug(&self) -> &str {
        &self.server_slug
    }

    fn listed_tools(&self) -> &[McpListedTool] {
        &self.listed_tools
    }

    async fn call_tool_named(&self, name: &str, arguments: Value) -> Result<ToolOutput, CoreError> {
        McpStdioSession::call_tool_named(self, name, arguments).await
    }

    async fn resources_list(&self, server: Option<&str>) -> Result<Value, CoreError> {
        McpStdioSession::resources_list(self, server).await
    }

    async fn resources_read(&self, uri: &str) -> Result<Value, CoreError> {
        McpStdioSession::resources_read(self, uri).await
    }
}

