//! 统一 stdio 与远程 MCP 会话，供工具代理与 `mcp` 工具入口使用。

use anycode_core::prelude::*;
use async_trait::async_trait;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct McpListedTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[async_trait]
pub trait McpConnected: Send + Sync {
    fn server_slug(&self) -> &str;
    fn listed_tools(&self) -> &[McpListedTool];
    async fn call_tool_named(&self, name: &str, arguments: Value) -> Result<ToolOutput, CoreError>;
    async fn resources_list(&self, server: Option<&str>) -> Result<Value, CoreError>;
    async fn resources_read(&self, uri: &str) -> Result<Value, CoreError>;
}
