//! LSP JSON-RPC over stdio（`tools-lsp`）：`ANYCODE_LSP_COMMAND` 或 `config.json` 的 `lsp.command` 启动语言服务器子进程，完成 handshake 后转发自定义请求。

use crate::lsp_root_uri::lsp_root_uri_json;
use anycode_core::prelude::*;
use serde_json::{json, Value};
use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

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
    read_timeout: Duration,
) -> Result<Value, CoreError> {
    let mut line = String::new();
    for _ in 0..512 {
        line.clear();
        timeout(read_timeout, reader.read_line(&mut line))
            .await
            .map_err(|_| CoreError::LLMError("LSP read timeout".into()))?
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
        "LSP: no JSON-RPC response with matching id".into(),
    ))
}

#[derive(serde::Deserialize)]
struct LspToolIn {
    method: String,
    #[serde(default)]
    params: Value,
}

pub async fn lsp_forward_shell(
    input: &Value,
    command_shell: &str,
    workspace_root: Option<&Path>,
    read_timeout: Duration,
) -> Result<ToolOutput, CoreError> {
    let start = std::time::Instant::now();
    let req: LspToolIn = serde_json::from_value(input.clone()).map_err(|_| {
        CoreError::LLMError("LSP 工具输入需要字段 method（字符串），可选 params（对象）".into())
    })?;
    if req.method.trim().is_empty() {
        return Err(CoreError::LLMError("LSP method 不能为空".into()));
    }

    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command_shell)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| CoreError::LLMError(format!("LSP spawn failed: {}", e)))?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| CoreError::LLMError("LSP stdin missing".into()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| CoreError::LLMError("LSP stdout missing".into()))?;
    let mut reader = BufReader::new(stdout);

    let root_uri = lsp_root_uri_json(workspace_root);

    write_line(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": null,
                "rootUri": root_uri,
                "capabilities": {},
                "clientInfo": { "name": "anycode", "version": env!("CARGO_PKG_VERSION") }
            }
        }),
    )
    .await?;

    let init_resp = read_until_id(&mut reader, 1, read_timeout).await?;
    if let Some(err) = init_resp.get("error") {
        return Ok(ToolOutput {
            result: json!({ "lsp_error": err, "stage": "initialize" }),
            error: Some("lsp initialize failed".into()),
            duration_ms: start.elapsed().as_millis() as u64,
        });
    }

    write_line(
        &mut stdin,
        &json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }),
    )
    .await?;

    write_line(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": req.method,
            "params": req.params
        }),
    )
    .await?;

    let out = read_until_id(&mut reader, 2, read_timeout).await?;
    if let Some(err) = out.get("error") {
        return Ok(ToolOutput {
            result: json!({ "lsp_error": err }),
            error: Some("lsp request failed".into()),
            duration_ms: start.elapsed().as_millis() as u64,
        });
    }

    Ok(ToolOutput {
        result: out.get("result").cloned().unwrap_or(out),
        error: None,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

#[cfg(test)]
mod forward_tests {
    use super::lsp_forward_shell;
    use serde_json::json;
    use std::time::Duration;

    /// 最小 echo JSON-RPC 服务：对 `id` 1 / 2 各回一条响应（忽略 `initialized` 通知）。
    const FAKE_LSP_SH: &str = r##"while IFS= read -r line; do
  if echo "$line" | grep -q '"id":1'; then
    echo '{"jsonrpc":"2.0","id":1,"result":{}}'
  elif echo "$line" | grep -q '"id":2'; then
    echo '{"jsonrpc":"2.0","id":2,"result":{"ok":true}}'
  fi
done"##;

    #[tokio::test]
    async fn lsp_forward_fake_server_roundtrip() {
        let input = json!({ "method": "custom/ping", "params": {} });
        let out = lsp_forward_shell(&input, FAKE_LSP_SH, None, Duration::from_secs(5))
            .await
            .expect("lsp_forward_shell");
        assert!(out.error.is_none(), "{:?}", out);
        assert_eq!(out.result["ok"], true);
    }

    #[tokio::test]
    async fn lsp_forward_with_workspace_root_still_roundtrips() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let abs = std::fs::canonicalize(tmp.path()).expect("canonicalize");
        let input = json!({ "method": "shutdown", "params": null });
        let out = lsp_forward_shell(
            &input,
            FAKE_LSP_SH,
            Some(abs.as_path()),
            Duration::from_secs(5),
        )
        .await
        .expect("lsp_forward_shell");
        assert!(out.error.is_none(), "{:?}", out);
        assert_eq!(out.result["ok"], true);
    }
}
