//! 将 MCP `tools/list` 中的条目注册为独立 API 工具名 `mcp__<server>__<tool>`。

use crate::mcp_connected::McpConnected;
use anycode_core::prelude::*;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::{Mutex, OnceLock};

static MCP_SERVER_COUNTS: OnceLock<Mutex<HashMap<String, usize>>> = OnceLock::new();

pub struct McpProxiedTool {
    session: Arc<dyn McpConnected>,
    server_slug: String,
    logical_name: String,
    mcp_tool_name: String,
    description: String,
    schema: Value,
    policy: SecurityPolicy,
}

impl McpProxiedTool {
    pub fn new(
        session: Arc<dyn McpConnected>,
        server_slug: String,
        logical_name: String,
        mcp_tool_name: String,
        description: String,
        schema: Value,
    ) -> Self {
        Self {
            session,
            server_slug,
            logical_name,
            mcp_tool_name,
            description,
            schema,
            policy: SecurityPolicy::sensitive_mutation(),
        }
    }

    fn governance_check(&self) -> Result<(), CoreError> {
        mcp_governance_check(&self.server_slug, &self.logical_name, &self.mcp_tool_name)
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
        self.governance_check()?;
        self.session
            .call_tool_named(&self.mcp_tool_name, input.input.clone())
            .await
    }
}

pub(crate) fn mcp_governance_check(
    server: &str,
    logical_name: &str,
    mcp_tool_name: &str,
) -> Result<(), CoreError> {
    if mcp_strict_enabled() && !mcp_tool_allowed(server, logical_name, mcp_tool_name) {
        return Err(CoreError::PermissionDenied(format!(
            "MCP strict mode denied {logical_name} (server={server}, tool={mcp_tool_name})"
        )));
    }
    if let Some(max) = mcp_max_calls_per_server() {
        let counts = MCP_SERVER_COUNTS.get_or_init(|| Mutex::new(HashMap::new()));
        let mut counts = counts.lock().expect("mcp server call counts");
        let count = counts.entry(server.to_string()).or_insert(0);
        if *count >= max {
            return Err(CoreError::PermissionDenied(format!(
                "MCP server {server} exceeded call quota {max} (set ANYCODE_MCP_MAX_CALLS_PER_SERVER to adjust)"
            )));
        }
        *count += 1;
    }
    Ok(())
}

fn mcp_strict_enabled() -> bool {
    matches!(
        std::env::var("ANYCODE_MCP_STRICT").as_deref(),
        Ok("1") | Ok("true") | Ok("yes") | Ok("on")
    )
}

fn mcp_max_calls_per_server() -> Option<usize> {
    std::env::var("ANYCODE_MCP_MAX_CALLS_PER_SERVER")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|v| *v > 0)
}

fn mcp_tool_allowed(server: &str, logical_name: &str, mcp_tool_name: &str) -> bool {
    let allow = std::env::var("ANYCODE_MCP_ALLOWED_TOOLS").unwrap_or_default();
    let allow: HashSet<String> = allow
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    if allow.is_empty() {
        return false;
    }
    allow.contains(logical_name)
        || allow.contains(mcp_tool_name)
        || allow.contains(&format!("{server}:{mcp_tool_name}"))
}

#[cfg(test)]
mod tests {
    use super::mcp_tool_allowed;

    #[test]
    fn allowlist_accepts_logical_and_server_tool_names() {
        std::env::set_var("ANYCODE_MCP_ALLOWED_TOOLS", "mcp__s__search,api:list");
        assert!(mcp_tool_allowed("s", "mcp__s__search", "search"));
        assert!(mcp_tool_allowed("api", "mcp__api__list", "list"));
        assert!(!mcp_tool_allowed("api", "mcp__api__write", "write"));
        std::env::remove_var("ANYCODE_MCP_ALLOWED_TOOLS");
    }
}
