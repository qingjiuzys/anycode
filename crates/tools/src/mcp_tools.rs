//! MCP 相关工具：`mcp`（通用入口）、`ListMcpResourcesTool`、`ReadMcpResourceTool`、`McpAuth`（兼容静态名）。

use crate::services::ToolServices;
use anycode_core::prelude::*;
use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Instant;

#[cfg(feature = "tools-mcp")]
fn pick_mcp_session(
    services: &ToolServices,
    input: &serde_json::Value,
) -> Result<Option<Arc<dyn crate::mcp_connected::McpConnected>>, ToolOutput> {
    use crate::mcp_normalization::normalize_name_for_mcp;
    let sessions = services.mcp_sessions();
    if sessions.is_empty() {
        return Ok(None);
    }
    let want = input
        .get("mcp_server")
        .or_else(|| input.get("server"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if sessions.len() == 1 {
        return Ok(Some(sessions[0].clone()));
    }
    match want {
        Some(w) => {
            let wn = normalize_name_for_mcp(w);
            Ok(sessions
                .into_iter()
                .find(|s| s.server_slug() == wn.as_str()))
        }
        None => Err(ToolOutput {
            result: serde_json::json!({
                "error": "multiple MCP servers connected: set mcp_server (or server) to the server slug"
            }),
            error: Some("mcp_server required".into()),
            duration_ms: 0,
        }),
    }
}

pub struct McpTool {
    #[cfg_attr(not(feature = "tools-mcp"), allow(dead_code))]
    services: Arc<ToolServices>,
    security_policy: SecurityPolicy,
}

impl McpTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            services,
            security_policy: SecurityPolicy::sensitive_mutation(),
        }
    }
}

#[async_trait]
impl Tool for McpTool {
    fn name(&self) -> &str {
        "mcp"
    }

    fn description(&self) -> &str {
        "Invoke an MCP server tool via JSON-RPC tools/call. With multiple stdio servers, set mcp_server (or server) to the configured slug. Build with --features tools-mcp and set ANYCODE_MCP_COMMAND or ANYCODE_MCP_SERVERS."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "mcp_server": { "type": "string", "description": "MCP server slug when using ANYCODE_MCP_SERVERS" },
                "server": { "type": "string", "description": "Alias of mcp_server" },
                "name": { "type": "string", "description": "MCP tool name to call" },
                "tool": { "type": "string", "description": "Alias of name" },
                "arguments": { "type": "object", "description": "Arguments for tools/call" }
            },
            "additionalProperties": true
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
        #[cfg(feature = "tools-mcp")]
        {
            match pick_mcp_session(&self.services, &input.input) {
                Ok(Some(sess)) => {
                    let name = input
                        .input
                        .get("name")
                        .or_else(|| input.input.get("tool"))
                        .and_then(|v| v.as_str())
                        .map(str::trim)
                        .filter(|s| !s.is_empty());
                    if let Some(n) = name {
                        let arguments = input
                            .input
                            .get("arguments")
                            .cloned()
                            .unwrap_or_else(|| serde_json::json!({}));
                        return sess.call_tool_named(n, arguments).await;
                    }
                }
                Ok(None) => {}
                Err(mut o) => {
                    o.duration_ms = start.elapsed().as_millis() as u64;
                    return Ok(o);
                }
            }
            if let Ok(cmd) = std::env::var("ANYCODE_MCP_COMMAND") {
                let cmd = cmd.trim();
                if !cmd.is_empty() {
                    return crate::mcp_stdio::mcp_tools_call_shell(&input.input, cmd).await;
                }
            }
        }

        Ok(ToolOutput {
            result: serde_json::json!({
                "status": "unsupported",
                "error": "MCP bridge not configured",
                "hint": "Build with --features tools-mcp and set ANYCODE_MCP_COMMAND or ANYCODE_MCP_SERVERS.",
                "echo": input.input
            }),
            error: Some("mcp not configured".into()),
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

pub struct ListMcpResourcesTool {
    #[cfg_attr(not(feature = "tools-mcp"), allow(dead_code))]
    services: Arc<ToolServices>,
}

impl ListMcpResourcesTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self { services }
    }
}

#[async_trait]
impl Tool for ListMcpResourcesTool {
    fn name(&self) -> &str {
        "ListMcpResourcesTool"
    }

