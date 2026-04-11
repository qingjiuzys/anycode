//! 默认工具集：id 常量、`DEFAULT_TOOL_IDS`、`SECURITY_SENSITIVE_TOOL_IDS`、`build_registry` / `build_default_registry`、校验与 CLI 文案。

use crate::registry::build_registry;
use crate::services::ToolRegistryDeps;
use anycode_core::prelude::*;
use std::collections::HashMap;

pub const TOOL_FILE_READ: &str = "FileRead";
pub const TOOL_FILE_WRITE: &str = "FileWrite";
pub const TOOL_BASH: &str = "Bash";
pub const TOOL_GLOB: &str = "Glob";
pub const TOOL_GREP: &str = "Grep";
pub const TOOL_EDIT: &str = "Edit";
pub const TOOL_NOTEBOOK_EDIT: &str = "NotebookEdit";
pub const TOOL_TODO_WRITE: &str = "TodoWrite";
pub const TOOL_WEB_FETCH: &str = "WebFetch";
pub const TOOL_WEB_SEARCH: &str = "WebSearch";
pub const TOOL_MCP: &str = "mcp";
pub const TOOL_LIST_MCP_RESOURCES: &str = "ListMcpResourcesTool";
pub const TOOL_READ_MCP_RESOURCE: &str = "ReadMcpResourceTool";
pub const TOOL_MCP_AUTH: &str = "McpAuth";
pub const TOOL_LSP: &str = "LSP";
pub const TOOL_AGENT: &str = "Agent";
pub const TOOL_SKILL: &str = "Skill";
pub const TOOL_SEND_MESSAGE: &str = "SendMessage";
pub const TOOL_LEGACY_TASK_AGENT: &str = "Task";
pub const TOOL_TASK_CREATE: &str = "TaskCreate";
pub const TOOL_TASK_UPDATE: &str = "TaskUpdate";
pub const TOOL_TASK_LIST: &str = "TaskList";
pub const TOOL_TASK_GET: &str = "TaskGet";
pub const TOOL_TASK_STOP: &str = "TaskStop";
pub const TOOL_TASK_OUTPUT: &str = "TaskOutput";
pub const TOOL_TEAM_CREATE: &str = "TeamCreate";
pub const TOOL_TEAM_DELETE: &str = "TeamDelete";
pub const TOOL_CRON_CREATE: &str = "CronCreate";
pub const TOOL_CRON_DELETE: &str = "CronDelete";
pub const TOOL_CRON_LIST: &str = "CronList";
pub const TOOL_REMOTE_TRIGGER: &str = "RemoteTrigger";
pub const TOOL_ENTER_PLAN: &str = "EnterPlanMode";
pub const TOOL_EXIT_PLAN: &str = "ExitPlanMode";
pub const TOOL_ENTER_WORKTREE: &str = "EnterWorktree";
pub const TOOL_EXIT_WORKTREE: &str = "ExitWorktree";
pub const TOOL_TOOL_SEARCH: &str = "ToolSearch";
pub const TOOL_SLEEP: &str = "Sleep";
pub const TOOL_STRUCTURED_OUTPUT: &str = "StructuredOutput";
pub const TOOL_POWERSHELL: &str = "PowerShell";
pub const TOOL_CONFIG: &str = "Config";
pub const TOOL_SEND_USER_MESSAGE: &str = "SendUserMessage";
pub const TOOL_BRIEF: &str = "Brief";
pub const TOOL_ASK_USER_QUESTION: &str = "AskUserQuestion";
pub const TOOL_REPL: &str = "REPL";

/// general-purpose Agent 暴露的完整工具 id（与 `build_registry` 插入集合一致）。
pub const DEFAULT_TOOL_IDS: &[&str] = &[
    TOOL_FILE_READ,
    TOOL_FILE_WRITE,
    TOOL_BASH,
    TOOL_GLOB,
    TOOL_GREP,
    TOOL_EDIT,
    TOOL_NOTEBOOK_EDIT,
    TOOL_TODO_WRITE,
    TOOL_WEB_FETCH,
    TOOL_WEB_SEARCH,
    TOOL_MCP,
    TOOL_LIST_MCP_RESOURCES,
    TOOL_READ_MCP_RESOURCE,
    TOOL_MCP_AUTH,
    TOOL_LSP,
    TOOL_AGENT,
    TOOL_SKILL,
    TOOL_SEND_MESSAGE,
    TOOL_LEGACY_TASK_AGENT,
    TOOL_TASK_CREATE,
    TOOL_TASK_UPDATE,
    TOOL_TASK_LIST,
    TOOL_TASK_GET,
    TOOL_TASK_STOP,
    TOOL_TASK_OUTPUT,
    TOOL_TEAM_CREATE,
    TOOL_TEAM_DELETE,
    TOOL_CRON_CREATE,
    TOOL_CRON_DELETE,
    TOOL_CRON_LIST,
    TOOL_REMOTE_TRIGGER,
    TOOL_ENTER_PLAN,
    TOOL_EXIT_PLAN,
    TOOL_ENTER_WORKTREE,
    TOOL_EXIT_WORKTREE,
    TOOL_TOOL_SEARCH,
    TOOL_SLEEP,
    TOOL_STRUCTURED_OUTPUT,
    TOOL_POWERSHELL,
    TOOL_CONFIG,
    TOOL_SEND_USER_MESSAGE,
    TOOL_BRIEF,
    TOOL_ASK_USER_QUESTION,
    TOOL_REPL,
];

