//! MCP 工具/服务器名规范化与 deny 规则匹配 — 与 Claude Code 对齐。
//!
//! 参考本地镜像：`../claude-code/src/services/mcp/normalization.ts`、`mcpStringUtils.ts`、
//! `src/tools.ts`（`filterToolsByDenyRules` / `assembleToolPool`）、`src/utils/permissions/permissions.ts`（`toolMatchesRule`）。

/// Claude.ai 服务器名前缀（与 TS `CLAUDEAI_SERVER_PREFIX` 一致）
const CLAUDEAI_SERVER_PREFIX: &str = "claude.ai ";

/// 将名称规范为 API 友好片段：`^[a-zA-Z0-9_-]{1,64}$` 语义（非法字符替换为 `_`）。
///
/// 与 `normalizeNameForMCP`（TypeScript）一致：**保留大小写**与 **连字符 `-`**。
pub fn normalize_name_for_mcp(name: &str) -> String {
    let normalized: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if name.starts_with(CLAUDEAI_SERVER_PREFIX) {
        return collapse_underscores_trim_edges(&normalized);
    }
    normalized
}

fn collapse_underscores_trim_edges(s: &str) -> String {
    let mut out = String::new();
    let mut prev_us = false;
    for c in s.chars() {
        if c == '_' {
            if prev_us {
                continue;
            }
            prev_us = true;
            out.push('_');
        } else {
            prev_us = false;
            out.push(c);
        }
    }
    out.trim_matches('_').to_string()
}

/// 解析 `mcp__serverName__toolName`（与 `mcpInfoFromString` 一致；`tool` 段可含 `__`）。
pub fn mcp_info_from_string(tool_string: &str) -> Option<(String, Option<String>)> {
    let parts: Vec<&str> = tool_string.split("__").collect();
    if parts.len() < 2 {
        return None;
    }
    if parts[0] != "mcp" {
        return None;
    }
    let server_name = parts[1].to_string();
    if server_name.is_empty() {
        return None;
    }
    let tool_name = if parts.len() > 2 {
        Some(parts[2..].join("__"))
    } else {
        None
    };
    Some((server_name, tool_name))
}

/// 无 `ruleContent` 的 blanket deny：`toolMatchesRule` 中与 MCP 相关的部分。
///
/// - 规则串与工具 API 名完全一致 → 匹配
/// - 规则为 `mcp__<server>`（无第三段）→ 匹配该 server 下所有 `mcp__<server>__*`
/// - 规则为 `mcp__<server>__*` → 同上
pub fn blanket_deny_rule_matches_tool(rule_tool_name: &str, tool_api_name: &str) -> bool {
    let rule_tool_name = rule_tool_name.trim();
    if rule_tool_name.is_empty() {
        return false;
    }
    if rule_tool_name == tool_api_name {
        return true;
    }
    let rule_info = mcp_info_from_string(rule_tool_name);
    let tool_info = mcp_info_from_string(tool_api_name);
    match (rule_info, tool_info) {
        (Some((rs, r_tool)), Some((ts, _))) => {
            rs == ts && (r_tool.is_none() || r_tool.as_deref() == Some("*"))
        }
        _ => false,
    }
}

/// `buildMcpToolName`：完整 MCP 工具 API 名。
pub fn build_mcp_tool_name(server_name: &str, tool_name: &str) -> String {
    format!(
        "mcp__{}__{}",
        normalize_name_for_mcp(server_name),
        normalize_name_for_mcp(tool_name)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_matches_claude_examples() {
        assert_eq!(normalize_name_for_mcp("My Server"), "My_Server");
        assert_eq!(normalize_name_for_mcp("test-case"), "test-case");
        assert_eq!(normalize_name_for_mcp("a.b"), "a_b");
    }

    #[test]
    fn normalize_claude_ai_prefix_collapses_underscores() {
        let n = normalize_name_for_mcp("claude.ai Foo  Bar");
        assert!(!n.contains("__"), "{}", n);
        assert!(!n.starts_with('_') && !n.ends_with('_'), "{}", n);
    }

    #[test]
    fn mcp_info_roundtrip() {
        let s = "mcp__my__server__tool";
        let info = mcp_info_from_string(s).unwrap();
        assert_eq!(info.0, "my");
        assert_eq!(info.1.as_deref(), Some("server__tool"));
    }

    #[test]
    fn blanket_deny_server_level() {
        assert!(blanket_deny_rule_matches_tool("mcp__slack", "mcp__slack__send"));
        assert!(blanket_deny_rule_matches_tool("mcp__slack__*", "mcp__slack__send"));
        assert!(blanket_deny_rule_matches_tool("mcp__slack__send", "mcp__slack__send"));
        assert!(!blanket_deny_rule_matches_tool("mcp__slack__send", "mcp__slack__other"));
    }

    #[test]
    fn blanket_deny_builtin_exact() {
        assert!(blanket_deny_rule_matches_tool("Bash", "Bash"));
        assert!(!blanket_deny_rule_matches_tool("Bash", "FileRead"));
    }
}
