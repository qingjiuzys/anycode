//! anyCode Tools — 内置工具注册、MCP、权限规则与任务编排（Rust）。

pub mod catalog;
pub mod claude_rules;
pub mod cron_schedule;
mod limits;
pub mod mcp_normalization;
mod paths;
pub mod permission_rule_parser;
pub mod project_templates;
mod registry;
pub mod runtime_tool_policy;
mod sandbox;
pub mod services;
pub mod shell_rule_match;
pub mod skills;
pub mod workflows;

mod agent_tools;
pub mod ask_user_question_host;
mod bash;
mod edit;
mod file_read;
mod file_write;
mod glob;
mod grep;
mod knowledge_index;
mod knowledge_scoring;
mod knowledge_tools;
mod knowledge_vectors;
mod lsp_root_uri;
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
mod mcp_read_timeout;
#[cfg(feature = "tools-mcp")]
pub mod mcp_rmcp_session;
#[cfg(feature = "tools-mcp")]
pub mod mcp_session;
#[cfg(feature = "tools-mcp")]
mod mcp_stdio;
mod mcp_tool_scan;
mod mcp_tools;
mod media_tools;
mod mode_tools;
mod notebook_edit;
mod orchestration;
mod platform_tools;
mod todo_write;
mod web_fetch;
mod web_search;

pub use ask_user_question_host::{
    AskUserQuestionHost, AskUserQuestionHostArc, AskUserQuestionHostError, AskUserQuestionOption,
    AskUserQuestionRequest, AskUserQuestionResponse,
};
pub use catalog::{
    build_default_registry, build_registry_with_services, cron_tool_profile_filters,
    explore_plan_tool_names, explore_plan_tool_names_with_skill, general_purpose_tool_names,
    iter_cli_tool_help, sidebar_tool_lines, tool_catalog, tool_catalog_entry,
    validate_default_registry, workspace_assistant_tool_names, ToolCatalogEntry, DEFAULT_TOOL_IDS,
    EXPLORE_PLAN_TOOL_IDS,
};
pub use claude_rules::CompiledClaudePermissionRules;
pub use cron_schedule::{
    format_next_fire_human, next_fire_utc_from_stored_schedule, normalize_cron_schedule_expr,
    parse_natural_cron_hint, resolve_schedule_timezone, validate_cron_schedule_expr,
    wall_clock_cron_to_utc_storage, wall_clock_cron_to_utc_storage_for_timezone,
    wall_clock_cron_to_utc_storage_in_iana, NaturalCronResult, ScheduleTimezone,
};
pub use knowledge_scoring::score_knowledge_chunk;
pub use knowledge_vectors::{
    merge_hybrid_knowledge_hits, rebuild_project_vectors, search_project_vectors,
    vector_chunk_count, vector_store_path, vectors_feature_enabled, VectorHit,
};
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
#[cfg(feature = "tools-mcp")]
pub use mcp_tool_scan::scan_listed_tools;
pub use mcp_tool_scan::{scan_tool_entry, McpToolScanFinding};
pub use project_templates::{
    apply_project_template, list_project_templates, resolve_project_templates_root,
    ApplyTemplateOptions, ApplyTemplateResult, ProjectTemplateManifest,
};
pub use registry::build_registry;
pub use runtime_tool_policy::{
    detect_ci_environment, resolve_runtime_tool_filters, RuntimeToolPolicyInput,
    ToolExecutionSurface, ToolPolicyProfiles,
};
pub use services::{
    append_cron_job_to_orchestration_file, read_cron_jobs_from_orchestration_file, CronJob,
    CronJobCreateOptions, LspConnectionConfig, ToolRegistryDeps, ToolServices,
};
pub use skills::{
    default_skill_roots, install_skill, install_starter_skills, resolve_skills_starter_dir,
    truncate_skill_output, vet_skill_by_id, vet_skill_dir, SkillCatalog, SkillInstallResult,
    SkillMeta, SkillVetReport, SkillsGovernance, MAX_SKILL_OUTPUT_BYTES,
};
