//! 内置 Agent 约定：小写 id、`/` 切换命令、子 Agent 默认类型。

/// 与 `AgentRuntime::new` 注册的 `AgentType` 一致。
pub const BUILTIN_AGENT_IDS: [&str; 5] = [
    "general-purpose",
    "explore",
    "plan",
    "workspace-assistant",
    "goal",
];

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
