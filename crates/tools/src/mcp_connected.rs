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

    /// 重新拉取 `tools/list` 并返回最新列表（Claude Code `RefreshMcpTools` 语义）。
    /// 默认实现不支持；stdio/rmcp 会话覆盖后真正向服务器重新请求。
    async fn refresh_tools(&self) -> Result<Value, CoreError> {
        Err(CoreError::LLMError(
            "refresh_tools not supported by this MCP session type".into(),
        ))
    }
}
