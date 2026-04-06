//! Which tool names and schemas the LLM sees — shared by `execute_task` and `execute_turn_from_messages`.

use super::AgentClaudeToolGating;
use anycode_core::prelude::*;
use anycode_tools::DEFAULT_TOOL_IDS;
use regex::Regex;
use std::collections::{HashMap, HashSet};

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

/// Apply deny regexes, Claude blanket deny, MCP defer allowlist, then stable ordering.
pub(crate) fn prepare_tool_names_for_llm(
    names: Vec<ToolName>,
    tool_name_deny: &[Regex],
    gating: &AgentClaudeToolGating,
) -> Vec<ToolName> {
    let names: Vec<_> = names
        .into_iter()
        .filter(|n| {
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
        let out = prepare_tool_names_for_llm(names, std::slice::from_ref(&re), &gating);
        assert_eq!(out, vec!["Glob".to_string()]);
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
}
