//! 工具结果 JSON、单行摘要与工具块级排版（`⏺` / `⎿`）。

use crate::i18n::{tr, tr_args};
use crate::md_tui::{render_markdown_styled, wrap_plain_bullet_prefixed, wrap_plain_prefixed};
use fluent_bundle::FluentArgs;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use serde_json::{Map, Value};
use std::borrow::Cow;

use anycode_core::strip_llm_reasoning_xml_blocks;

use super::types::WorkspaceLiveLayout;
use crate::tui::styles::*;

/// 单个 ToolCall / ToolResult 块在 Workspace 中最多占用的物理行数（超出则截断并提示）。
pub(crate) const MAX_TOOL_BLOCK_LINES: usize = 64;
const TOOL_BODY_INDENT_COLS: usize = 3;
/// 折叠块下 `⎿` 预览路径条数上限。
pub(crate) const MAX_COLLAPSED_PATH_PREVIEWS: usize = 4;

pub(crate) fn push_lines_truncated(
    out: &mut Vec<Line<'static>>,
    mut block: Vec<Line<'static>>,
    max_lines: usize,
) {
    if block.len() <= max_lines {
        out.extend(block);
        return;
    }
    let keep = max_lines.saturating_sub(1);
    out.extend(block.drain(..keep));
    out.push(Line::from(Span::styled(
        tr("tui-block-truncated"),
        style_dim(),
    )));
}

/// 剥离 `ERROR: …` / `RESULT: …` 包装后尝试解析 JSON。
/// 若整段是仅含 `content` 字符串的 JSON（模型偶发），展开为正文再渲染。
pub(crate) fn unwrap_single_content_json<'a>(text: &'a str) -> Cow<'a, str> {
    let t = text.trim();
    if !t.starts_with('{') {
        return Cow::Borrowed(text);
    }
    let Ok(v) = serde_json::from_str::<Value>(t) else {
        return Cow::Borrowed(text);
    };
    let Value::Object(map) = v else {
        return Cow::Borrowed(text);
    };
    if map.len() != 1 {
        return Cow::Borrowed(text);
    }
    if let Some(Value::String(c)) = map.get("content") {
        return Cow::Owned(c.clone());
    }
    Cow::Borrowed(text)
}

/// 比较 assistant 正文（含 `unwrap_single_content_json`）是否与 `final_text` 语义一致。
pub(crate) fn assistant_markdown_meaningful_eq(stored: &str, candidate: &str) -> bool {
    let a = strip_llm_reasoning_xml_blocks(unwrap_single_content_json(stored).as_ref());
    let b = strip_llm_reasoning_xml_blocks(unwrap_single_content_json(candidate).as_ref());
    a.trim() == b.trim()
}

fn parse_tool_result_json(raw: &str) -> Option<Value> {
    let mut s = raw.trim();
    if let Some(pos) = s.find("\nRESULT: ") {
        s = s[pos + "\nRESULT: ".len()..].trim();
    } else if let Some(rest) = s.strip_prefix("RESULT: ") {
        s = rest.trim();
    }
    serde_json::from_str(s).ok()
}

fn short_tool_use_id(id: &str) -> String {
    if id.len() <= 10 {
        return id.to_string();
    }
    format!("…{}", &id[id.len().saturating_sub(8)..])
}

fn tool_result_header(tool_name: Option<&str>, tool_use_id: &str) -> String {
    let tail = short_tool_use_id(tool_use_id);
    match tool_name {
        Some(n) if !n.is_empty() => format!("{n} · {tail}"),
        _ => format!("result · {tail}"),
    }
}

