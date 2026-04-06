//! Claude Code `permissionRuleValueFromString` / 转义 — 对齐 `permissionRuleParser.ts`。

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionRuleValue {
    pub tool_name: String,
    pub rule_content: Option<String>,
}

fn find_first_unescaped_char(s: &str, ch: char) -> Option<usize> {
    for (i, c) in s.char_indices() {
        if c != ch {
            continue;
        }
        let mut backslashes = 0usize;
        let mut j = i;
        while j > 0 {
            j -= 1;
            if s.as_bytes()[j] == b'\\' {
                backslashes += 1;
            } else {
                break;
            }
        }
        if backslashes % 2 == 0 {
            return Some(i);
        }
    }
    None
}

fn find_last_unescaped_char(s: &str, ch: char) -> Option<usize> {
    for (i, c) in s.char_indices().rev() {
        if c != ch {
            continue;
        }
        let mut backslashes = 0usize;
        let mut j = i;
        while j > 0 {
            j -= 1;
            if s.as_bytes()[j] == b'\\' {
                backslashes += 1;
            } else {
                break;
            }
        }
        if backslashes % 2 == 0 {
            return Some(i);
        }
    }
    None
}

pub fn escape_rule_content(content: &str) -> String {
    content
        .replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}

pub fn unescape_rule_content(raw: &str) -> String {
    raw.replace("\\)", ")")
        .replace("\\(", "(")
        .replace("\\\\", "\\")
}

/// 与 Claude `normalizeLegacyToolName` 对齐（anycode 工具名）。
pub fn normalize_legacy_tool_name(name: &str) -> String {
    match name.trim() {
        "Task" => "Agent".to_string(),
        "KillShell" => "TaskStop".to_string(),
        "AgentOutputTool" | "BashOutputTool" => "TaskOutput".to_string(),
        "Brief" => "SendUserMessage".to_string(),
        _ => name.trim().to_string(),
    }
}

/// `permissionRuleValueFromString`
pub fn permission_rule_value_from_string(rule_string: &str) -> PermissionRuleValue {
    let rule_string = rule_string.trim();
    if rule_string.is_empty() {
        return PermissionRuleValue {
            tool_name: String::new(),
            rule_content: None,
        };
    }
    let open = match find_first_unescaped_char(rule_string, '(') {
        Some(i) => i,
        None => {
            return PermissionRuleValue {
                tool_name: normalize_legacy_tool_name(rule_string),
                rule_content: None,
            };
        }
    };
    let close = match find_last_unescaped_char(rule_string, ')') {
        Some(i) => i,
        None => {
            return PermissionRuleValue {
                tool_name: normalize_legacy_tool_name(rule_string),
                rule_content: None,
            };
        }
    };
    if close <= open || close != rule_string.len() - 1 {
        return PermissionRuleValue {
            tool_name: normalize_legacy_tool_name(rule_string),
            rule_content: None,
        };
    }
    let tool_name = &rule_string[..open];
    if tool_name.is_empty() {
        return PermissionRuleValue {
            tool_name: normalize_legacy_tool_name(rule_string),
            rule_content: None,
        };
    }
    let raw_content = &rule_string[open + 1..close];
    if raw_content.is_empty() || raw_content == "*" {
        return PermissionRuleValue {
            tool_name: normalize_legacy_tool_name(tool_name),
            rule_content: None,
        };
    }
    PermissionRuleValue {
        tool_name: normalize_legacy_tool_name(tool_name),
        rule_content: Some(unescape_rule_content(raw_content)),
    }
}

pub fn permission_rule_value_to_string(v: &PermissionRuleValue) -> String {
    match &v.rule_content {
        None => v.tool_name.clone(),
        Some(c) => format!("{}({})", v.tool_name, escape_rule_content(c)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_like_claude_doc() {
        let a = permission_rule_value_from_string("Bash");
        assert_eq!(a.tool_name, "Bash");
        assert!(a.rule_content.is_none());

        let b = permission_rule_value_from_string("Bash(npm install)");
        assert_eq!(b.tool_name, "Bash");
        assert_eq!(b.rule_content.as_deref(), Some("npm install"));
    }

    #[test]
    fn legacy_task_to_agent() {
        let t = permission_rule_value_from_string("Task");
        assert_eq!(t.tool_name, "Agent");
    }
}
