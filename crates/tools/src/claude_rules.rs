//! Claude 风格权限规则编译与求值（deny / allow / ask）。

use crate::mcp_normalization::blanket_deny_rule_matches_tool;
use crate::permission_rule_parser::permission_rule_value_from_string;
use crate::shell_rule_match::command_matches_shell_rule_content;
use std::sync::Arc;

#[derive(Debug, Clone)]
struct DenyEntry {
    tool_name: String,
}

#[derive(Debug, Clone)]
struct ContentEntry {
    tool_name: String,
    rule_content: String,
}

#[derive(Debug, Default)]
pub struct CompiledClaudePermissionRules {
    deny_blanket: Vec<DenyEntry>,
    deny_content: Vec<ContentEntry>,
    allow_blanket: Vec<DenyEntry>,
    allow_content: Vec<ContentEntry>,
    ask_blanket: Vec<DenyEntry>,
    ask_content: Vec<ContentEntry>,
}

impl CompiledClaudePermissionRules {
    pub fn compile(deny: &[String], allow: &[String], ask: &[String]) -> Arc<Self> {
        let mut out = Self::default();
        for s in deny {
            out.push_rule(s, RuleKind::Deny);
        }
        for s in allow {
            out.push_rule(s, RuleKind::Allow);
        }
        for s in ask {
            out.push_rule(s, RuleKind::Ask);
        }
        Arc::new(out)
    }

    fn push_rule(&mut self, s: &str, kind: RuleKind) {
        let v = permission_rule_value_from_string(s);
        if v.tool_name.is_empty() {
            return;
        }
        let entry = DenyEntry {
            tool_name: v.tool_name.clone(),
        };
        let content_entry = v.rule_content.as_ref().map(|c| ContentEntry {
            tool_name: v.tool_name.clone(),
            rule_content: c.clone(),
        });
        let is_blanket = v.rule_content.is_none();
        match kind {
            RuleKind::Deny => {
                if is_blanket {
                    self.deny_blanket.push(entry);
                } else if let Some(ce) = content_entry {
                    self.deny_content.push(ce);
                }
            }
            RuleKind::Allow => {
                if is_blanket {
                    self.allow_blanket.push(entry);
                } else if let Some(ce) = content_entry {
                    self.allow_content.push(ce);
                }
            }
            RuleKind::Ask => {
                if is_blanket {
                    self.ask_blanket.push(entry);
                } else if let Some(ce) = content_entry {
                    self.ask_content.push(ce);
                }
            }
        }
    }

    /// LLM 工具列表过滤：仅无 `ruleContent` 的 deny（+ 与 TS 一致的 MCP 全名）。
    pub fn blanket_denies_tool(&self, tool_api_name: &str) -> bool {
        self.deny_blanket
            .iter()
            .any(|e| blanket_rule_matches_tool(e, tool_api_name))
    }

    /// 执行前：content 级 deny（Bash(command) 等）。
    pub fn content_denies(&self, tool_api_name: &str, args_json: &str) -> bool {
        self.deny_content.iter().any(|e| {
            e.tool_name == tool_api_name
                && content_matches(&e.rule_content, tool_api_name, args_json)
        })
    }

    /// 执行前：content 级 allow（覆盖 deny）。
    pub fn content_allows(&self, tool_api_name: &str, args_json: &str) -> bool {
        self.allow_content.iter().any(|e| {
            e.tool_name == tool_api_name
                && content_matches(&e.rule_content, tool_api_name, args_json)
        })
    }

    pub fn blanket_allows_tool(&self, tool_api_name: &str) -> bool {
        self.allow_blanket
            .iter()
            .any(|e| blanket_rule_matches_tool(e, tool_api_name))
    }

    /// 是否需要用户确认（ask 命中且未被 allow 覆盖）。
    pub fn needs_ask(&self, tool_api_name: &str, args_json: &str) -> bool {
        let ask_hit = self
            .ask_blanket
            .iter()
            .any(|e| blanket_rule_matches_tool(e, tool_api_name))
            || self.ask_content.iter().any(|e| {
                e.tool_name == tool_api_name
                    && content_matches(&e.rule_content, tool_api_name, args_json)
            });
        if !ask_hit {
            return false;
        }
        if self.blanket_allows_tool(tool_api_name) {
            return false;
        }
        if self.content_allows(tool_api_name, args_json) {
            return false;
        }
        true
    }
}

enum RuleKind {
    Deny,
    Allow,
    Ask,
}

fn blanket_rule_matches_tool(e: &DenyEntry, tool_api_name: &str) -> bool {
    blanket_deny_rule_matches_tool(&e.tool_name, tool_api_name)
}

fn content_matches(rule_content: &str, tool_api_name: &str, args_json: &str) -> bool {
    let t = tool_api_name;
    if t == "Bash" || t == "Shell" {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(args_json) {
            if let Some(cmd) = v.get("command").and_then(|x| x.as_str()) {
                return command_matches_shell_rule_content(rule_content, cmd);
            }
        }
        return false;
    }
    if t.starts_with("mcp__") {
        return args_json.contains(rule_content);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bash_content_deny_and_allow_override() {
        let r = CompiledClaudePermissionRules::compile(
            &["Bash(npm *)".to_string()],
            &["Bash(npm install)".to_string()],
            &[],
        );
        let args = r#"{"command":"npm install"}"#;
        assert!(r.content_denies("Bash", args));
        assert!(r.content_allows("Bash", args));
    }

    #[test]
    fn blanket_deny_lists_tools() {
        let r = CompiledClaudePermissionRules::compile(&["Bash".to_string()], &[], &[]);
        assert!(r.blanket_denies_tool("Bash"));
        assert!(!r.blanket_denies_tool("FileRead"));
    }
}