fn ordered_object_keys(map: &Map<String, Value>) -> Vec<String> {
    let preferred = [
        "error",
        "message",
        "content",
        "stdout",
        "stderr",
        "path",
        "file_path",
        "matches",
        "result",
        "success",
    ];
    let mut keys: Vec<String> = map.keys().cloned().collect();
    keys.sort_by(|a, b| {
        let ia = preferred.iter().position(|p| *p == a.as_str());
        let ib = preferred.iter().position(|p| *p == b.as_str());
        match (ia, ib) {
            (Some(ia), Some(ib)) => ia.cmp(&ib),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            _ => a.cmp(b),
        }
    });
    keys
}

fn format_json_object_human(map: &Map<String, Value>) -> String {
    let keys = ordered_object_keys(map);
    let mut out = String::new();
    for k in keys {
        let Some(val) = map.get(&k) else {
            continue;
        };
        if !out.is_empty() {
            out.push('\n');
        }
        match val {
            Value::String(s) => {
                out.push_str(&k);
                out.push_str(":\n");
                for line in s.lines() {
                    out.push_str("  ");
                    out.push_str(line);
                    out.push('\n');
                }
            }
            Value::Object(_) | Value::Array(_) => {
                out.push_str(&k);
                out.push_str(":\n");
                let inner = serde_json::to_string_pretty(val).unwrap_or_else(|_| val.to_string());
                for line in inner.lines() {
                    out.push_str("  ");
                    out.push_str(line);
                    out.push('\n');
                }
            }
            _ => {
                out.push_str(&k);
                out.push_str(": ");
                out.push_str(&val.to_string());
            }
        }
    }
    out.trim_end().to_string()
}

fn format_value_human(v: &Value) -> String {
    match v {
        Value::Null => "(null)".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(arr) => {
            if arr.iter().all(|x| x.is_string()) {
                arr.iter()
                    .filter_map(|x| x.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string())
            }
        }
        Value::Object(map) => format_json_object_human(map),
    }
}

/// 对 FileRead / Grep 等：主 `content` 用 Markdown 渲染，其余键单独列印。
fn markdown_eligible_content(map: &Map<String, Value>, tool_name: Option<&str>) -> Option<String> {
    let content = map.get("content")?.as_str()?;
    let tn = tool_name.unwrap_or("");
    let rich = matches!(
        tn,
        "FileRead" | "FileWrite" | "Grep" | "Glob" | "WebSearch" | "WebFetch"
    );
    if rich {
        return Some(content.to_string());
    }
    if content.contains("```")
        || content.contains("\n#")
        || content.starts_with('#')
        || content.len() > 400
    {
        return Some(content.to_string());
    }
    None
}

fn prefix_block_lines(
    mut lines: Vec<Line<'static>>,
    prefix: &str,
    pstyle: Style,
) -> Vec<Line<'static>> {
    let p = Span::styled(prefix.to_string(), pstyle);
    for line in &mut lines {
        let mut spans = vec![p.clone()];
        spans.extend(line.spans.clone());
        *line = Line::from(spans);
    }
    lines
}

