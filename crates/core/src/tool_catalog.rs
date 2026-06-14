//! Shared tool catalog metadata.

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
pub const TOOL_SEND_WECHAT_MESSAGE: &str = "SendWeChatMessage";
pub const TOOL_QUERY_WECHAT_HISTORY: &str = "QueryWeChatHistory";
pub const TOOL_BRIEF: &str = "Brief";
pub const TOOL_ASK_USER_QUESTION: &str = "AskUserQuestion";
pub const TOOL_REPL: &str = "REPL";
pub const TOOL_KNOWLEDGE_SEARCH: &str = "KnowledgeSearch";

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
    TOOL_SEND_WECHAT_MESSAGE,
    TOOL_QUERY_WECHAT_HISTORY,
    TOOL_BRIEF,
    TOOL_ASK_USER_QUESTION,
    TOOL_REPL,
    TOOL_KNOWLEDGE_SEARCH,
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
    TOOL_SEND_WECHAT_MESSAGE,
    TOOL_QUERY_WECHAT_HISTORY,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolCatalogEntry {
    pub id: &'static str,
    pub category: &'static str,
    pub risk_tier: &'static str,
    pub default_agents: &'static [&'static str],
    pub requires_approval: bool,
    pub audit_level: &'static str,
}

const ALL_AGENTS: &[&str] = &["general-purpose", "workspace", "goal"];
const EXPLORE_PLAN_AGENTS: &[&str] = &["explore", "plan"];
const ORCHESTRATION_AGENTS: &[&str] = &["general-purpose", "workspace"];

const fn tool_entry(
    id: &'static str,
    category: &'static str,
    risk_tier: &'static str,
    default_agents: &'static [&'static str],
    requires_approval: bool,
    audit_level: &'static str,
) -> ToolCatalogEntry {
    ToolCatalogEntry {
        id,
        category,
        risk_tier,
        default_agents,
        requires_approval,
        audit_level,
    }
}

