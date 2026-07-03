//! 内置 Agent 约定：小写 id、`/` 切换命令、子 Agent 默认类型。

use anycode_agent::BUILTIN_AGENT_SEED;

/// 与 `AgentRuntime::new` 注册的 `AgentType` 一致。
pub const BUILTIN_AGENT_IDS: [&str; 5] = [
    "general-purpose",
    "explore",
    "plan",
    "workspace-assistant",
    "goal",
];

/// Shipped declarative role profiles (always registered at runtime).
pub const SHIPPED_PROFILE_IDS: [&str; 11] = [
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

/// Routing-only compaction key (not a registered agent).
pub const ROUTING_ONLY_AGENT_IDS: [&str; 1] = ["summary"];

#[must_use]
pub fn is_known_agent_id(id: &str) -> bool {
    let t = id.trim();
    if t.is_empty() {
        return false;
    }
    BUILTIN_AGENT_IDS.contains(&t)
        || SHIPPED_PROFILE_IDS.contains(&t)
        || ROUTING_ONLY_AGENT_IDS.contains(&t)
        || BUILTIN_AGENT_SEED.iter().any(|s| s.id == t)
        || matches!(t, "workspace" | "code")
}

/// TUI / REPL 中 `/…` 切换当前会话 Agent；返回目标 id。
pub fn parse_agent_slash_command(trimmed: &str) -> Option<&'static str> {
    match trimmed {
        "/general-purpose" => Some("general-purpose"),
        "/explore" => Some("explore"),
        "/plan" => Some("plan"),
        "/workspace-assistant" => Some("workspace-assistant"),
        "/goal" => Some("goal"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anycode_agent::{BUILTIN_AGENT_SEED, SHIPPED_ROLE_IDS};

    #[test]
    fn shipped_profile_ids_match_catalog_seed() {
        assert_eq!(SHIPPED_PROFILE_IDS.len(), SHIPPED_ROLE_IDS.len());
        for id in SHIPPED_PROFILE_IDS {
            assert!(
                BUILTIN_AGENT_SEED.iter().any(|s| s.id == id),
                "missing shipped profile `{id}` in BUILTIN_AGENT_SEED"
            );
            assert!(
                SHIPPED_ROLE_IDS.contains(&id),
                "SHIPPED_PROFILE_IDS out of sync with agent crate for `{id}`"
            );
        }
    }
}
