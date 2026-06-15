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
pub const TOOL_PLAN_WRITE: &str = "PlanWrite";
pub const TOOL_WEB_FETCH: &str = "WebFetch";
pub const TOOL_WEB_SEARCH: &str = "WebSearch";
pub const TOOL_KNOWLEDGE_SEARCH: &str = "KnowledgeSearch";
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
pub const TOOL_SPEECH_TO_TEXT: &str = "SpeechToText";
pub const TOOL_TEXT_TO_SPEECH: &str = "TextToSpeech";
pub const TOOL_GENERATE_IMAGE: &str = "GenerateImage";
pub const TOOL_GENERATE_VIDEO: &str = "GenerateVideo";

/// general-purpose Agent 暴露的完整工具 id（与 `build_registry` 插入集合一致）。
pub use anycode_core::{ToolCatalogEntry, DEFAULT_TOOL_IDS, SECURITY_SENSITIVE_TOOL_IDS};

/// Shared catalog metadata (SSOT in `anycode_core::tool_catalog`).
pub use anycode_core::{tool_catalog, tool_catalog_entry};

pub const EXPLORE_PLAN_TOOL_IDS: [&str; 4] = [TOOL_FILE_READ, TOOL_GLOB, TOOL_GREP, TOOL_BASH];

/// Tools denied when a cron job uses the `read_only` profile.
pub const CRON_READ_ONLY_DENIED_TOOL_IDS: &[&str] = &[
    TOOL_FILE_WRITE,
    TOOL_BASH,
    TOOL_EDIT,
    TOOL_NOTEBOOK_EDIT,
    TOOL_TODO_WRITE,
    TOOL_PLAN_WRITE,
    TOOL_POWERSHELL,
    TOOL_AGENT,
    TOOL_LEGACY_TASK_AGENT,
    TOOL_TASK_CREATE,
    TOOL_TASK_UPDATE,
    TOOL_TASK_STOP,
    TOOL_TEAM_CREATE,
    TOOL_TEAM_DELETE,
    TOOL_CRON_CREATE,
    TOOL_CRON_DELETE,
    TOOL_REMOTE_TRIGGER,
    TOOL_ENTER_PLAN,
    TOOL_EXIT_PLAN,
    TOOL_ENTER_WORKTREE,
    TOOL_EXIT_WORKTREE,
    TOOL_REPL,
    TOOL_CONFIG,
    TOOL_MCP,
    TOOL_MCP_AUTH,
    TOOL_SEND_MESSAGE,
    TOOL_LSP,
];

/// Tools allowed when a cron job uses the `observability` profile (monitoring-only).
pub const CRON_OBSERVABILITY_ALLOWED_TOOL_IDS: &[&str] = &[
    TOOL_FILE_READ,
    TOOL_GLOB,
    TOOL_GREP,
    TOOL_WEB_FETCH,
    TOOL_WEB_SEARCH,
    TOOL_TASK_LIST,
    TOOL_TASK_GET,
    TOOL_TASK_OUTPUT,
    TOOL_CRON_LIST,
];

/// Known persisted cron tool profiles (`CronJob.tool_profile` / `CronCreate`).
pub fn known_cron_tool_profiles() -> &'static [&'static str] {
    &["default", "read_only", "observability", "allowlist"]
}

pub fn is_known_cron_tool_profile(profile: &str) -> bool {
    known_cron_tool_profiles().contains(&profile.trim())
}

/// Known failure routing targets for cron jobs.
pub fn known_cron_failure_destinations() -> &'static [&'static str] {
    &["log", "same_channel", "shell", "http"]
}

pub fn is_known_cron_failure_destination(dest: &str) -> bool {
    known_cron_failure_destinations().contains(&dest.trim())
}

fn cron_allowlist_filters(allowlist: Option<&[String]>) -> (Vec<String>, Vec<String>) {
    let allow: Vec<&str> = allowlist
        .unwrap_or(&[])
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if allow.is_empty() {
        return (
            DEFAULT_TOOL_IDS.iter().map(|s| (*s).to_string()).collect(),
            vec!["mcp__".to_string()],
        );
    }
    let deny: Vec<String> = DEFAULT_TOOL_IDS
        .iter()
        .filter(|id| !allow.contains(id))
        .map(|s| (*s).to_string())
        .collect();
    let prefixes = if allow.iter().any(|a| a.starts_with("mcp__")) {
        vec![]
    } else {
        vec!["mcp__".to_string()]
    };
    (deny, prefixes)
}