pub const TOOL_CATALOG: &[ToolCatalogEntry] = &[
    tool_entry(TOOL_FILE_READ, "read", "low", ALL_AGENTS, false, "standard"),
    tool_entry(TOOL_FILE_WRITE, "write", "high", ALL_AGENTS, true, "full"),
    tool_entry(TOOL_BASH, "shell", "critical", ALL_AGENTS, true, "full"),
    tool_entry(TOOL_GLOB, "read", "low", ALL_AGENTS, false, "standard"),
    tool_entry(TOOL_GREP, "read", "low", ALL_AGENTS, false, "standard"),
    tool_entry(TOOL_EDIT, "write", "high", ALL_AGENTS, true, "full"),
    tool_entry(
        TOOL_NOTEBOOK_EDIT,
        "write",
        "high",
        ALL_AGENTS,
        true,
        "full",
    ),
    tool_entry(TOOL_TODO_WRITE, "ui", "low", ALL_AGENTS, false, "standard"),
    tool_entry(
        TOOL_WEB_FETCH,
        "network",
        "medium",
        ALL_AGENTS,
        true,
        "full",
    ),
    tool_entry(
        TOOL_WEB_SEARCH,
        "network",
        "medium",
        ALL_AGENTS,
        true,
        "standard",
    ),
    tool_entry(TOOL_MCP, "mcp", "high", ALL_AGENTS, true, "full"),
    tool_entry(
        TOOL_LIST_MCP_RESOURCES,
        "mcp",
        "medium",
        ALL_AGENTS,
        false,
        "standard",
    ),
    tool_entry(
        TOOL_READ_MCP_RESOURCE,
        "mcp",
        "medium",
        ALL_AGENTS,
        false,
        "standard",
    ),
    tool_entry(TOOL_MCP_AUTH, "mcp", "high", ALL_AGENTS, true, "full"),
    tool_entry(TOOL_LSP, "read", "medium", ALL_AGENTS, true, "standard"),
    tool_entry(
        TOOL_AGENT,
        "orchestration",
        "high",
        ALL_AGENTS,
        true,
        "full",
    ),
    tool_entry(
        TOOL_SKILL,
        "orchestration",
        "high",
        ALL_AGENTS,
        true,
        "full",
    ),
    tool_entry(
        TOOL_SEND_MESSAGE,
        "ui",
        "medium",
        ALL_AGENTS,
        true,
        "standard",
    ),
    tool_entry(
        TOOL_LEGACY_TASK_AGENT,
        "orchestration",
        "high",
        ALL_AGENTS,
        true,
        "full",
    ),
    tool_entry(
        TOOL_TASK_CREATE,
        "orchestration",
        "medium",
        ORCHESTRATION_AGENTS,
        true,
        "standard",
    ),
    tool_entry(
        TOOL_TASK_UPDATE,
        "orchestration",
        "medium",
        ORCHESTRATION_AGENTS,
        true,
        "standard",
    ),
    tool_entry(
        TOOL_TASK_LIST,
        "orchestration",
        "low",
        ORCHESTRATION_AGENTS,
        false,
        "standard",
    ),
    tool_entry(
        TOOL_TASK_GET,
        "orchestration",
        "low",
        ORCHESTRATION_AGENTS,
        false,
        "standard",
    ),
    tool_entry(
        TOOL_TASK_STOP,
        "orchestration",
        "high",
        ORCHESTRATION_AGENTS,
        true,
        "full",
    ),
    tool_entry(
        TOOL_TASK_OUTPUT,
        "orchestration",
        "low",
        ORCHESTRATION_AGENTS,
        false,
        "standard",
    ),
    tool_entry(
        TOOL_TEAM_CREATE,
        "orchestration",
        "high",
        ORCHESTRATION_AGENTS,
        true,
        "full",
    ),
    tool_entry(
        TOOL_TEAM_DELETE,
        "orchestration",
        "high",
        ORCHESTRATION_AGENTS,
        true,
        "full",
    ),
    tool_entry(
        TOOL_CRON_CREATE,
        "orchestration",
        "high",
        ORCHESTRATION_AGENTS,
        true,
        "full",
    ),
    tool_entry(
        TOOL_CRON_DELETE,
        "orchestration",
        "high",
        ORCHESTRATION_AGENTS,
        true,
        "full",
    ),
    tool_entry(
        TOOL_CRON_LIST,
        "orchestration",
        "low",
        ORCHESTRATION_AGENTS,
        false,
        "standard",
    ),
    tool_entry(
        TOOL_REMOTE_TRIGGER,
        "orchestration",
        "high",
        ORCHESTRATION_AGENTS,
        true,
        "full",
    ),
    tool_entry(
        TOOL_ENTER_PLAN,
        "mode",
        "medium",
        ALL_AGENTS,
        false,
        "standard",
    ),
    tool_entry(
        TOOL_EXIT_PLAN,
        "mode",
        "medium",
        ALL_AGENTS,
        false,
        "standard",
    ),
    tool_entry(
        TOOL_ENTER_WORKTREE,
        "workspace",
        "high",
        ORCHESTRATION_AGENTS,
        true,
        "full",
    ),
    tool_entry(
        TOOL_EXIT_WORKTREE,
        "workspace",
        "high",
        ORCHESTRATION_AGENTS,
        true,
        "full",
    ),
    tool_entry(
        TOOL_TOOL_SEARCH,
        "read",
        "low",
        EXPLORE_PLAN_AGENTS,
        false,
        "standard",
    ),
    tool_entry(TOOL_SLEEP, "control", "low", ALL_AGENTS, false, "minimal"),
    tool_entry(
        TOOL_STRUCTURED_OUTPUT,
        "control",
        "low",
        ALL_AGENTS,
        false,
        "minimal",
    ),
    tool_entry(
        TOOL_POWERSHELL,
        "shell",
        "critical",
        ALL_AGENTS,
        true,
        "full",
    ),
    tool_entry(TOOL_CONFIG, "write", "high", ALL_AGENTS, true, "full"),
    tool_entry(
        TOOL_SEND_USER_MESSAGE,
        "ui",
        "low",
        ALL_AGENTS,
        false,
        "minimal",
    ),
    tool_entry(
        TOOL_SEND_WECHAT_MESSAGE,
        "channel",
        "high",
        ALL_AGENTS,
        true,
        "full",
    ),
    tool_entry(
        TOOL_QUERY_WECHAT_HISTORY,
        "read",
        "critical",
        ALL_AGENTS,
        true,
        "full",
    ),
    tool_entry(TOOL_BRIEF, "ui", "low", ALL_AGENTS, false, "minimal"),
    tool_entry(
        TOOL_ASK_USER_QUESTION,
        "ui",
        "medium",
        ALL_AGENTS,
        true,
        "standard",
    ),
    tool_entry(TOOL_REPL, "shell", "critical", ALL_AGENTS, true, "full"),
    tool_entry(
        TOOL_KNOWLEDGE_SEARCH,
        "read",
        "low",
        ALL_AGENTS,
        false,
        "standard",
    ),
];

pub fn tool_catalog() -> &'static [ToolCatalogEntry] {
    TOOL_CATALOG
}

pub fn tool_catalog_entry(id: &str) -> Option<&'static ToolCatalogEntry> {
    TOOL_CATALOG.iter().find(|entry| entry.id == id)
}
