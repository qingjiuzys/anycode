//! Resolve declarative agent profile tool surfaces from `extends` + allow/deny.

use anycode_core::RuntimeMode;
use anycode_tools::{
    explore_plan_tool_names_with_skill, general_purpose_tool_names, workspace_assistant_tool_names,
};
use std::collections::HashSet;

pub const BUILTIN_EXTENDS: &[&str] = &[
    "general-purpose",
    "explore",
    "plan",
    "workspace-assistant",
    "goal",
];

/// Shipped role preset metadata (Composite catalog seed).
pub struct BuiltinAgentSeed {
    pub id: &'static str,
    pub extends: &'static str,
    pub description: &'static str,
}

pub const BUILTIN_AGENT_SEED: &[BuiltinAgentSeed] = &[
    BuiltinAgentSeed {
        id: "general-purpose",
        extends: "general-purpose",
        description: "Default implementation-focused coding agent",
    },
    BuiltinAgentSeed {
        id: "explore",
        extends: "explore",
        description: "Fast codebase exploration",
    },
    BuiltinAgentSeed {
        id: "plan",
        extends: "plan",
        description: "Architecture and task decomposition",
    },
    BuiltinAgentSeed {
        id: "workspace-assistant",
        extends: "workspace-assistant",
        description: "IM / cron channel operations",
    },
    BuiltinAgentSeed {
        id: "goal",
        extends: "goal",
        description: "Autonomous goal iteration",
    },
    BuiltinAgentSeed {
        id: "builder",
        extends: "general-purpose",
        description: "Default implementation-focused coding agent",
    },
    BuiltinAgentSeed {
        id: "planner",
        extends: "plan",
        description: "Architecture and task decomposition",
    },
    BuiltinAgentSeed {
        id: "explorer",
        extends: "explore",
        description: "Fast codebase exploration",
    },
    BuiltinAgentSeed {
        id: "verifier",
        extends: "explore",
        description: "Read-only verification and test inspection",
    },
    BuiltinAgentSeed {
        id: "reviewer",
        extends: "explore",
        description: "PR-style review without shell mutation",
    },
    BuiltinAgentSeed {
        id: "channel-ops",
        extends: "workspace-assistant",
        description: "IM / cron channel operations",
    },
    BuiltinAgentSeed {
        id: "goal-runner",
        extends: "goal",
        description: "Autonomous goal iteration",
    },
    BuiltinAgentSeed {
        id: "office-writer",
        extends: "general-purpose",
        description: "Office writing: reports, briefs, content drafts",
    },
    BuiltinAgentSeed {
        id: "data-analyst",
        extends: "general-purpose",
        description: "Spreadsheets, summaries, and data reports",
    },
    BuiltinAgentSeed {
        id: "researcher",
        extends: "explore",
        description: "Industry research and daily briefs",
    },
    BuiltinAgentSeed {
        id: "file-operator",
        extends: "workspace-assistant",
        description: "Batch file organization",
    },
];

#[must_use]
pub fn runtime_mode_for_extends(extends: &str) -> RuntimeMode {
    match extends.trim() {
        "plan" => RuntimeMode::Plan,
        "explore" => RuntimeMode::Explore,
        "workspace-assistant" | "channel" | "channel-ops" => RuntimeMode::Channel,
        "goal" | "goal-runner" => RuntimeMode::Goal,
        _ => RuntimeMode::Code,
    }
}