/// 工具结果正文（不含 ⏷/⎿ 标题行），供 ToolResult 与 ToolTurn 共用。
fn layout_tool_body_content(
    tool_name: Option<&str>,
    body: &str,
    is_error: bool,
    w: usize,
) -> Vec<Line<'static>> {
    let mut block: Vec<Line<'static>> = Vec::new();
    let mut rest = body.trim_end();
    let mut error_line: Option<&str> = None;
    if let Some(stripped) = rest.strip_prefix("ERROR: ") {
        if let Some(nl) = stripped.find('\n') {
            error_line = Some(stripped[..nl].trim());
            rest = stripped[nl + 1..].trim_start();
        } else {
            error_line = Some(stripped.trim());
            rest = "";
        }
    }
    if is_error {
        if let Some(e) = error_line {
            block.push(Line::from(Span::styled(
                format!("   error: {e}"),
                style_error(),
            )));
        } else {
            block.push(Line::from(Span::styled("   (error)", style_error())));
        }
    }

    // 成功但 stdout/正文为空：不渲染占位（对齐 Claude Code：无输出则不占一行）
    if rest.is_empty() {
        return block;
    }

    let inner_w = w.saturating_sub(TOOL_BODY_INDENT_COLS).max(8);
    let base_style = if is_error {
        style_error()
    } else {
        style_tool_result().add_modifier(Modifier::DIM)
    };

    if let Some(v) = parse_tool_result_json(rest) {
        if let Value::Object(map) = &v {
            if let Some(md) = markdown_eligible_content(map, tool_name) {
                for key in ["path", "file_path"] {
                    if let Some(p) = map.get(key).and_then(|x| x.as_str()) {
                        block.extend(wrap_plain_prefixed(
                            "   ",
                            &format!("{key}: {p}"),
                            style_dim(),
                            w,
                        ));
                    }
                }
                for key in ["success", "error"] {
                    if key == "error" && error_line.is_some() {
                        continue;
                    }
                    if let Some(val) = map.get(key) {
                        if !val.is_null() && !(val.is_string() && val.as_str() == Some("")) {
                            block.extend(wrap_plain_prefixed(
                                "   ",
                                &format!("{key}: {val}"),
                                style_dim(),
                                w,
                            ));
                        }
                    }
                }
                let mut md_lines = render_markdown_styled(&md, inner_w, base_style);
                md_lines = prefix_block_lines(md_lines, "   ", style_dim());
                block.extend(md_lines);
                return block;
            }

            let human = format_json_object_human(map);
            if !human.is_empty() {
                block.extend(wrap_plain_prefixed("   ", &human, base_style, w));
                return block;
            }
        }

        let human = format_value_human(&v);
        if !human.is_empty() {
            block.extend(wrap_plain_prefixed("   ", &human, base_style, w));
            return block;
        }
    }

    block.extend(wrap_plain_prefixed("   ", rest, base_style, w));
    block
}

/// 旧式 ⏷ 块（孤儿 ToolResult 等）。
pub(crate) fn layout_tool_result_block(
    tool_name: Option<&str>,
    tool_use_id: &str,
    body: &str,
    is_error: bool,
    w: usize,
) -> Vec<Line<'static>> {
    let mut block: Vec<Line<'static>> = Vec::new();
    let title = tool_result_header(tool_name, tool_use_id);
    block.extend(wrap_plain_prefixed(
        "⏷ ",
        title.as_str(),
        style_tool().add_modifier(Modifier::BOLD),
        w,
    ));
    block.extend(layout_tool_body_content(tool_name, body, is_error, w));
    block
}

/// Claude 风格单行摘要：`Bash(cd …)` / `Read path`。
fn tool_invocation_one_liner(name: &str, args: &str, max_chars: usize) -> String {
    let v: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let mut s = match name {
        "Bash" | "PowerShell" => v
            .get("command")
            .and_then(|x| x.as_str())
            .map(|c| c.trim().replace('\n', " "))
            .unwrap_or_else(|| args.chars().take(120).collect::<String>()),
        "FileRead" => v
            .get("file_path")
            .and_then(|x| x.as_str())
            .map(|p| format!("{p}"))
            .unwrap_or_else(|| args.chars().take(120).collect::<String>()),
        "FileWrite" => v
            .get("file_path")
            .and_then(|x| x.as_str())
            .map(|p| format!("write {p}"))
            .unwrap_or_else(|| args.chars().take(120).collect::<String>()),
        "Grep" | "Glob" => v
            .get("pattern")
            .or_else(|| v.get("glob_pattern"))
            .and_then(|x| x.as_str())
            .map(|p| format!("{p}"))
            .unwrap_or_else(|| args.chars().take(120).collect::<String>()),
        _ => args.lines().next().unwrap_or(args).trim().to_string(),
    };
    s = s.replace('\n', " ");
    if s.chars().count() > max_chars {
        s = format!(
            "{}…",
            s.chars()
                .take(max_chars.saturating_sub(1))
                .collect::<String>()
        );
    }
    format!("{name}({s})")
}

