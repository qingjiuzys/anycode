//! 从 `ToolRegistryDeps` 构建完整工具注册表。
//!
//! ## 新增默认工具时的 checklist
//!
//! 1. 在本文件 `ins!(…)` 中注册实例（与模型所见 API 名一致）。
//! 2. 在 [`catalog`](crate::catalog) 增加 `TOOL_*` 常量；若属于 general-purpose 全集，同步 [`DEFAULT_TOOL_IDS`](crate::catalog::DEFAULT_TOOL_IDS)。
//! 3. 运行 `cargo test -p anycode-tools`（含 `validate_default_registry`）；CI 对 workspace 执行 `cargo test`。
//! 4. 若工具敏感：加入 `crate::catalog::SECURITY_SENSITIVE_TOOL_IDS`（单一事实来源）；CLI `bootstrap` 自动为该表注册 `SecurityLayer` 策略。

use crate::agent_tools::{AgentTool, LegacyTaskAgentTool, SendMessageTool, SkillTool};
use crate::bash::BashTool;
use crate::edit::EditTool;
use crate::file_read::FileReadTool;
use crate::file_write::FileWriteTool;
use crate::glob::GlobTool;
use crate::grep::GrepTool;
use crate::lsp_tool::LspTool;
use crate::mcp_tools::{ListMcpResourcesTool, McpAuthTool, McpTool, ReadMcpResourceTool};
use crate::mode_tools::{
    EnterPlanModeTool, EnterWorktreeTool, ExitPlanModeTool, ExitWorktreeTool, SleepTool,
    StructuredOutputTool, ToolSearchTool,
};
use crate::notebook_edit::NotebookEditTool;
use crate::orchestration::{
    CronCreateTool, CronDeleteTool, CronListTool, RemoteTriggerTool, TaskCreateTool, TaskGetTool,
    TaskListTool, TaskOutputTool, TaskStopTool, TaskUpdateTool, TeamCreateTool, TeamDeleteTool,
};
use crate::platform_tools::{
    AskUserQuestionTool, BriefTool, ConfigTool, PowerShellTool, ReplTool, SendUserMessageTool,
};
use crate::services::ToolRegistryDeps;
use crate::todo_write::TodoWriteTool;
use crate::web_fetch::WebFetchTool;
use crate::web_search::WebSearchTool;
use anycode_core::prelude::*;
use std::collections::HashMap;

/// 构建与 Claude Code 工具名对齐的完整注册表。
pub fn build_registry(deps: &ToolRegistryDeps) -> HashMap<ToolName, Box<dyn Tool>> {
    let sm = deps.sandbox_mode;
    let s = &deps.services;
    let mut tools: HashMap<ToolName, Box<dyn Tool>> = HashMap::new();

    macro_rules! ins {
        ($t:expr) => {
            let b: Box<dyn Tool> = Box::new($t);
            tools.insert(b.name().to_string(), b);
        };
    }

    ins!(FileReadTool::new(sm));
    ins!(FileWriteTool::new(sm));
    ins!(BashTool::new(sm));
    ins!(GlobTool::new(sm));
    ins!(GrepTool::new(sm));
    ins!(EditTool::new(sm));
    ins!(NotebookEditTool::new(sm));
    ins!(TodoWriteTool::new(s.clone()));
    ins!(WebFetchTool::new(s.clone()));
    ins!(WebSearchTool::new(s.clone()));
    ins!(McpTool::new(s.clone()));
    ins!(ListMcpResourcesTool::new(s.clone()));
    ins!(ReadMcpResourceTool::new(s.clone()));
    ins!(McpAuthTool::new(s.clone()));
    ins!(LspTool::new(s.clone()));
    ins!(AgentTool::new(s.clone()));
    ins!(SkillTool::new(s.clone()));
    ins!(SendMessageTool::new(s.clone()));
    ins!(LegacyTaskAgentTool::new(s.clone()));
    ins!(TaskCreateTool::new(s.clone()));
    ins!(TaskUpdateTool::new(s.clone()));
    ins!(TaskListTool::new(s.clone()));
    ins!(TaskGetTool::new(s.clone()));
    ins!(TaskStopTool::new(s.clone()));
    ins!(TaskOutputTool::new(s.clone()));
    ins!(TeamCreateTool::new(s.clone()));
    ins!(TeamDeleteTool::new(s.clone()));
    ins!(CronCreateTool::new(s.clone()));
    ins!(CronDeleteTool::new(s.clone()));
    ins!(CronListTool::new(s.clone()));
    ins!(RemoteTriggerTool::new(s.clone()));
    ins!(EnterPlanModeTool::new(s.clone()));
    ins!(ExitPlanModeTool::new(s.clone()));
    ins!(EnterWorktreeTool::new(s.clone()));
    ins!(ExitWorktreeTool::new(s.clone()));
    ins!(ToolSearchTool::new(s.clone()));
    ins!(SleepTool);
    ins!(StructuredOutputTool);
    ins!(PowerShellTool::new(sm));
    ins!(ConfigTool::new(s.clone()));
    ins!(SendUserMessageTool::new());
    ins!(BriefTool::new());
    ins!(AskUserQuestionTool::new(s.clone()));
    ins!(ReplTool::new());

    #[cfg(feature = "tools-mcp")]
    {
        use crate::mcp_normalization::normalize_name_for_mcp;
        use crate::mcp_proxied_tool::McpProxiedTool;
        for mcp in s.mcp_sessions() {
            let slug = mcp.server_slug().to_string();
            let slug = if slug.is_empty() {
                "default".to_string()
            } else {
                slug
            };
            for meta in mcp.listed_tools() {
                let tn = normalize_name_for_mcp(&meta.name);
                if tn.is_empty() {
                    continue;
                }
                let mut logical = format!("mcp__{}__{}", slug, tn);
                let mut n = 0u32;
                while tools.contains_key(&logical) {
                    n += 1;
                    logical = format!("mcp__{}__{}_{}", slug, tn, n);
                }
                let b: Box<dyn Tool> = Box::new(McpProxiedTool::new(
                    mcp.clone(),
                    logical.clone(),
                    meta.name.clone(),
                    meta.description.clone(),
                    meta.input_schema.clone(),
                ));
                tools.insert(logical, b);
            }
            // Claude Code 风格：每服务器一个 `mcp__<slug>__authenticate`（若 tools/list 未声明同名则补注册）
            let auth_logical = format!("mcp__{}__authenticate", slug);
            if !tools.contains_key(&auth_logical) {
                let b: Box<dyn Tool> = Box::new(McpProxiedTool::new(
                    mcp.clone(),
                    auth_logical.clone(),
                    "authenticate".to_string(),
                    "MCP OAuth or session authentication for this server (if supported)."
                        .to_string(),
                    serde_json::json!({
                        "type": "object",
                        "additionalProperties": true
                    }),
                ));
                tools.insert(auth_logical, b);
            }
        }
    }

    tools
}