/// 需在 CLI `bootstrap` 中套用 `SecurityPolicy::sensitive_mutation()` 的工具 id（与 `FileWrite` / `Bash` 的专用策略并列）。
/// **新增可写/外链/编排类工具时同步更新本表**，并跑 `catalog` 单测校验子集关系。
pub const SECURITY_SENSITIVE_TOOL_IDS: &[&str] = &[
    TOOL_EDIT,
    TOOL_NOTEBOOK_EDIT,
    TOOL_WEB_FETCH,
    TOOL_WEB_SEARCH,
    TOOL_MCP,
    TOOL_MCP_AUTH,
    TOOL_LSP,
    TOOL_AGENT,
    TOOL_SKILL,
    TOOL_SEND_MESSAGE,
    TOOL_LEGACY_TASK_AGENT,
    TOOL_TASK_CREATE,
    TOOL_TASK_UPDATE,
    TOOL_TASK_STOP,
    TOOL_TEAM_CREATE,
    TOOL_TEAM_DELETE,
    TOOL_CRON_CREATE,
    TOOL_CRON_DELETE,
    TOOL_REMOTE_TRIGGER,
    TOOL_ENTER_WORKTREE,
    TOOL_EXIT_WORKTREE,
    TOOL_POWERSHELL,
    TOOL_CONFIG,
    TOOL_ASK_USER_QUESTION,
    TOOL_REPL,
];

pub const EXPLORE_PLAN_TOOL_IDS: [&str; 4] = [TOOL_FILE_READ, TOOL_GLOB, TOOL_GREP, TOOL_BASH];

pub fn general_purpose_tool_names() -> Vec<ToolName> {
    DEFAULT_TOOL_IDS.iter().map(|s| (*s).to_string()).collect()
}

pub fn explore_plan_tool_names() -> Vec<ToolName> {
    explore_plan_tool_names_with_skill(false)
}

/// When `include_skill` is true (config `skills.expose_on_explore_plan`), `Skill` is exposed to explore/plan agents.
pub fn explore_plan_tool_names_with_skill(include_skill: bool) -> Vec<ToolName> {
    let mut v: Vec<ToolName> = EXPLORE_PLAN_TOOL_IDS
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    if include_skill {
        v.push(TOOL_SKILL.to_string());
    }
    v.sort();
    v
}

pub fn workspace_assistant_tool_names(include_skill: bool) -> Vec<ToolName> {
    let mut out = vec![
        TOOL_FILE_READ.to_string(),
        TOOL_GLOB.to_string(),
        TOOL_GREP.to_string(),
        TOOL_BASH.to_string(),
        TOOL_TASK_LIST.to_string(),
        TOOL_TASK_GET.to_string(),
        TOOL_CRON_LIST.to_string(),
        TOOL_TOOL_SEARCH.to_string(),
        TOOL_STRUCTURED_OUTPUT.to_string(),
        TOOL_SEND_USER_MESSAGE.to_string(),
        TOOL_BRIEF.to_string(),
        TOOL_ASK_USER_QUESTION.to_string(),
    ];
    if include_skill {
        out.push(TOOL_SKILL.to_string());
    }
    out.sort();
    out
}

/// 使用共享 `ToolServices` 构建注册表（推荐：`bootstrap` 传入单例 Arc）。
pub fn build_registry_with_services(
    sandbox_mode: bool,
    services: std::sync::Arc<crate::services::ToolServices>,
) -> HashMap<ToolName, Box<dyn Tool>> {
    build_registry(&ToolRegistryDeps {
        sandbox_mode,
        services,
    })
}

/// 默认注册表：独立 `ToolServices`（每调用一次一个新实例；测试/简单场景可用）。
pub fn build_default_registry(sandbox_mode: bool) -> HashMap<ToolName, Box<dyn Tool>> {
    build_registry(&ToolRegistryDeps::minimal(sandbox_mode))
}

