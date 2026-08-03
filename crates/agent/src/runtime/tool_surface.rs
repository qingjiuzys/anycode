//! Which tool names and schemas the LLM sees — shared by `execute_task` and `execute_turn_from_messages`.

use super::AgentClaudeToolGating;
use anycode_core::prelude::*;
use anycode_tools::DEFAULT_TOOL_IDS;
use regex::Regex;
use std::collections::{HashMap, HashSet};

const WEAK_LOCAL_CORE_TOOLS: &[&str] = &[
    "FileRead",
    "FileWrite",
    "Edit",
    "Glob",
    "Grep",
    "Bash",
    "ToolSearch",
    "AskUserQuestion",
    "SkillSearch",
    "Skill",
];

/// 长会话工具收敛时每轮始终注入的高频核心工具（覆盖绝大多数 agentic 编码/检索场景）。
/// 非核心工具按「会话中已使用过」动态保留；模型可通过 `ToolSearch` 发现并解锁其余工具。
const ALWAYS_INJECT_CORE_TOOLS: &[&str] = &[
    "FileRead",
    "FileWrite",
    "Edit",
    "Glob",
    "Grep",
    "Bash",
    "ToolSearch",
    "AskUserQuestion",
    "SkillSearch",
    "Skill",
    "TodoWrite",
    "PlanWrite",
    "WebFetch",
    "WebSearch",
    "KnowledgeSearch",
    "NotebookEdit",
    "PowerShell",
    "TaskCreate",
    "TaskUpdate",
    "TaskList",
    "TaskGet",
    "TaskOutput",
    "Agent",
];

/// Sort tool names: builtins (sorted), then other non-MCP (sorted), then `mcp__*` (sorted).
pub(crate) fn order_tool_names_like_assemble_tool_pool(names: Vec<ToolName>) -> Vec<ToolName> {
    let builtin: HashSet<&str> = DEFAULT_TOOL_IDS.iter().copied().collect();
    let mut bi = Vec::new();
    let mut mcp = Vec::new();
    let mut rest = Vec::new();
    for n in names {
        if n.starts_with("mcp__") {
            mcp.push(n);
        } else if builtin.contains(n.as_str()) {
            bi.push(n);
        } else {
            rest.push(n);
        }
    }
    bi.sort();
    mcp.sort();
    rest.sort();
    bi.into_iter().chain(rest).chain(mcp).collect()
}

/// Resolve the raw tool name list: empty `agent_tools` → all registry keys; `general-purpose` also merges `mcp__*` from registry.
pub(crate) fn resolve_agent_tool_names(
    agent_type: &str,
    mut agent_tools: Vec<ToolName>,
    registry: &HashMap<ToolName, Box<dyn Tool>>,
) -> Vec<ToolName> {
    if agent_tools.is_empty() {
        let mut ks: Vec<_> = registry.keys().cloned().collect();
        ks.sort();
        ks
    } else if agent_type == "general-purpose" {
        for k in registry.keys() {
            if k.starts_with("mcp__") && !agent_tools.contains(k) {
                agent_tools.push(k.clone());
            }
        }
        agent_tools.sort();
        agent_tools
    } else {
        agent_tools
    }
}

fn mcp_tool_visible_to_llm(name: &str, gating: &AgentClaudeToolGating) -> bool {
    let Some(g) = &gating.mcp_defer_allowlist else {
        return true;
    };
    g.lock().map(|set| set.contains(name)).unwrap_or(false)
}

/// Apply deny regexes, Claude blanket deny, MCP defer allowlist, per-task deny lists, then stable ordering.
pub(crate) fn prepare_tool_names_for_llm(
    names: Vec<ToolName>,
    tool_name_deny: &[Regex],
    gating: &AgentClaudeToolGating,
    extra_deny_names: &[String],
    extra_deny_prefixes: &[String],
) -> Vec<ToolName> {
    let names: Vec<_> = names
        .into_iter()
        .filter(|n| {
            if extra_deny_names.iter().any(|d| d == n) {
                return false;
            }
            if extra_deny_prefixes
                .iter()
                .any(|prefix| !prefix.is_empty() && n.starts_with(prefix))
            {
                return false;
            }
            if tool_name_deny.iter().any(|re| re.is_match(n)) {
                return false;
            }
            if gating
                .rules
                .as_ref()
                .is_some_and(|r| r.blanket_denies_tool(n))
            {
                return false;
            }
            if n.starts_with("mcp__")
                && gating.defer_mcp_tools
                && !mcp_tool_visible_to_llm(n, gating)
            {
                return false;
            }
            true
        })
        .collect();
    order_tool_names_like_assemble_tool_pool(names)
}