fn prefix_lines_braille(
    mut lines: Vec<Line<'static>>,
    first_prefix: &str,
    rest_prefix: &str,
    pstyle: Style,
) -> Vec<Line<'static>> {
    for (i, line) in lines.iter_mut().enumerate() {
        let p = if i == 0 {
            Span::styled(first_prefix.to_string(), pstyle)
        } else {
            Span::styled(rest_prefix.to_string(), pstyle)
        };
        let mut spans = vec![p];
        spans.extend(line.spans.clone());
        *line = Line::from(spans);
    }
    lines
}

/// 工具标题行：`text_style` 为正文；仅 `⏺` 在活动时随 `pulse_frame` 压暗。
pub(crate) fn assistant_tool_header_styles(
    text_dim_inactive: bool,
    is_active: bool,
    live: WorkspaceLiveLayout,
) -> (Style, Style) {
    let text_style = if text_dim_inactive {
        style_assistant()
            .add_modifier(Modifier::BOLD)
            .add_modifier(Modifier::DIM)
    } else {
        style_assistant().add_modifier(Modifier::BOLD)
    };
    let bullet_style = if is_active && live.executing && live.pulse_frame % 2 == 0 {
        text_style.add_modifier(Modifier::DIM)
    } else {
        text_style
    };
    (bullet_style, text_style)
}

pub(crate) fn layout_tool_turn_block(
    _fold_id: u64,
    name: &str,
    args: &str,
    tool_name: Option<&str>,
    body: &str,
    is_error: bool,
    expanded: bool,
    w: usize,
    is_active: bool,
    live: WorkspaceLiveLayout,
) -> Vec<Line<'static>> {
    let shell_tool = matches!(name, "Bash" | "PowerShell");
    let mut summary = tool_invocation_one_liner(name, args, w.saturating_sub(8).max(24));
    if is_active && !expanded {
        summary.push('…');
    }
    let text_dim_inactive = !expanded && !is_active;
    let (bullet_style, text_style) =
        assistant_tool_header_styles(text_dim_inactive, is_active, live);
    let header_text = if !expanded {
        format!("{} {}", summary, tr("tui-expand-hint"))
    } else {
        summary.clone()
    };
    let mut header =
        wrap_plain_bullet_prefixed("⏺ ", bullet_style, header_text.as_str(), text_style, w);

    // 对齐 Claude：未展开时**不**渲染工具输出正文；shell 执行中显示 `⎿  Running…`
    if !expanded {
        if is_active && !is_error && shell_tool && body.trim().is_empty() {
            let run = Line::from(Span::styled(tr("tui-tool-running"), style_dim()));
            header.extend(prefix_lines_braille(vec![run], "⎿  ", "   ", style_dim()));
        }
        return header;
    }

    let mut body_lines = layout_tool_body_content(tool_name, body, is_error, w);
    if body_lines.is_empty() && is_active && !is_error && shell_tool {
        body_lines.push(Line::from(Span::styled(
            tr("tui-tool-running"),
            style_dim(),
        )));
    }
    let braille_style = style_dim();
    let mut body_prefixed = prefix_lines_braille(body_lines, "⎿  ", "   ", braille_style);
    header.append(&mut body_prefixed);
    header
}

pub(crate) fn file_path_from_file_read_args(args: &str) -> Option<String> {
    let v: Value = serde_json::from_str(args).ok()?;
    v.get("file_path")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
}