pub fn validate_default_registry(tools: &HashMap<ToolName, Box<dyn Tool>>) -> anyhow::Result<()> {
    for id in DEFAULT_TOOL_IDS {
        if !tools.contains_key(*id) {
            anyhow::bail!("default tool registry missing tool {:?}", id);
        }
    }
    Ok(())
}

/// `anycode run` / `list_tools` 英文说明（核心工具详述，其余一行）。
pub fn iter_cli_tool_help() -> impl Iterator<Item = (&'static str, &'static str)> {
    [
        (
            TOOL_FILE_READ,
            "Read file contents (supports text, images, PDFs, Jupyter notebooks)",
        ),
        (TOOL_FILE_WRITE, "Create or overwrite files"),
        (TOOL_BASH, "Execute shell commands"),
        (TOOL_GLOB, "Find files by pattern (supports **/*.ts)"),
        (TOOL_GREP, "Search file contents using ripgrep"),
        (
            TOOL_EDIT,
            "Partial file edit (string replace, Claude Code Edit tool)",
        ),
        (TOOL_NOTEBOOK_EDIT, "Edit Jupyter .ipynb cells"),
        (TOOL_TODO_WRITE, "Session todo checklist"),
        (TOOL_WEB_FETCH, "HTTP(S) fetch with size limit"),
        (TOOL_WEB_SEARCH, "Web search (DDG or custom endpoint)"),
        (
            TOOL_MCP,
            "MCP tools/call passthrough (stdio with tools-mcp + ANYCODE_MCP_* env)",
        ),
        (TOOL_LSP, "LSP queries (stub)"),
        (
            TOOL_AGENT,
            "Nested agent (Claude-compatible: subagent_type, description, cwd; status/agent_id/output_file in result)",
        ),
        (
            TOOL_SKILL,
            "Run a discovered skill's `run` script (SKILL.md roots; timeout and env from skills.*)",
        ),
        (
            TOOL_TASK_CREATE,
            "Create orchestration task record (persists with ~/.anycode/tasks/orchestration.json when home dir exists)",
        ),
        (
            TOOL_TASK_OUTPUT,
            "Task record + output.log path/tail when id is a runtime execution UUID (e.g. nested_task_id from Agent)",
        ),
        (
            TOOL_CRON_CREATE,
            "Register cron-like job (persisted under ~/.anycode/tasks/orchestration.json; executed by `anycode scheduler` when running)",
        ),
        (TOOL_ENTER_WORKTREE, "git worktree add + record path"),
        (TOOL_STRUCTURED_OUTPUT, "Structured JSON passthrough"),
        (TOOL_SEND_USER_MESSAGE, "User-visible message payload"),
        (TOOL_BRIEF, "Alias of SendUserMessage"),
    ]
    .into_iter()
}

pub fn sidebar_tool_lines() -> Vec<String> {
    vec![
        "FileRead / FileWrite / Edit / NotebookEdit — 文件".to_string(),
        "Bash / PowerShell — Shell".to_string(),
        "Glob / Grep — 搜索".to_string(),
        "WebFetch / WebSearch — 网络".to_string(),
        "Task* / Team* / Cron* — 编排（通常落盘 orchestration.json）".to_string(),
        "mcp / LSP / Agent / Skill — 扩展（LSP 部分能力）".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explore_plan_subset_of_default() {
        for id in EXPLORE_PLAN_TOOL_IDS {
            assert!(
                DEFAULT_TOOL_IDS.contains(&id),
                "{id} must be in DEFAULT_TOOL_IDS"
            );
        }
    }

    #[test]
    fn security_sensitive_tools_are_in_default_registry() {
        for id in SECURITY_SENSITIVE_TOOL_IDS {
            assert!(
                DEFAULT_TOOL_IDS.contains(id),
                "{id} must be in DEFAULT_TOOL_IDS"
            );
        }
    }

    #[test]
    fn security_sensitive_tools_no_duplicates() {
        let mut s = SECURITY_SENSITIVE_TOOL_IDS.to_vec();
        let n = s.len();
        s.sort_unstable();
        s.dedup();
        assert_eq!(
            s.len(),
            n,
            "SECURITY_SENSITIVE_TOOL_IDS must not repeat entries"
        );
    }

    #[test]
    fn build_and_validate_default_registry() {
        let m = build_default_registry(false);
        validate_default_registry(&m).unwrap();
    }

    #[test]
    fn security_sensitive_tools_registered_in_built_registry() {
        let m = build_default_registry(false);
        for id in SECURITY_SENSITIVE_TOOL_IDS {
            assert!(
                m.contains_key(*id),
                "{id} must be registered in default registry map"
            );
        }
    }
}
