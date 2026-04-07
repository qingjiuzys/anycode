use anycode_core::{SlashCommand, SlashCommandScope, BUILTIN_SLASH_COMMANDS};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedSlashCommand {
    Mode(Option<String>),
    Model(Option<String>),
    Status,
    Compact,
    Memory,
    Approve,
    Workflow(Option<String>),
}

pub fn registry() -> &'static [SlashCommand] {
    BUILTIN_SLASH_COMMANDS
}

pub fn help_lines() -> Vec<String> {
    registry()
        .iter()
        .map(|cmd| {
            let scope = match cmd.scope {
                SlashCommandScope::Local => "local",
                SlashCommandScope::Runtime => "runtime",
                SlashCommandScope::PromptOnly => "prompt",
            };
            format!("/{:<10} {:<7} {}", cmd.name, scope, cmd.summary)
        })
        .collect()
}

/// 取首行，用于 `/` 命令补全（多行输入时只处理第一行）。
pub fn first_line(buffer: &str) -> &str {
    buffer.split('\n').next().unwrap_or("")
}

/// 对齐 Claude `hasCommandArgs`：已有「命令名后的实质参数」时为 true（`/mode x`），
/// 仅 `/` 或 `/mode` 或 `/mode `（尾随空格）为 false。
pub fn has_command_args_claude(first_line: &str) -> bool {
    if !first_line.starts_with('/') {
        return false;
    }
    if !first_line.contains(' ') {
        return false;
    }
    if first_line.ends_with(' ') {
        return false;
    }
    true
}

/// 单条斜杠候选（对齐 Claude `SuggestionItem`：展示名 + 说明）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashSuggestionItem {
    pub id: String,
    pub display: String,
    pub description: String,
    /// 写入首行的完整串，含尾部空格（如 `/mode `）。
    pub replacement: String,
}

fn catalog_rows() -> Vec<(&'static str, &'static str)> {
    let mut rows: Vec<(&'static str, &'static str)> =
        registry().iter().map(|c| (c.name, c.summary)).collect();
    for (name, summary) in [
        ("help", "帮助与快捷键"),
        ("agents", "列出可用 Agent"),
        ("tools", "列出工具"),
        ("clear", "清空会话与 transcript"),
        ("exit", "退出 TUI"),
    ] {
        if !rows.iter().any(|(n, _)| *n == name) {
            rows.push((name, summary));
        }
    }
    rows.sort_by(|a, b| a.0.cmp(b.0));
    rows.dedup_by_key(|(n, _)| *n);
    rows
}

/// 模糊 + 前缀优先级排序（对齐 Claude `commandSuggestions` + Fuse 排序思想）。
pub fn slash_suggestions_for_first_line(buffer: &str) -> Vec<SlashSuggestionItem> {
    let first = first_line(buffer);
    if !first.starts_with('/') {
        return Vec::new();
    }
    if has_command_args_claude(first) {
        return Vec::new();
    }
    let rest = first.strip_prefix('/').unwrap_or("");
    let (token, after_space) = match rest.find(char::is_whitespace) {
        None => (rest, ""),
        Some(i) => (&rest[..i], &rest[i..]),
    };
    if !after_space.trim().is_empty() {
        return Vec::new();
    }
    let query = token.to_ascii_lowercase();
    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(i64, &'static str, &'static str)> = Vec::new();

    for (name, desc) in catalog_rows() {
        let name_l = name.to_ascii_lowercase();
        let desc_l = desc.to_ascii_lowercase();
        let score: Option<i64> = if query.is_empty() {
            Some(0)
        } else if name_l == query {
            Some(i64::MAX / 4)
        } else if name_l.starts_with(&query) {
            Some(1_000_000_000 - name_l.len() as i64)
        } else {
            let hay = format!("{name_l} {desc_l}");
            matcher.fuzzy_match(&hay, &query)
        };
        if let Some(s) = score {
            scored.push((s, name, desc));
        }
    }

    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));

    let mut seen = std::collections::HashSet::<String>::new();
    let mut out: Vec<SlashSuggestionItem> = Vec::new();
    for (_, name, desc) in scored {
        let id = name.to_string();
        if !seen.insert(id.clone()) {
            continue;
        }
        out.push(SlashSuggestionItem {
            id: id.clone(),
            display: format!("/{name}"),
            description: desc.to_string(),
            replacement: format!("/{name} "),
        });
    }
    out
}