/// 同一轮内多次 `FileRead` 合并后的排版：折叠为「Read N files」单行摘要（展开后见全文）。
pub(crate) fn layout_read_tool_batch(
    _fold_id: u64,
    parts: &[(String, String, bool)],
    expanded: bool,
    w: usize,
    is_active: bool,
    live: WorkspaceLiveLayout,
) -> Vec<Line<'static>> {
    let n = parts.len();
    if n == 0 {
        return Vec::new();
    }
    let mut n_arg = FluentArgs::new();
    n_arg.set("n", n as i64);
    let title_expanded = if is_active {
        tr_args("tui-read-ex-a", &n_arg)
    } else {
        tr_args("tui-read-ex-i", &n_arg)
    };
    let mut col_arg = FluentArgs::new();
    col_arg.set("n", n as i64);
    col_arg.set("hint", tr("tui-expand-hint"));
    let title_collapsed = if is_active {
        tr_args("tui-read-col-a", &col_arg)
    } else {
        tr_args("tui-read-col-i", &col_arg)
    };
    let (bullet_style, text_style) = assistant_tool_header_styles(!is_active, is_active, live);
    let header = wrap_plain_bullet_prefixed(
        "⏺ ",
        bullet_style,
        if expanded {
            title_expanded.as_str()
        } else {
            title_collapsed.as_str()
        },
        text_style,
        w,
    );

    if !expanded {
        let mut out = header;
        let show = n.min(MAX_COLLAPSED_PATH_PREVIEWS);
        for (args, _, _) in parts.iter().take(show) {
            if let Some(p) = file_path_from_file_read_args(args) {
                out.extend(wrap_plain_prefixed("   ⎿  ", p.as_str(), style_dim(), w));
            }
        }
        if n > MAX_COLLAPSED_PATH_PREVIEWS {
            let extra = n - MAX_COLLAPSED_PATH_PREVIEWS;
            let mut a = FluentArgs::new();
            a.set("n", extra as i64);
            out.push(Line::from(Span::styled(
                tr_args("tui-read-more-paths", &a),
                style_dim(),
            )));
        }
        out
    } else {
        let mut out = header;
        for (i, (args, body, err)) in parts.iter().enumerate() {
            let one =
                tool_invocation_one_liner("FileRead", args.as_str(), w.saturating_sub(12).max(16));
            let sub = format!("[{}/{}] {}", i + 1, n, one);
            out.extend(wrap_plain_prefixed("   ", sub.as_str(), style_dim(), w));
            let bl = layout_tool_body_content(Some("FileRead"), body.as_str(), *err, w);
            out.append(&mut prefix_lines_braille(bl, "⎿  ", "   ", style_dim()));
        }
        out
    }
}

#[cfg(test)]
mod tool_render_tests {
    use super::*;
    use std::borrow::Cow;

    #[test]
    fn parse_tool_result_strips_result_prefix() {
        let raw = "RESULT: {\"content\":\"a\\nb\",\"path\":\"/x\"}";
        let v = parse_tool_result_json(raw).expect("json");
        let obj = v.as_object().unwrap();
        assert_eq!(obj.get("path").and_then(|x| x.as_str()), Some("/x"));
        assert_eq!(obj.get("content").and_then(|x| x.as_str()), Some("a\nb"));
    }

    #[test]
    fn unwrap_single_content_json_expands() {
        let s = "{\"content\":\"# Hi\\n\\nok\"}";
        match unwrap_single_content_json(s) {
            Cow::Owned(c) => assert!(c.contains("# Hi")),
            _ => unreachable!("test: expected owned Cow from unwrap_single_content_json"),
        }
    }

    #[test]
    fn layout_tool_body_empty_success_has_no_placeholder() {
        assert!(layout_tool_body_content(Some("Bash"), "", false, 80).is_empty());
    }

    #[test]
    fn layout_tool_turn_bash_collapsed_running_shows_subline() {
        let lines = layout_tool_turn_block(
            1,
            "Bash",
            r#"{"command":"pwd"}"#,
            None,
            "",
            false,
            false,
            80,
            true,
            WorkspaceLiveLayout {
                executing: true,
                ..Default::default()
            },
        );
        let joined: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
            .collect();
        assert!(
            joined.contains("Running"),
            "expected Claude-style ⎿ Running…, got {joined:?}"
        );
    }
}