/// Resolve per-cron tool deny lists from `CronJob.tool_profile`.
pub fn cron_tool_profile_filters(
    profile: Option<&str>,
    allowlist: Option<&[String]>,
) -> (Vec<String>, Vec<String>) {
    match profile.map(str::trim).filter(|s| !s.is_empty()) {
        None | Some("default") => (vec![], vec![]),
        Some("read_only") => (
            CRON_READ_ONLY_DENIED_TOOL_IDS
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
            vec!["mcp__".to_string()],
        ),
        Some("observability") => {
            let deny: Vec<String> = DEFAULT_TOOL_IDS
                .iter()
                .filter(|id| !CRON_OBSERVABILITY_ALLOWED_TOOL_IDS.contains(id))
                .map(|s| (*s).to_string())
                .collect();
            (deny, vec!["mcp__".to_string()])
        }
        Some("allowlist") => cron_allowlist_filters(allowlist),
        Some(other) => {
            tracing::warn!(
                target: "anycode_tools",
                profile = other,
                "unknown cron tool_profile; using default tool surface"
            );
            (vec![], vec![])
        }
    }
}

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

/// Plan agent tools: explore/read/bash plus hierarchical `PlanWrite`.
pub fn plan_tool_names_with_skill(include_skill: bool) -> Vec<ToolName> {
    let mut v = explore_plan_tool_names_with_skill(include_skill);
    v.push(TOOL_PLAN_WRITE.to_string());
    v.sort();
    v.dedup();
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
        TOOL_CRON_CREATE.to_string(),
        TOOL_CRON_DELETE.to_string(),
        TOOL_CRON_LIST.to_string(),
        TOOL_TOOL_SEARCH.to_string(),
        TOOL_STRUCTURED_OUTPUT.to_string(),
        TOOL_SEND_USER_MESSAGE.to_string(),
        TOOL_BRIEF.to_string(),
        TOOL_ASK_USER_QUESTION.to_string(),
        TOOL_GENERATE_IMAGE.to_string(),
        TOOL_GENERATE_VIDEO.to_string(),
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

#[cfg(test)]
mod workspace_assistant_tools_tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn workspace_assistant_exposes_cron_create_delete_list() {
        let tools = workspace_assistant_tool_names(false);
        assert!(tools.contains(&TOOL_CRON_CREATE.to_string()));
        assert!(tools.contains(&TOOL_CRON_DELETE.to_string()));
        assert!(tools.contains(&TOOL_CRON_LIST.to_string()));
    }

    #[test]
    fn tool_catalog_covers_default_tools() {
        let catalog: HashSet<&str> = tool_catalog().iter().map(|entry| entry.id).collect();
        for id in DEFAULT_TOOL_IDS {
            assert!(
                catalog.contains(id),
                "missing tool catalog metadata for {id}"
            );
        }
    }

    #[test]
    fn local_tool_constants_match_core_default_ids() {
        let mut local = vec![
            TOOL_FILE_READ,
            TOOL_FILE_WRITE,
            TOOL_BASH,
            TOOL_GLOB,
            TOOL_GREP,
            TOOL_EDIT,
            TOOL_KNOWLEDGE_SEARCH,
            TOOL_NOTEBOOK_EDIT,
            TOOL_TODO_WRITE,
            TOOL_PLAN_WRITE,
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
            TOOL_SPEECH_TO_TEXT,
            TOOL_TEXT_TO_SPEECH,
            TOOL_GENERATE_IMAGE,
            TOOL_GENERATE_VIDEO,
        ];
        let mut core = DEFAULT_TOOL_IDS.to_vec();
        local.sort_unstable();
        core.sort_unstable();
        assert_eq!(local, core);
    }

    #[test]
    fn default_tools_have_governance_metadata() {
        for id in DEFAULT_TOOL_IDS {
            let entry = tool_catalog_entry(id).unwrap_or_else(|| panic!("catalog entry for {id}"));
            assert!(!entry.risk_tier.trim().is_empty(), "risk_tier for {id}");
            assert!(!entry.category.trim().is_empty(), "category for {id}");
        }
    }

    #[test]
    fn sensitive_tools_require_approval_metadata() {
        for id in SECURITY_SENSITIVE_TOOL_IDS {
            let entry = tool_catalog_entry(id).expect("sensitive tool must be cataloged");
            assert!(
                entry.requires_approval,
                "sensitive tool {id} must require approval"
            );
        }
    }
}