pub(crate) fn build_tool_schemas(
    names: &[ToolName],
    registry: &HashMap<ToolName, Box<dyn Tool>>,
) -> Vec<ToolSchema> {
    names
        .iter()
        .filter_map(|name| {
            registry.get(name).map(|tool| ToolSchema {
                name: tool.name().to_string(),
                description: tool.api_tool_description(),
                input_schema: tool.schema(),
            })
        })
        .collect()
}

/// 每轮注入的工具 schema：
/// - turn 1：weak 本地模型只给核心工具（[`WEAK_LOCAL_CORE_TOOLS`]），其余全量；
/// - turn ≥ 2：收敛为「核心工具 ∪ 会话已用工具」，控制长会话每轮请求体积。
///   `ToolSearch` 是逃生舱：模型可发现并解锁核心集之外的任何工具。
pub(crate) fn schemas_for_model_turn(
    all: &[ToolSchema],
    model: &ModelConfig,
    turn: usize,
    used_tools: &HashSet<String>,
) -> Vec<ToolSchema> {
    let weak = anycode_llm::capabilities_for_model_config(model).weak_local_model;
    if turn == 1 {
        if weak {
            return all
                .iter()
                .filter(|schema| WEAK_LOCAL_CORE_TOOLS.contains(&schema.name.as_str()))
                .cloned()
                .collect();
        }
        return all.to_vec();
    }
    if !weak {
        // 云端模型上下文充裕：turn≥2 保持全量工具，避免 Browser/mcp/生成类工具被隐藏后
        // 被迫额外 ToolSearch 解锁——那是「改页面」这类任务多出步骤的常见来源。
        return all.to_vec();
    }
    all.iter()
        .filter(|schema| {
            ALWAYS_INJECT_CORE_TOOLS.contains(&schema.name.as_str())
                || used_tools.contains(&schema.name)
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct StubTool(&'static str);

    #[async_trait]
    impl Tool for StubTool {
        fn name(&self) -> &str {
            self.0
        }

        fn description(&self) -> &str {
            "stub"
        }

        fn schema(&self) -> serde_json::Value {
            serde_json::json!({})
        }

        fn permission_mode(&self) -> PermissionMode {
            PermissionMode::Default
        }

        fn security_policy(&self) -> Option<&SecurityPolicy> {
            None
        }

        async fn execute(&self, _input: ToolInput) -> Result<ToolOutput, CoreError> {
            Ok(ToolOutput {
                result: serde_json::json!({}),
                error: None,
                duration_ms: 0,
            })
        }
    }

    fn reg_with(keys: &[&'static str]) -> HashMap<ToolName, Box<dyn Tool>> {
        let mut m = HashMap::new();
        for k in keys {
            m.insert((*k).to_string(), Box::new(StubTool(k)) as Box<dyn Tool>);
        }
        m
    }

    #[test]
    fn order_puts_mcp_last() {
        let names = vec![
            "mcp__x__t".to_string(),
            "Glob".to_string(),
            "mcp__a__t".to_string(),
            "Grep".to_string(),
        ];
        let out = order_tool_names_like_assemble_tool_pool(names);
        assert!(out.iter().filter(|n| n.starts_with("mcp__")).count() == 2);
        assert_eq!(out[out.len() - 2..], ["mcp__a__t", "mcp__x__t"]);
    }

    #[test]
    fn resolve_empty_agent_tools_is_all_registry_keys_sorted() {
        let reg = reg_with(&["Zebra", "Alpha", "mcp__s__z"]);
        let out = resolve_agent_tool_names("explore", vec![], &reg);
        assert_eq!(out, vec!["Alpha", "Zebra", "mcp__s__z"]);
    }

    #[test]
    fn resolve_general_purpose_merges_mcp_from_registry() {
        let reg = reg_with(&["FileRead", "mcp__srv__tool"]);
        let agent_list = vec!["FileRead".to_string()];
        let out = resolve_agent_tool_names("general-purpose", agent_list, &reg);
        assert_eq!(out, vec!["FileRead", "mcp__srv__tool"]);
    }

    #[test]
    fn resolve_explore_does_not_merge_mcp() {
        let reg = reg_with(&["FileRead", "mcp__srv__tool"]);
        let agent_list = vec!["FileRead".to_string()];
        let out = resolve_agent_tool_names("explore", agent_list, &reg);
        assert_eq!(out, vec!["FileRead".to_string()]);
    }

    #[test]
    fn prepare_drops_regex_deny() {
        let re = Regex::new("^mcp__").unwrap();
        let names = vec!["Glob".to_string(), "mcp__a".to_string()];
        let gating = AgentClaudeToolGating::default();
        let out = prepare_tool_names_for_llm(names, std::slice::from_ref(&re), &gating, &[], &[]);
        assert_eq!(out, vec!["Glob".to_string()]);
    }

    #[test]
    fn prepare_applies_cron_observability_extra_deny() {
        let reg = reg_with(&["FileRead", "Bash", "TaskList", "CronList", "mcp__srv__tool"]);
        let names = resolve_agent_tool_names("general-purpose", vec![], &reg);
        let (deny_names, deny_prefixes) =
            anycode_tools::cron_tool_profile_filters(Some("observability"), None);
        let gating = AgentClaudeToolGating::default();
        let out = prepare_tool_names_for_llm(names, &[], &gating, &deny_names, &deny_prefixes);
        assert!(out.contains(&"FileRead".to_string()));
        assert!(out.contains(&"TaskList".to_string()));
        assert!(out.contains(&"CronList".to_string()));
        assert!(!out.iter().any(|n| n == "Bash"));
        assert!(!out.iter().any(|n| n.starts_with("mcp__")));
    }

    #[test]
    fn execute_task_and_turn_paths_share_extra_deny_lists() {
        let reg = reg_with(&["FileRead", "Bash", "Glob", "mcp__srv__tool"]);
        let raw = resolve_agent_tool_names("general-purpose", vec![], &reg);
        let gating = AgentClaudeToolGating::default();
        let deny_names = vec!["Bash".to_string()];
        let deny_prefixes = vec!["mcp__".to_string()];
        let task_path =
            prepare_tool_names_for_llm(raw.clone(), &[], &gating, &deny_names, &deny_prefixes);
        let turn_path = prepare_tool_names_for_llm(raw, &[], &gating, &deny_names, &deny_prefixes);
        assert_eq!(task_path, turn_path);
    }

    #[test]
    fn prepare_applies_cron_read_only_extra_deny() {
        let reg = reg_with(&["FileRead", "Bash", "Glob", "mcp__srv__tool"]);
        let names = resolve_agent_tool_names("general-purpose", vec![], &reg);
        let (deny_names, deny_prefixes) =
            anycode_tools::cron_tool_profile_filters(Some("read_only"), None);
        let gating = AgentClaudeToolGating::default();
        let out = prepare_tool_names_for_llm(names, &[], &gating, &deny_names, &deny_prefixes);
        assert!(out.contains(&"FileRead".to_string()));
        assert!(out.contains(&"Glob".to_string()));
        assert!(!out.iter().any(|n| n == "Bash"));
        assert!(!out.iter().any(|n| n.starts_with("mcp__")));
    }

    #[test]
    fn prepare_explore_plan_denies_browser_screenshot() {
        let reg = reg_with(&["FileRead", "BrowserScreenshot", "BrowserSnapshot"]);
        let names = resolve_agent_tool_names("explore", vec!["FileRead".into()], &reg);
        let merged = anycode_tools::merge_agent_type_tool_denies("explore", &[]);
        let gating = AgentClaudeToolGating::default();
        let out = prepare_tool_names_for_llm(names, &[], &gating, &merged, &[]);
        assert!(out.contains(&"FileRead".to_string()));
        assert!(!out.iter().any(|n| n == "BrowserScreenshot"));
    }

    #[test]
    fn build_schemas_preserves_name_order() {
        let reg = reg_with(&["A", "B"]);
        let names = vec!["B".to_string(), "A".to_string()];
        let schemas = build_tool_schemas(&names, &reg);
        assert_eq!(schemas.len(), 2);
        assert_eq!(schemas[0].name, "B");
        assert_eq!(schemas[1].name, "A");
    }

    #[test]
    fn weak_local_first_turn_schema_is_core_only_and_shrinks_materially() {
        let reg = reg_with(&[
            "FileRead",
            "FileWrite",
            "Edit",
            "Glob",
            "Grep",
            "Bash",
            "ToolSearch",
            "AskUserQuestion",
            "WebFetch",
            "WebSearch",
            "Agent",
            "Skill",
            "TaskCreate",
            "TaskList",
            "BrowserNavigate",
            "BrowserSnapshot",
        ]);
        let all = build_tool_schemas(&reg.keys().cloned().collect::<Vec<_>>(), &reg);
        let model = ModelConfig {
            provider: LLMProvider::OpenAI,
            model: "minicpm5-1b".into(),
            base_url: Some("http://127.0.0.1:47100/v1/chat/completions".into()),
            ..Default::default()
        };
        let used = HashSet::new();
        let compact = schemas_for_model_turn(&all, &model, 1, &used);
        assert!(compact
            .iter()
            .all(|schema| { WEAK_LOCAL_CORE_TOOLS.contains(&schema.name.as_str()) }));
        assert!(
            compact.len() * 10 <= all.len() * 6,
            "expected >=40% reduction"
        );
        // turn 2+ 收敛：核心 ∪ 已用（这里没有任何已用工具，仅核心集注入）
        let mut used2 = HashSet::new();
        let conv = schemas_for_model_turn(&all, &model, 2, &used2);
        assert!(conv
            .iter()
            .all(|sc| ALWAYS_INJECT_CORE_TOOLS.contains(&sc.name.as_str())));
        assert!(conv.len() < all.len());
        // 已用工具保持可用
        used2.insert("BrowserNavigate".to_string());
        used2.insert("BrowserSnapshot".to_string());
        let conv2 = schemas_for_model_turn(&all, &model, 3, &used2);
        assert!(conv2.iter().any(|sc| sc.name == "BrowserNavigate"));
        assert!(conv2.iter().any(|sc| sc.name == "BrowserSnapshot"));
    }

    #[test]
    fn cloud_model_turn_ge_2_keeps_full_tool_set() {
        let reg = reg_with(&[
            "FileRead",
            "FileWrite",
            "Edit",
            "Glob",
            "Grep",
            "Bash",
            "ToolSearch",
            "AskUserQuestion",
            "WebFetch",
            "WebSearch",
            "Agent",
            "Skill",
            "TaskCreate",
            "TaskList",
            "BrowserNavigate",
            "BrowserSnapshot",
            "GenerateImage",
            "mcp__foo__bar",
        ]);
        let all = build_tool_schemas(&reg.keys().cloned().collect::<Vec<_>>(), &reg);
        let model = ModelConfig {
            provider: LLMProvider::Anthropic,
            model: "claude-sonnet-4-5".into(),
            base_url: None,
            ..Default::default()
        };
        // 云端模型：turn 1 与 turn≥2 都应保持全量工具（含 Browser/mcp/生成类），
        // 避免非核心工具被隐藏后被迫额外 ToolSearch 解锁。
        for turn in [1usize, 2, 3, 5] {
            let out = schemas_for_model_turn(&all, &model, turn, &HashSet::new());
            assert_eq!(out.len(), all.len(), "turn={turn} 云端应保持全量工具");
        }
        // 核心工具仍可用
        let out = schemas_for_model_turn(&all, &model, 4, &HashSet::new());
        assert!(out.iter().any(|sc| sc.name == "BrowserSnapshot"));
        assert!(out.iter().any(|sc| sc.name == "mcp__foo__bar"));
    }
}