#[derive(Debug, Clone)]
pub struct AgentProfileSpec {
    pub extends: String,
    pub description: Option<String>,
    pub tools_allow: Option<Vec<String>>,
    pub tools_deny: Option<Vec<String>>,
    pub skills_allowlist: Option<Vec<String>>,
    pub prompt_overlay: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedAgentProfile {
    pub id: String,
    pub extends: String,
    pub description: String,
    pub tools: Vec<String>,
    pub skills_allowlist: Option<Vec<String>>,
    pub prompt_overlay: Option<String>,
    pub runtime_mode: RuntimeMode,
}

/// Base tool names for a builtin `extends` id.
#[must_use]
pub fn base_tools_for_extends(extends: &str, include_skill_on_explore_plan: bool) -> Vec<String> {
    match extends.trim() {
        "general-purpose" => general_purpose_tool_names(),
        "explore" | "plan" => explore_plan_tool_names_with_skill(include_skill_on_explore_plan),
        "workspace-assistant" => workspace_assistant_tool_names(include_skill_on_explore_plan),
        "goal" => general_purpose_tool_names(),
        other if is_builtin_extends(other) => general_purpose_tool_names(),
        _ => explore_plan_tool_names_with_skill(include_skill_on_explore_plan),
    }
}

#[must_use]
pub fn apply_tool_filters(
    mut tools: Vec<String>,
    allow: Option<&[String]>,
    deny: Option<&[String]>,
) -> Vec<String> {
    if let Some(allow) = allow {
        let set: HashSet<&str> = allow
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if !set.is_empty() {
            tools.retain(|t| set.contains(t.as_str()));
        }
    }
    if let Some(deny) = deny {
        let set: HashSet<&str> = deny
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        tools.retain(|t| !set.contains(t.as_str()));
    }
    tools.sort();
    tools.dedup();
    tools
}

/// Resolve a declarative profile into runtime capabilities (Strategy single point).
#[must_use]
pub fn resolve_profile(
    id: &str,
    spec: &AgentProfileSpec,
    include_skill_on_explore_plan: bool,
) -> ResolvedAgentProfile {
    let extends = spec.extends.trim();
    let extends = if extends.is_empty() {
        "general-purpose"
    } else {
        extends
    };
    if !is_builtin_extends(extends) {
        tracing::warn!(
            target: "anycode_agent",
            "agent profile `{id}`: unknown extends `{extends}`, using general-purpose"
        );
    }
    let base = base_tools_for_extends(extends, include_skill_on_explore_plan);
    let tools = apply_tool_filters(
        base,
        spec.tools_allow.as_deref(),
        spec.tools_deny.as_deref(),
    );
    let description = spec
        .description
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| format!("Custom agent profile `{id}` extending `{extends}`"));
    ResolvedAgentProfile {
        id: id.to_string(),
        extends: extends.to_string(),
        description,
        tools,
        skills_allowlist: spec.skills_allowlist.clone(),
        prompt_overlay: spec.prompt_overlay.clone(),
        runtime_mode: runtime_mode_for_extends(extends),
    }
}

#[must_use]
pub fn is_builtin_extends(id: &str) -> bool {
    BUILTIN_EXTENDS.contains(&id.trim())
}

/// Shipped role preset ids (extends builtins) registered by CLI bootstrap when not overridden in config.
pub const SHIPPED_ROLE_IDS: &[&str] = &[
    "builder",
    "planner",
    "explorer",
    "verifier",
    "reviewer",
    "channel-ops",
    "goal-runner",
    "office-writer",
    "data-analyst",
    "researcher",
    "file-operator",
];

/// Canonical declarative spec for a builtin or shipped role id (single catalog source).
#[must_use]
pub fn profile_spec_for_builtin(id: &str) -> Option<AgentProfileSpec> {
    let seed = BUILTIN_AGENT_SEED.iter().find(|s| s.id == id)?;
    let mut spec = AgentProfileSpec {
        extends: seed.extends.to_string(),
        description: Some(seed.description.to_string()),
        tools_allow: None,
        tools_deny: None,
        skills_allowlist: None,
        prompt_overlay: None,
    };
    match id {
        "verifier" => {
            spec.tools_deny = Some(vec!["Bash".into(), "Edit".into(), "FileWrite".into()]);
        }
        "reviewer" => {
            spec.tools_allow = Some(vec![
                "FileRead".into(),
                "Grep".into(),
                "Glob".into(),
                "StructuredOutput".into(),
            ]);
        }
        "office-writer" => {
            spec.skills_allowlist = Some(vec![
                "content-repurpose".into(),
                "doc-summary".into(),
                "md-to-pdf".into(),
                "weekly-report".into(),
            ]);
            spec.prompt_overlay = Some(
                "You are an office writing assistant. Produce clear Markdown drafts; do not publish externally. Use KnowledgeSearch for indexed project materials when paths are configured.".into(),
            );
        }
        "data-analyst" => {
            spec.skills_allowlist = Some(vec![
                "doc-summary".into(),
                "report-to-csv".into(),
                "weekly-report".into(),
            ]);
            spec.prompt_overlay = Some(
                "Focus on accurate data summaries and tables; cite source files. Use KnowledgeSearch and report-to-csv when exporting tabular results.".into(),
            );
        }
        "researcher" => {
            spec.skills_allowlist = Some(vec!["daily-brief".into()]);
            spec.prompt_overlay = Some(
                "Gather sources with WebSearch/WebFetch; synthesize with citations. Bind daily-brief skill for scheduled summaries.".into(),
            );
        }
        "file-operator" => {
            spec.skills_allowlist = Some(vec!["file-organizer".into()]);
        }
        _ => {}
    }
    Some(spec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deny_removes_tools() {
        let base = vec!["A".into(), "B".into(), "C".into()];
        let out = apply_tool_filters(base, None, Some(&["B".into()]));
        assert_eq!(out, vec!["A", "C"]);
    }
}
