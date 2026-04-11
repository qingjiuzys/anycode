//! Claude-style tool permission gating (compiled deny/allow/ask rules, optional MCP defer).

use std::collections::HashSet;
use std::sync::{Arc, Mutex as StdMutex};

use anycode_tools::CompiledClaudePermissionRules;

/// Tool permission gating (deny/allow/ask compiled rules + optional MCP first-turn hide).
#[derive(Default)]
pub struct AgentClaudeToolGating {
    pub rules: Option<Arc<CompiledClaudePermissionRules>>,
    pub defer_mcp_tools: bool,
    pub mcp_defer_allowlist: Option<Arc<StdMutex<HashSet<String>>>>,
}
