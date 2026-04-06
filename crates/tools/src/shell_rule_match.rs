//! Shell 权限规则 — 对齐 `shellRuleMatching.ts` 的 `parsePermissionRule` + `matchWildcardPattern`。

use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellRule {
    Exact { command: String },
    Prefix { prefix: String },
    Wildcard { pattern: String },
}

fn has_wildcards(s: &str) -> bool {
    s.contains('*') || s.contains('?')
}

/// 与 TS `parsePermissionRule` 对齐（trim + 前缀/通配/精确）。
pub fn parse_shell_permission_rule(rule_content: &str) -> ShellRule {
    let trimmed = rule_content.trim();
    if trimmed.is_empty() {
        return ShellRule::Exact {
            command: String::new(),
        };
    }
    if trimmed.ends_with(":*") && trimmed.len() > 2 {
        return ShellRule::Prefix {
            prefix: trimmed[..trimmed.len() - 2].trim().to_string(),
        };
    }
    if has_wildcards(trimmed) {
        return ShellRule::Wildcard {
            pattern: trimmed.to_string(),
        };
    }
    ShellRule::Exact {
        command: trimmed.to_string(),
    }
}

/// 与 TS `matchWildcardPattern` 对齐（`*` / `?`，可选忽略大小写）。
pub fn match_wildcard_pattern(pattern: &str, text: &str, case_insensitive: bool) -> bool {
    let mut re = String::with_capacity(pattern.len() * 2 + 2);
    re.push('^');
    for ch in pattern.chars() {
        match ch {
            '*' => re.push_str(".*"),
            '?' => re.push('.'),
            '.' | '+' | '^' | '$' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '\\' => {
                re.push('\\');
                re.push(ch);
            }
            _ => re.push(ch),
        }
    }
    re.push('$');
    let flags = if case_insensitive {
        "(?i)"
    } else {
        ""
    };
    let full = format!("{flags}{re}");
    Regex::new(&full)
        .map(|r| r.is_match(text))
        .unwrap_or(false)
}

/// `command` 是否匹配一条 Shell 规则内容（Bash 等）。
pub fn command_matches_shell_rule_content(rule_content: &str, command: &str) -> bool {
    let cmd = command.trim();
    match parse_shell_permission_rule(rule_content) {
        ShellRule::Exact { command: c } => c == cmd,
        ShellRule::Prefix { prefix } => cmd.starts_with(&prefix),
        ShellRule::Wildcard { pattern } => match_wildcard_pattern(&pattern, cmd, true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcard_star() {
        assert!(command_matches_shell_rule_content("npm *", "npm install"));
        assert!(!command_matches_shell_rule_content("npm *", "yarn install"));
    }

    #[test]
    fn prefix() {
        assert!(command_matches_shell_rule_content("git:*", "git pull"));
    }
}
