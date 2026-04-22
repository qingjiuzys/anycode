//! `/` 命令解析与（仅单测使用的）补全辅助；流式终端不展示候选菜单。
#![allow(dead_code)]

use crate::md_render::pad_end_to_display_width;
use anycode_core::{SlashCommand, SlashCommandScope, BUILTIN_SLASH_COMMANDS};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedSlashCommand {
    Mode(Option<String>),
    Status,
    /// `None` = `/compact`；`Some` = 自定义压缩说明（与全屏 TUI 一致）。
    Compact(Option<String>),
    Clear,
    Workflow(Option<String>),
    /// `None` = cwd 优先解析最近会话；`Some("list")`；`Some(uuid)` 显式 id。
    Session(Option<String>),
    /// 从系统剪贴板插入 UTF-8（与 `repl_clipboard` 一致）。
    Paste,
    /// 只读上下文与上一轮用量（与 HUD / 脚标同源字段）。
    Context,
    /// 可选路径；默认写到 cwd 下 `anycode-export-<session>.txt`。
    Export(Option<String>),
    /// 与 `/context` 同源数据 + 无货币计费说明（诚实版 usage）。
    Cost,
}

pub fn registry() -> &'static [SlashCommand] {
    BUILTIN_SLASH_COMMANDS
}

pub fn help_lines() -> Vec<String> {
    let rows = registry();
    let name_col_w = rows
        .iter()
        .map(|c| format!("/{}", c.name).width())
        .max()
        .unwrap_or(8)
        .clamp(10, 22);
    let scope_w = 8usize;
    rows.iter()
        .map(|cmd| {
            let scope = match cmd.scope {
                SlashCommandScope::Local => "local",
                SlashCommandScope::Runtime => "runtime",
                SlashCommandScope::PromptOnly => "prompt",
            };
            let cmd_col = pad_end_to_display_width(&format!("/{}", cmd.name), name_col_w);
            format!(
                "{cmd_col}  {scope:<scope_w$}  {}",
                cmd.summary,
                scope_w = scope_w
            )
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

/// 斜杠补全列表里「命令」列的统一显示宽度，使右侧说明纵向对齐。
/// `inner_w_cols` 为可用行宽（与 `Rect::width` 一致）。
pub(crate) fn slash_menu_cmd_column_width(
    candidates: &[SlashSuggestionItem],
    start: usize,
    end: usize,
    inner_w_cols: usize,
) -> usize {
    const PREFIX_W: usize = 2; // "▸ " / "  "
    const GAP_W: usize = 2; // 命令列与说明之间的空白
    /// 说明列至少保留的显示宽度（过窄时宁可缩短命令列，避免说明被折行挤乱）。
    const MIN_DESC: usize = 14;
    let cap = inner_w_cols
        .saturating_sub(PREFIX_W + GAP_W + MIN_DESC)
        .max(8);
    let m = (start..end)
        .map(|i| crate::md_render::text_display_width(candidates[i].display.as_str()))
        .max()
        .unwrap_or(0);
    m.max(8).min(cap)
}

/// 完整斜杠目录（用于输入了首字母后的模糊/前缀匹配，以及 ghost 补全）。
fn full_catalog_rows() -> Vec<(&'static str, &'static str)> {
    let mut rows: Vec<(&'static str, &'static str)> =
        registry().iter().map(|c| (c.name, c.summary)).collect();
    for (name, summary) in [
        ("help", "帮助与快捷键"),
        ("agents", "列出可用 Agent"),
        ("tools", "列出工具"),
        ("clear", "清空会话与 transcript"),
        ("paste", "从剪贴板插入文本"),
        ("context", "只读上下文与 token 用量摘要"),
        ("export", "导出会话为纯文本文件"),
        ("cost", "上一轮 token 用量（无美元估算）"),
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

/// 仅输入 `/`（未输入命令名片段）时展示的常用项，减少列表噪音；其余命令通过输入首字母筛选。
fn primary_catalog_rows(
    full: &[(&'static str, &'static str)],
) -> Vec<(&'static str, &'static str)> {
    const ORDER: &[&str] = &[
        "help", "clear", "mode", "session", "context", "cost", "export", "status", "compact",
        "exit",
    ];
    ORDER
        .iter()
        .filter_map(|name| full.iter().find(|(n, _)| *n == *name).copied())
        .collect()
}

/// 模糊 + 前缀优先级排序（对齐 Claude `commandSuggestions` + Fuse 排序思想）。
/// 仅 `/` 时用短列表；有查询前缀时在全量目录中匹配（含 `agents` / `tools` / `workflow` 等）。
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
    let full = full_catalog_rows();
    let rows: Vec<(&'static str, &'static str)> = if query.is_empty() {
        primary_catalog_rows(&full)
    } else {
        full
    };
    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(i64, &'static str, &'static str)> = Vec::new();

    for (name, desc) in rows {
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
    if before.is_empty() || !before.starts_with('/') {
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
    for (name, _) in full_catalog_rows() {
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
        "status" => Some(ParsedSlashCommand::Status),
        "compact" => Some(ParsedSlashCommand::Compact(arg)),
        "clear" => Some(ParsedSlashCommand::Clear),
        "workflow" => Some(ParsedSlashCommand::Workflow(arg)),
        "session" => Some(ParsedSlashCommand::Session(arg)),
        "paste" => Some(ParsedSlashCommand::Paste),
        "context" => Some(ParsedSlashCommand::Context),
        "export" => Some(ParsedSlashCommand::Export(arg)),
        "cost" => Some(ParsedSlashCommand::Cost),
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
    fn slash_empty_lists_primary_only() {
        let v = slash_suggestions_for_first_line("/");
        assert_eq!(v.len(), 10);
        let names: Vec<_> = v.iter().map(|s| s.id.as_str()).collect();
        assert!(names.contains(&"help"));
        assert!(names.contains(&"session"));
        assert!(!names.contains(&"agents"));
    }

    #[test]
    fn slash_typed_still_finds_secondary_commands() {
        let a = slash_suggestions_for_first_line("/a");
        assert!(a.iter().any(|s| s.id == "agents"));
        let w = slash_suggestions_for_first_line("/w");
        assert!(w.iter().any(|s| s.id == "workflow"));
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

    #[test]
    fn parse_compact_and_clear() {
        assert_eq!(parse("/compact"), Some(ParsedSlashCommand::Compact(None)));
        assert_eq!(
            parse("/compact focus on next steps"),
            Some(ParsedSlashCommand::Compact(Some(
                "focus on next steps".to_string()
            )))
        );
        assert_eq!(parse("/clear"), Some(ParsedSlashCommand::Clear));
    }

    #[test]
    fn parse_session_variants() {
        assert_eq!(parse("/session"), Some(ParsedSlashCommand::Session(None)));
        assert_eq!(
            parse("/session list"),
            Some(ParsedSlashCommand::Session(Some("list".to_string())))
        );
        assert_eq!(
            parse("/session 550e8400-e29b-41d4-a716-446655440000"),
            Some(ParsedSlashCommand::Session(Some(
                "550e8400-e29b-41d4-a716-446655440000".to_string()
            )))
        );
    }

    #[test]
    fn parse_cost() {
        assert_eq!(parse("/cost"), Some(ParsedSlashCommand::Cost));
    }

    #[test]
    fn parse_context_and_export() {
        assert_eq!(parse("/context"), Some(ParsedSlashCommand::Context));
        assert_eq!(parse("/export"), Some(ParsedSlashCommand::Export(None)));
        assert_eq!(
            parse("/export out.txt"),
            Some(ParsedSlashCommand::Export(Some("out.txt".to_string())))
        );
    }
}