    fn description(&self) -> &str {
        "List resources from MCP servers."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "server": { "type": "string", "description": "MCP server slug（多服务器时必填）" },
                "mcp_server": { "type": "string", "description": "同 server" }
            }
        })
    }

    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Auto
    }

    fn security_policy(&self) -> Option<&SecurityPolicy> {
        None
    }

    #[cfg_attr(not(feature = "tools-mcp"), allow(unused_variables))]
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let start = Instant::now();
        #[cfg(feature = "tools-mcp")]
        {
            use crate::mcp_normalization::normalize_name_for_mcp;
            let sessions = self.services.mcp_sessions();
            if !sessions.is_empty() {
                let want = input
                    .input
                    .get("mcp_server")
                    .or_else(|| input.input.get("server"))
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(normalize_name_for_mcp);
                let mut servers_out = Vec::new();
                for sess in sessions {
                    if let Some(ref w) = want {
                        if sess.server_slug() != w.as_str() {
                            continue;
                        }
                    }
                    match sess.resources_list(None).await {
                        Ok(v) => servers_out.push(serde_json::json!({
                            "mcp_server": sess.server_slug(),
                            "result": v
                        })),
                        Err(e) => servers_out.push(serde_json::json!({
                            "mcp_server": sess.server_slug(),
                            "error": e.to_string()
                        })),
                    }
                }
                return Ok(ToolOutput {
                    result: serde_json::json!({ "servers": servers_out }),
                    error: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        }
        Ok(ToolOutput {
            result: serde_json::json!({
                "status": "unsupported",
                "resources": [],
                "info": "MCP not connected"
            }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[derive(Deserialize)]
struct ReadMcpIn {
    #[serde(default)]
    uri: String,
}

pub struct ReadMcpResourceTool {
    #[cfg_attr(not(feature = "tools-mcp"), allow(dead_code))]
    services: Arc<ToolServices>,
}

impl ReadMcpResourceTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self { services }
    }
}

#[async_trait]
impl Tool for ReadMcpResourceTool {
    fn name(&self) -> &str {
        "ReadMcpResourceTool"
    }

    fn description(&self) -> &str {
        "Read an MCP resource by URI."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "uri": { "type": "string" },
                "mcp_server": { "type": "string", "description": "Optional: only try this MCP server slug first" }
            },
            "required": ["uri"]
        })
    }

    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Default
    }

    fn security_policy(&self) -> Option<&SecurityPolicy> {
        None
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let start = Instant::now();
        let r: ReadMcpIn =
            serde_json::from_value(input.input.clone()).unwrap_or(ReadMcpIn { uri: String::new() });
        if r.uri.trim().is_empty() {
            return Ok(ToolOutput {
                result: serde_json::json!({ "error": "uri required" }),
                error: Some("uri required".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
        #[cfg(feature = "tools-mcp")]
        {
            use crate::mcp_normalization::normalize_name_for_mcp;
            let sessions = self.services.mcp_sessions();
            if !sessions.is_empty() {
                let prefer = input
                    .input
                    .get("mcp_server")
                    .or_else(|| input.input.get("server"))
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(normalize_name_for_mcp);
                let ordered: Vec<_> = if let Some(ref w) = prefer {
                    let mut first: Vec<_> = sessions
                        .iter()
                        .filter(|s| s.server_slug() == w.as_str())
                        .cloned()
                        .collect();
                    let mut rest: Vec<_> = sessions
                        .iter()
                        .filter(|s| s.server_slug() != w.as_str())
                        .cloned()
                        .collect();
                    first.append(&mut rest);
                    first
                } else {
                    sessions
                };
                let mut last_err = None::<String>;
                for sess in ordered {
                    match sess.resources_read(r.uri.trim()).await {
                        Ok(v) => {
                            return Ok(ToolOutput {
                                result: serde_json::json!({
                                    "mcp_server": sess.server_slug(),
                                    "result": v
                                }),
                                error: None,
                                duration_ms: start.elapsed().as_millis() as u64,
                            });
                        }
                        Err(e) => last_err = Some(e.to_string()),
                    }
                }
                return Ok(ToolOutput {
                    result: serde_json::json!({
                        "error": last_err.unwrap_or_else(|| "resources/read failed".into()),
                        "uri": r.uri
                    }),
                    error: Some("resources/read failed".into()),
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        }
        Ok(ToolOutput {
            result: serde_json::json!({
                "status": "unsupported",
                "error": "MCP not configured",
                "uri": r.uri
            }),
            error: Some("not configured".into()),
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

pub struct McpAuthTool {
    #[cfg(feature = "tools-mcp")]
    services: Arc<ToolServices>,
    auth_policy: SecurityPolicy,
}

impl McpAuthTool {
    pub fn new(
        #[cfg_attr(not(feature = "tools-mcp"), allow(unused_variables))] services: Arc<
            ToolServices,
        >,
    ) -> Self {
        Self {
            #[cfg(feature = "tools-mcp")]
            services,
            auth_policy: SecurityPolicy::sensitive_mutation(),
        }
    }
}

#[async_trait]
impl Tool for McpAuthTool {
    fn name(&self) -> &str {
        "McpAuth"
    }

    fn description(&self) -> &str {
        "MCP OAuth / session handshake: forwards to the server's `authenticate` tool when stdio MCP is connected. Prefer tool name mcp__<slug>__authenticate when registered. With multiple servers, set mcp_server or server."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "server": { "type": "string", "description": "MCP server slug" },
                "mcp_server": { "type": "string", "description": "Alias of server" }
            },
            "additionalProperties": true
        })
    }

    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Default
    }

    fn security_policy(&self) -> Option<&SecurityPolicy> {
        Some(&self.auth_policy)
    }

    #[cfg_attr(not(feature = "tools-mcp"), allow(unused_variables))]
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let start = Instant::now();
        #[cfg(feature = "tools-mcp")]
        {
            match pick_mcp_session(&self.services, &input.input) {
                Ok(Some(sess)) => {
                    let mut args = input.input.clone();
                    if let Some(o) = args.as_object_mut() {
                        o.remove("server");
                        o.remove("mcp_server");
                    }
                    return sess.call_tool_named("authenticate", args).await;
                }
                Ok(None) => {}
                Err(mut o) => {
                    o.duration_ms = start.elapsed().as_millis() as u64;
                    return Ok(o);
                }
            }
        }
        Ok(ToolOutput {
            result: serde_json::json!({
                "status": "unsupported",
                "message": "MCP not connected (build with --features tools-mcp and set ANYCODE_MCP_COMMAND / ANYCODE_MCP_SERVERS)"
            }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}
