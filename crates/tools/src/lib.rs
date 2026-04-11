//! anyCode Tools — 内置工具注册、MCP、权限规则与任务编排（Rust）。

pub mod catalog;
pub mod claude_rules;
mod limits;
pub mod mcp_normalization;
mod paths;
pub mod permission_rule_parser;
mod registry;
mod sandbox;
pub mod services;
pub mod shell_rule_match;
pub mod skills;
pub mod workflows;

mod agent_tools;
mod bash;
mod edit;
mod file_read;
mod file_write;
mod glob;
mod grep;
#[cfg(feature = "tools-lsp")]
mod lsp_stdio;
mod lsp_tool;
#[cfg(feature = "tools-mcp")]
pub mod mcp_connected;
#[cfg(feature = "tools-mcp")]
pub mod mcp_legacy_sse_session;
#[cfg(feature = "tools-mcp-oauth")]
mod mcp_oauth_login;
#[cfg(feature = "tools-mcp")]
mod mcp_oauth_store;
#[cfg(feature = "tools-mcp")]
mod mcp_proxied_tool;
#[cfg(feature = "tools-mcp")]
pub mod mcp_rmcp_session;
#[cfg(feature = "tools-mcp")]
pub mod mcp_session;
#[cfg(feature = "tools-mcp")]
mod mcp_stdio;
mod mcp_tools;
mod mode_tools;
mod notebook_edit;
mod orchestration;
mod platform_tools;
mod todo_write;
mod web_fetch;
mod web_search;

pub use catalog::{
    build_default_registry, build_registry_with_services, explore_plan_tool_names,
    explore_plan_tool_names_with_skill, general_purpose_tool_names, iter_cli_tool_help,
    sidebar_tool_lines, validate_default_registry, workspace_assistant_tool_names,
    DEFAULT_TOOL_IDS, EXPLORE_PLAN_TOOL_IDS,
};
pub use claude_rules::CompiledClaudePermissionRules;
#[cfg(feature = "tools-mcp")]
pub use mcp_connected::McpListedTool;
#[cfg(feature = "tools-mcp")]
pub use mcp_legacy_sse_session::McpLegacySseSession;
pub use mcp_normalization::{
    blanket_deny_rule_matches_tool, build_mcp_tool_name, mcp_info_from_string,
    normalize_name_for_mcp,
};
#[cfg(feature = "tools-mcp-oauth")]
pub use mcp_oauth_login::{mcp_oauth_login, McpOAuthLoginError, McpOAuthLoginOptions};
#[cfg(feature = "tools-mcp")]
pub use mcp_rmcp_session::McpRmcpSession;
pub use registry::build_registry;
pub use services::{
    read_cron_jobs_from_orchestration_file, CronJob, ToolRegistryDeps, ToolServices,
};
pub use skills::{
    default_skill_roots, truncate_skill_output, SkillCatalog, SkillMeta, MAX_SKILL_OUTPUT_BYTES,
};
