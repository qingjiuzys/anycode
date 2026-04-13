//! 长驻 MCP stdio 连接：`initialize`、`tools/list`、`tools/call`、`resources/list`、`resources/read`（JSON-RPC 单行、串行锁）。

pub use crate::mcp_connected::McpListedTool;
use crate::mcp_normalization::normalize_name_for_mcp;
use crate::mcp_read_timeout::{
    self, mcp_jsonrpc_line_timeout, mcp_tools_call_wall_timeout, mcp_wall_timeout_core_error,
};
use anycode_core::prelude::*;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::Mutex;
use tokio::time::timeout;

const DEFAULT_LINE_READ_TIMEOUT: Duration = Duration::from_secs(120);

fn line_read_timeout() -> Duration {
    mcp_jsonrpc_line_timeout(DEFAULT_LINE_READ_TIMEOUT)
}

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
    child: &mut Child,
    want_id: u64,
    read_timeout: Duration,
    server_slug: &str,
) -> Result<Value, CoreError> {
    let mut line = String::new();
    for _ in 0..1024 {
        line.clear();
        let n = match timeout(read_timeout, reader.read_line(&mut line)).await {
            Err(_) => {
                return Err(CoreError::LLMError(format!(
                    "MCP read timed out after {}s waiting for JSON-RPC id={} (server={}); set {} to adjust",
                    read_timeout.as_secs(),
                    want_id,
                    server_slug,
                    mcp_read_timeout::ANYCODE_MCP_READ_TIMEOUT_SECS
                )));
            }
            Ok(Err(e)) => return Err(CoreError::LLMError(e.to_string())),
            Ok(Ok(n)) => n,
        };
        if n == 0 {
            let detail = match child.try_wait() {
                Ok(Some(st)) => format!("child exited: {st}"),
                Ok(None) => "stdout closed while child still running (unexpected)".into(),
                Err(e) => format!("try_wait: {e}"),
            };
            return Err(CoreError::LLMError(format!(
                "MCP unexpected end of stdout before JSON-RPC id={} (server={}): {}",
                want_id, server_slug, detail
            )));
        }
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
    Err(CoreError::LLMError(format!(
        "MCP: no JSON-RPC response with matching id={} after 1024 lines (server={})",
        want_id, server_slug
    )))
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
                    "clientInfo": { "name": "anycode", "version": env!("CARGO_PKG_VERSION") }
                }
            }),
        )
        .await?;

        let read_to = line_read_timeout();
        let init_resp =
            read_until_id(&mut reader, &mut child, 1, read_to, server_slug.as_str()).await?;
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

        let list_resp =
            read_until_id(&mut reader, &mut child, 2, read_to, server_slug.as_str()).await?;
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
        let read_to = line_read_timeout();
        let slug = self.server_slug.clone();
        let McpIo {
            ref mut stdin,
            ref mut reader,
            ref mut child,
            ..
        } = &mut *io;
        write_line(
            stdin,
            &json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }),
        )
        .await?;
        read_until_id(reader, child, id, read_to, slug.as_str()).await
    }

    /// Best-effort: `true` if the stdio child has not reported an exit status yet.
    pub async fn stdio_child_is_running(&self) -> bool {
        let mut io = self.io.lock().await;
        io.child.try_wait().ok().flatten().is_none()
    }

    pub async fn call_tool_named(
        &self,
        name: &str,
        arguments: Value,
    ) -> Result<ToolOutput, CoreError> {
        let start = std::time::Instant::now();
        let params = json!({ "name": name, "arguments": arguments });
        let slug = self.server_slug.as_str();
        let resp = match mcp_tools_call_wall_timeout() {
            Some(dur) => timeout(dur, self.rpc("tools/call", params))
                .await
                .map_err(|_| mcp_wall_timeout_core_error(dur, slug))??,
            None => self.rpc("tools/call", params).await?,
        };
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
        let resp = self.rpc("resources/read", json!({ "uri": uri })).await?;
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

#[cfg(test)]
mod connect_tests {
    use super::McpStdioSession;

    #[tokio::test]
    async fn mcp_stdio_connect_fails_when_shell_exits_immediately() {
        let err = match McpStdioSession::connect("exit 0", "early-exit-test").await {
            Ok(_) => panic!("expected connect failure when child produces no MCP handshake"),
            Err(e) => e,
        };
        let msg = err.to_string();
        assert!(
            msg.contains("MCP")
                || msg.contains("stdout")
                || msg.contains("exited")
                || msg.contains("Broken pipe")
                || msg.contains("broken pipe"),
            "unexpected error message: {msg}"
        );
    }
}
