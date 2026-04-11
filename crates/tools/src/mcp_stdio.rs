//! MCP JSON-RPC over stdio（`tools-mcp`）：`ANYCODE_MCP_COMMAND` 启动子进程，完成 `initialize` 后转发 `tools/call`。

use anycode_core::prelude::*;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

const READ_TIMEOUT: Duration = Duration::from_secs(60);

async fn write_line(stdin: &mut tokio::process::ChildStdin, v: &Value) -> Result<(), CoreError> {
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
    for _ in 0..512 {
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

/// 解析工具输入：`name` / `tool` + 可选 `arguments`。
fn mcp_name_and_args(input: &Value) -> Result<(String, Value), CoreError> {
    let name = input
        .get("name")
        .or_else(|| input.get("tool"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .ok_or_else(|| {
            CoreError::LLMError("MCP tools/call 需要 JSON 字段 name 或 tool（非空字符串）".into())
        })?;
    let arguments = input.get("arguments").cloned().unwrap_or_else(|| json!({}));
    Ok((name, arguments))
}

pub async fn mcp_tools_call_shell(
    input: &Value,
    command_shell: &str,
) -> Result<ToolOutput, CoreError> {
    let start = std::time::Instant::now();
    let (tool_name, arguments) = mcp_name_and_args(input)?;

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

    let init_resp = read_until_id(&mut reader, 1).await?;
    if let Some(err) = init_resp.get("error") {
        return Ok(ToolOutput {
            result: json!({ "mcp_error": err, "stage": "initialize" }),
            error: Some("mcp initialize failed".into()),
            duration_ms: start.elapsed().as_millis() as u64,
        });
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
            "method": "tools/call",
            "params": { "name": tool_name, "arguments": arguments }
        }),
    )
    .await?;

    let call_resp = read_until_id(&mut reader, 2).await?;
    if let Some(err) = call_resp.get("error") {
        return Ok(ToolOutput {
            result: json!({ "mcp_error": err }),
            error: Some("mcp tools/call failed".into()),
            duration_ms: start.elapsed().as_millis() as u64,
        });
    }

    Ok(ToolOutput {
        result: call_resp.get("result").cloned().unwrap_or(call_resp),
        error: None,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}
