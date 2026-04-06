//! 将 MCP `tools/list` 中的条目注册为独立 API 工具名 `mcp__<server>__<tool>`。

use crate::mcp_connected::McpConnected;
use anycode_core::prelude::*;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

pub struct McpProxiedTool {
    session: Arc<dyn McpConnected>,
    logical_name: String,
    mcp_tool_name: String,
    description: String,
    schema: Value,
    policy: SecurityPolicy,
}

impl McpProxiedTool {
    pub fn new(
        session: Arc<dyn McpConnected>,
        logical_name: String,
        mcp_tool_name: String,
        description: String,
        schema: Value,
    ) -> Self {
        Self {
            session,
            logical_name,
            mcp_tool_name,
            description,
            schema,
            policy: SecurityPolicy::sensitive_mutation(),
        }
    }
}

#[async_trait]
impl Tool for McpProxiedTool {
    fn name(&self) -> &str {
        &self.logical_name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn schema(&self) -> Value {
        self.schema.clone()
    }

    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Default
    }

    fn security_policy(&self) -> Option<&SecurityPolicy> {
        Some(&self.policy)
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        self.session
            .call_tool_named(&self.mcp_tool_name, input.input.clone())
            .await
    }
}