pub fn validate_default_registry(tools: &HashMap<ToolName, Box<dyn Tool>>) -> anyhow::Result<()> {
    for id in DEFAULT_TOOL_IDS {
        if !tools.contains_key(*id) {
            anyhow::bail!("default tool registry missing tool {:?}", id);
        }
        let entry = tool_catalog_entry(id).ok_or_else(|| {
            anyhow::anyhow!("default tool {id:?} missing governance catalog entry")
        })?;
        if entry.risk_tier.trim().is_empty() {
            anyhow::bail!("default tool {id:?} missing risk_tier metadata");
        }
        if entry.category.trim().is_empty() {
            anyhow::bail!("default tool {id:?} missing category metadata");
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
        (TOOL_PLAN_WRITE, "Hierarchical session plan tree"),
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
        (TOOL_SEND_USER_MESSAGE, "In-session user-visible message (not WeChat)"),
        (
            TOOL_SEND_WECHAT_MESSAGE,
            "Send text to bound WeChat chat via iLink bridge",
        ),
        (
            TOOL_QUERY_WECHAT_HISTORY,
            "Query local WeChat chat history for a calendar day (read-only)",
        ),
        (TOOL_BRIEF, "Alias of SendUserMessage"),
        (
            TOOL_GENERATE_IMAGE,
            "Generate an image from a text prompt using models.image",
        ),
        (
            TOOL_GENERATE_VIDEO,
            "Generate a video from a text prompt using models.video",
        ),
        (TOOL_SPEECH_TO_TEXT, "Transcribe audio to text using models.speech.stt"),
        (TOOL_TEXT_TO_SPEECH, "Synthesize speech from text using models.speech.tts"),
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
    fn general_purpose_includes_media_tools() {
        let tools = general_purpose_tool_names();
        assert!(tools.iter().any(|t| t == "GenerateVideo"));
        assert!(tools.iter().any(|t| t == "GenerateImage"));
    }

    #[test]
    fn workspace_assistant_includes_video_tools() {
        let tools = workspace_assistant_tool_names(false);
        assert!(tools.iter().any(|t| t == "GenerateVideo"));
        assert!(tools.iter().any(|t| t == "GenerateImage"));
    }

    #[test]
    fn cron_allowlist_profile_denies_unlisted_tools() {
        let allow = vec!["FileRead".to_string(), "Glob".to_string()];
        let (names, prefixes) = cron_tool_profile_filters(Some("allowlist"), Some(&allow));
        assert!(!names.iter().any(|n| n == "FileRead"));
        assert!(!names.iter().any(|n| n == "Glob"));
        assert!(names.iter().any(|n| n == "Bash"));
        assert!(prefixes.iter().any(|p| p == "mcp__"));
    }

    #[test]
    fn cron_observability_profile_allows_task_list_only_subset() {
        let (names, prefixes) = cron_tool_profile_filters(Some("observability"), None);
        assert!(!names.iter().any(|n| n == "TaskList"));
        assert!(!names.iter().any(|n| n == "CronList"));
        assert!(names.iter().any(|n| n == "Bash"));
        assert!(prefixes.iter().any(|p| p == "mcp__"));
    }

    #[test]
    fn known_cron_profiles_include_observability() {
        assert!(is_known_cron_tool_profile("observability"));
        assert!(!is_known_cron_tool_profile("custom"));
    }

    #[test]
    fn cron_read_only_profile_denies_bash_and_mcp_prefix() {
        let (names, prefixes) = cron_tool_profile_filters(Some("read_only"), None);
        assert!(names.iter().any(|n| n == "Bash"));
        assert!(prefixes.iter().any(|p| p == "mcp__"));
    }

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
    fn plan_tools_include_plan_write() {
        let names = plan_tool_names_with_skill(false);
        assert!(names.iter().any(|n| n == "PlanWrite"));
        assert!(names.iter().any(|n| n == "FileRead"));
        let explore = explore_plan_tool_names_with_skill(false);
        assert!(!explore.iter().any(|n| n == "PlanWrite"));
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