/// 行内 ghost 后缀（对齐 Claude `getBestCommandMatch` 的 prefix 补全）。
/// `cursor` 为字符下标；光标须在首行，且处于 `/` 命令名片段内。
pub fn slash_ghost_suffix(buffer: &str, cursor: usize) -> Option<String> {
    let full = buffer;
    let before: String = buffer.chars().take(cursor).collect();
    if before.contains('\n') {
        return None;
    }
    let first = first_line(full);
    if !first.starts_with('/') || has_command_args_claude(first) {
        return None;
    }
    if before.len() < 1 || !before.starts_with('/') {
        return None;
    }
    let rest = before.strip_prefix('/')?;
    if rest.contains(char::is_whitespace) {
        return None;
    }
    let partial = rest;
    if partial.is_empty() {
        return None;
    }
    let partial_l = partial.to_ascii_lowercase();
    let mut best: Option<&'static str> = None;
    for (name, _) in catalog_rows() {
        let nl = name.to_ascii_lowercase();
        if nl.starts_with(&partial_l) && nl.len() > partial_l.len() {
            match best {
                None => best = Some(name),
                Some(cur) => {
                    let cur_l = cur.to_ascii_lowercase();
                    if nl.len() < cur_l.len() || (nl.len() == cur_l.len() && name < cur) {
                        best = Some(name);
                    }
                }
            }
        }
    }
    best.map(|name| {
        let nl = name.to_ascii_lowercase();
        nl[partial_l.len()..].to_string()
    })
}

/// 只替换缓冲区第一行，保留后续行。
pub fn replace_first_line(buffer: &str, new_first: &str) -> String {
    match buffer.split_once('\n') {
        None => new_first.to_string(),
        Some((_, tail)) => format!("{new_first}\n{tail}"),
    }
}

pub fn parse(input: &str) -> Option<ParsedSlashCommand> {
    let trimmed = input.trim();
    let body = trimmed.strip_prefix('/')?;
    let mut parts = body.split_whitespace();
    let cmd = parts.next()?.to_ascii_lowercase();
    let rest = parts.collect::<Vec<_>>().join(" ");
    let arg = if rest.trim().is_empty() {
        None
    } else {
        Some(rest)
    };
    match cmd.as_str() {
        "mode" => Some(ParsedSlashCommand::Mode(arg)),
        "model" => Some(ParsedSlashCommand::Model(arg)),
        "status" => Some(ParsedSlashCommand::Status),
        "compact" => Some(ParsedSlashCommand::Compact),
        "memory" => Some(ParsedSlashCommand::Memory),
        "approve" => Some(ParsedSlashCommand::Approve),
        "workflow" => Some(ParsedSlashCommand::Workflow(arg)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_args_claude() {
        assert!(!has_command_args_claude("/mode"));
        assert!(!has_command_args_claude("/mode "));
        assert!(has_command_args_claude("/mode x"));
    }

    #[test]
    fn slash_fuzzy_and_prefix() {
        let v = slash_suggestions_for_first_line("/m");
        assert!(v.iter().any(|s| s.display == "/mode"));
        let st = slash_suggestions_for_first_line("/st");
        assert!(st.iter().any(|s| s.display == "/status"));
    }

    #[test]
    fn slash_args_empty() {
        assert!(slash_suggestions_for_first_line("/mode code").is_empty());
        assert!(slash_suggestions_for_first_line("hello").is_empty());
    }

    #[test]
    fn slash_multiline_first_line_only() {
        let ml = slash_suggestions_for_first_line("/st\nsecond");
        assert!(ml.iter().any(|s| s.display == "/status"));
    }

    #[test]
    fn replace_first_line_keeps_tail() {
        assert_eq!(replace_first_line("a\nb", "/x"), "/x\nb".to_string());
    }
}
