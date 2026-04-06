//! Workspace 消息块与 Markdown 排版。
//!
//! Claude Code 对齐：`apply_tool_transcript_pipeline` 为工具展示唯一归并路径；`rebuild_live_turn_tail` / 收尾判定供 TUI 实时与 turn 结束共用。

use crate::i18n::{tr, tr_args};
use crate::md_tui::{
    render_markdown, render_markdown_styled, wrap_plain_bullet_prefixed, wrap_plain_prefixed,
    wrap_ratatui_line,
};
use anycode_core::{
    Message, MessageContent, MessageRole, ToolCall, ANYCODE_TOOL_CALLS_METADATA_KEY,
};
use fluent_bundle::FluentArgs;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use serde_json::{Map, Value};
use std::borrow::Cow;
use std::path::Path;

use super::styles::*;

/// 单个 ToolCall / ToolResult 块在 Workspace 中最多占用的物理行数（超出则截断并提示）。
const MAX_TOOL_BLOCK_LINES: usize = 64;
const TOOL_BODY_INDENT_COLS: usize = 3;
/// 折叠块下 `⎿` 预览路径条数上限。
const MAX_COLLAPSED_PATH_PREVIEWS: usize = 4;

fn push_lines_truncated(
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
fn unwrap_single_content_json<'a>(text: &'a str) -> Cow<'a, str> {
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
    unwrap_single_content_json(stored).trim() == unwrap_single_content_json(candidate).trim()
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

    if rest.is_empty() && error_line.is_some() && !is_error {
        // 纯 ERROR 行且未标 is_error 的遗留格式
        block.push(Line::from(Span::styled("   <empty>", style_dim())));
        return block;
    }
    if rest.is_empty() {
        block.push(Line::from(Span::styled("   <empty>", style_dim())));
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
fn layout_tool_result_block(
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
fn assistant_tool_header_styles(
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

fn layout_tool_turn_block(
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

    // 对齐 Claude：未展开时**不**渲染工具输出正文（省算力 + 避免挡掉文末总结）。
    if !expanded {
        return header;
    }

    let body_lines = layout_tool_body_content(tool_name, body, is_error, w);
    let braille_style = style_dim();
    let mut body_prefixed = prefix_lines_braille(body_lines, "⎿  ", "   ", braille_style);
    header.append(&mut body_prefixed);
    header
}

/// 同一轮内多次 `FileRead` 合并后的排版：折叠为「Read N files」单行摘要（展开后见全文）。
fn layout_read_tool_batch(
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

/// 归并 ToolCall+ToolResult（按 `tool_use_id`）；仅在新用户消息时刷掉未匹配的 ToolCall。
pub(crate) fn normalize_transcript_global(
    entries: &mut Vec<TranscriptEntry>,
    next_fold_id: &mut u64,
) {
    use std::collections::HashMap;

    let mut calls: HashMap<String, (String, String)> = HashMap::new();
    let mut order: Vec<String> = Vec::new();

    let flush_calls = |calls: &mut HashMap<String, (String, String)>,
                       order: &mut Vec<String>,
                       out: &mut Vec<TranscriptEntry>| {
        for id in order.drain(..) {
            if let Some((name, args)) = calls.remove(&id) {
                out.push(TranscriptEntry::ToolCall {
                    tool_use_id: id.clone(),
                    name,
                    args,
                });
            }
        }
    };

    let mut out: Vec<TranscriptEntry> = Vec::with_capacity(entries.len());

    for e in entries.drain(..) {
        match e {
            TranscriptEntry::User(u) => {
                flush_calls(&mut calls, &mut order, &mut out);
                out.push(TranscriptEntry::User(u));
            }
            TranscriptEntry::ToolCall {
                tool_use_id,
                name,
                args,
            } => {
                if !calls.contains_key(&tool_use_id) {
                    order.push(tool_use_id.clone());
                }
                calls.insert(tool_use_id, (name, args));
            }
            TranscriptEntry::ToolResult {
                tool_use_id,
                tool_name,
                body,
                is_error,
            } => {
                if let Some((name, args)) = calls.remove(&tool_use_id) {
                    order.retain(|x| x != &tool_use_id);
                    *next_fold_id = next_fold_id.saturating_add(1);
                    out.push(TranscriptEntry::ToolTurn {
                        fold_id: *next_fold_id,
                        name,
                        args,
                        tool_use_id,
                        tool_name,
                        body,
                        is_error,
                    });
                } else {
                    out.push(TranscriptEntry::ToolResult {
                        tool_use_id,
                        tool_name,
                        body,
                        is_error,
                    });
                }
            }
            TranscriptEntry::ToolTurn { .. }
            | TranscriptEntry::ReadToolBatch { .. }
            | TranscriptEntry::CollapsedToolGroup { .. }
            | TranscriptEntry::AssistantMarkdown(_)
            | TranscriptEntry::Plain(_) => {
                out.push(e);
            }
        }
    }
    flush_calls(&mut calls, &mut order, &mut out);
    *entries = out;
}

/// 将连续的 `FileRead` `ToolTurn` 合并为一条 Read 批注（对齐 Claude Code `Reading N files… (ctrl+o to expand)`）。
pub(crate) fn coalesce_read_tool_batches(
    entries: &mut Vec<TranscriptEntry>,
    next_fold_id: &mut u64,
) {
    let old = std::mem::take(entries);
    let mut it = old.into_iter().peekable();
    while let Some(e) = it.next() {
        match e {
            TranscriptEntry::ReadToolBatch { .. } => {
                entries.push(e);
            }
            TranscriptEntry::ToolTurn {
                fold_id,
                name,
                args,
                tool_use_id,
                tool_name,
                body,
                is_error,
            } if name == "FileRead" => {
                let mut group: Vec<(u64, String, String, Option<String>, String, bool)> =
                    vec![(fold_id, args, tool_use_id, tool_name, body, is_error)];
                while let Some(TranscriptEntry::ToolTurn { name: ref nm, .. }) = it.peek() {
                    if nm != "FileRead" {
                        break;
                    }
                    let Some(TranscriptEntry::ToolTurn {
                        fold_id,
                        args,
                        tool_use_id,
                        tool_name,
                        body,
                        is_error,
                        ..
                    }) = it.next()
                    else {
                        break;
                    };
                    group.push((fold_id, args, tool_use_id, tool_name, body, is_error));
                }
                if group.len() == 1 {
                    let (fold_id, args, tool_use_id, tool_name, body, is_error) =
                        group.into_iter().next().expect("one");
                    entries.push(TranscriptEntry::ToolTurn {
                        fold_id,
                        name: "FileRead".to_string(),
                        args,
                        tool_use_id,
                        tool_name,
                        body,
                        is_error,
                    });
                } else {
                    *next_fold_id = next_fold_id.saturating_add(1);
                    let parts: Vec<(String, String, bool)> = group
                        .into_iter()
                        .map(|(_, a, _, _, b, e)| (a, b, e))
                        .collect();
                    entries.push(TranscriptEntry::ReadToolBatch {
                        fold_id: *next_fold_id,
                        parts,
                    });
                }
            }
            other => entries.push(other),
        }
    }
}

/// 与 Claude `collapsed_read_search` 对齐：合并展示用的子块（由连续可折叠 `ToolTurn` / `ReadToolBatch` 压平而来）。
#[derive(Clone, Debug)]
pub(crate) enum CollapsibleToolBlock {
    Turn {
        fold_id: u64,
        name: String,
        args: String,
        #[allow(dead_code)] // 与 ToolTurn 对齐保留，折叠 UI 当前不展示 id
        tool_use_id: String,
        tool_name: Option<String>,
        body: String,
        is_error: bool,
    },
    ReadBatch {
        fold_id: u64,
        parts: Vec<(String, String, bool)>,
    },
}

fn tool_turn_is_collapsible(name: &str) -> bool {
    matches!(name, "Bash" | "PowerShell" | "Grep" | "Glob" | "FileRead")
}

fn transcript_entry_is_collapsible(e: &TranscriptEntry) -> bool {
    match e {
        TranscriptEntry::ReadToolBatch { .. } => true,
        TranscriptEntry::ToolTurn { name, .. } => tool_turn_is_collapsible(name),
        _ => false,
    }
}

fn entry_into_collapsible_block(e: TranscriptEntry) -> Option<CollapsibleToolBlock> {
    match e {
        TranscriptEntry::ToolTurn {
            fold_id,
            name,
            args,
            tool_use_id,
            tool_name,
            body,
            is_error,
        } if tool_turn_is_collapsible(&name) => Some(CollapsibleToolBlock::Turn {
            fold_id,
            name,
            args,
            tool_use_id,
            tool_name,
            body,
            is_error,
        }),
        TranscriptEntry::ReadToolBatch { fold_id, parts } => {
            Some(CollapsibleToolBlock::ReadBatch { fold_id, parts })
        }
        _ => None,
    }
}

fn collapsible_block_into_entry(b: CollapsibleToolBlock) -> TranscriptEntry {
    match b {
        CollapsibleToolBlock::Turn {
            fold_id,
            name,
            args,
            tool_use_id,
            tool_name,
            body,
            is_error,
        } => TranscriptEntry::ToolTurn {
            fold_id,
            name,
            args,
            tool_use_id,
            tool_name,
            body,
            is_error,
        },
        CollapsibleToolBlock::ReadBatch { fold_id, parts } => {
            TranscriptEntry::ReadToolBatch { fold_id, parts }
        }
    }
}

fn display_path_hint(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(path)
        .to_string()
}

fn command_as_hint_line(command: &str) -> String {
    let one_line = command
        .split('\n')
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let prefixed = format!("$ {one_line}");
    const MAX: usize = 200;
    if prefixed.chars().count() > MAX {
        format!(
            "{}…",
            prefixed
                .chars()
                .take(MAX.saturating_sub(1))
                .collect::<String>()
        )
    } else {
        prefixed
    }
}

fn file_path_from_file_read_args(args: &str) -> Option<String> {
    let v: Value = serde_json::from_str(args).ok()?;
    v.get("file_path")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
}

fn hint_from_tool_turn(name: &str, args: &str) -> Option<String> {
    let v: Value = serde_json::from_str(args).ok()?;
    match name {
        "FileRead" => file_path_from_file_read_args(args).map(|p| display_path_hint(&p)),
        "Grep" => v
            .get("pattern")
            .and_then(|x| x.as_str())
            .map(|p| format!("\"{p}\"")),
        "Glob" => v
            .get("glob_pattern")
            .or_else(|| v.get("pattern"))
            .and_then(|x| x.as_str())
            .map(|p| format!("\"{p}\"")),
        "Bash" | "PowerShell" => v
            .get("command")
            .and_then(|x| x.as_str())
            .map(command_as_hint_line),
        _ => None,
    }
}

fn collect_read_paths_from_blocks(blocks: &[CollapsibleToolBlock]) -> Vec<String> {
    let mut paths = Vec::new();
    for b in blocks {
        match b {
            CollapsibleToolBlock::ReadBatch { parts, .. } => {
                for (args, _, _) in parts {
                    if let Some(p) = file_path_from_file_read_args(args) {
                        paths.push(p);
                    }
                }
            }
            CollapsibleToolBlock::Turn { name, args, .. } if name == "FileRead" => {
                if let Some(p) = file_path_from_file_read_args(args) {
                    paths.push(p);
                }
            }
            _ => {}
        }
    }
    paths
}

fn latest_display_hint_for_blocks(blocks: &[CollapsibleToolBlock]) -> Option<String> {
    for b in blocks.iter().rev() {
        match b {
            CollapsibleToolBlock::ReadBatch { parts, .. } => {
                if let Some((args, _, _)) = parts.last() {
                    if let Some(h) = hint_from_tool_turn("FileRead", args) {
                        return Some(h);
                    }
                }
            }
            CollapsibleToolBlock::Turn { name, args, .. } => {
                if let Some(h) = hint_from_tool_turn(name, args) {
                    return Some(h);
                }
            }
        }
    }
    None
}

fn collapsed_group_counts(blocks: &[CollapsibleToolBlock]) -> (usize, usize, usize, usize) {
    let mut search = 0usize;
    let mut read = 0usize;
    let mut list = 0usize;
    let mut bash = 0usize;
    for b in blocks {
        match b {
            CollapsibleToolBlock::ReadBatch { parts, .. } => read += parts.len(),
            CollapsibleToolBlock::Turn { name, .. } => match name.as_str() {
                "Grep" => search += 1,
                "Glob" => list += 1,
                "FileRead" => read += 1,
                "Bash" | "PowerShell" => bash += 1,
                _ => {}
            },
        }
    }
    (search, read, list, bash)
}

fn capitalize_first(s: &str) -> String {
    let mut it = s.chars();
    match it.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + it.as_str(),
    }
}

/// 绘制时「是否仍在跑 turn」：与 Claude `isActiveGroup` + `inProgressToolUseIDs` 同目的。
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct WorkspaceLiveLayout {
    pub executing: bool,
    /// 与顶栏一致：整秒，用于长 bash 时在摘要后挂 `(Ns)`（对齐 `bash_progress` ≥2s）。
    pub working_elapsed_secs: Option<u64>,
    /// 与 Buddy 同节拍（约 250ms×4），用于活动行 `⏺` 呼吸闪烁。
    pub pulse_frame: u64,
}

/// 执行中在偶数帧压暗，形成与 Claude 类似的「⏺ 闪烁」观感（不依赖终端 BLINK）。
fn pulse_dim_assistant_bold(live: WorkspaceLiveLayout, enabled: bool) -> Style {
    let mut s = style_assistant().add_modifier(Modifier::BOLD);
    if enabled && live.executing && live.pulse_frame % 2 == 0 {
        s = s.add_modifier(Modifier::DIM);
    }
    s
}

fn workspace_gap_before(prev: &TranscriptEntry, cur: &TranscriptEntry) -> bool {
    match (prev, cur) {
        (TranscriptEntry::ToolCall { .. }, TranscriptEntry::ToolResult { .. }) => false,
        _ => true,
    }
}

fn last_user_entry_index(entries: &[TranscriptEntry]) -> Option<usize> {
    entries
        .iter()
        .rposition(|e| matches!(e, TranscriptEntry::User(_)))
}

/// Turn 进行中：在最后一条用户消息之后显示 Germinating（会话流内，与 Prompt Dock 解耦）；已有非空 assistant 正文则隐藏。
fn should_show_turn_status_after_user(
    entries: &[TranscriptEntry],
    user_idx: usize,
    live: WorkspaceLiveLayout,
) -> bool {
    if !live.executing {
        return false;
    }
    if last_user_entry_index(entries) != Some(user_idx) {
        return false;
    }
    for e in entries.iter().skip(user_idx + 1) {
        match e {
            TranscriptEntry::AssistantMarkdown(s) if !s.trim().is_empty() => return false,
            TranscriptEntry::User(_) => break,
            _ => {}
        }
    }
    true
}

fn turn_status_line(live: WorkspaceLiveLayout) -> Line<'static> {
    let label = match live.working_elapsed_secs {
        Some(s) if s >= 1 => {
            let mut a = FluentArgs::new();
            a.set("s", s);
            tr_args("tui-germinating-secs", &a)
        }
        _ => tr("tui-germinating"),
    };
    Line::from(vec![
        Span::styled("⏺ ", pulse_dim_assistant_bold(live, true)),
        Span::styled(label, style_assistant().add_modifier(Modifier::ITALIC)),
    ])
}

/// 从尾部找「当前轮」仍在展开的工具块 `fold_id`（跳过 `ToolCall`/`Plain` 等）。
fn last_tool_ui_fold_id(entries: &[TranscriptEntry], live: WorkspaceLiveLayout) -> Option<u64> {
    for e in entries.iter().rev() {
        match e {
            TranscriptEntry::CollapsedToolGroup { fold_id, .. }
            | TranscriptEntry::ReadToolBatch { fold_id, .. }
            | TranscriptEntry::ToolTurn { fold_id, .. } => return Some(*fold_id),
            TranscriptEntry::User(_) => return None,
            TranscriptEntry::AssistantMarkdown(_) => {
                if !live.executing {
                    return None;
                }
            }
            _ => {}
        }
    }
    None
}

fn is_active_tool_fold(
    entries: &[TranscriptEntry],
    fold_id: u64,
    live: WorkspaceLiveLayout,
) -> bool {
    live.executing && last_tool_ui_fold_id(entries, live) == Some(fold_id)
}

/// 与 `CollapsedReadSearchContent` 一致：`active` 用现在时 + 句内小写衔接；否则过去时。
fn col_unit_key(n: usize, one: &'static str, many: &'static str) -> String {
    if n == 1 {
        tr(one)
    } else {
        tr(many)
    }
}

fn build_collapsed_summary_line(
    search: usize,
    read: usize,
    list: usize,
    bash: usize,
    active: bool,
) -> String {
    let mut fragments: Vec<String> = Vec::new();
    if search > 0 {
        let mut a = FluentArgs::new();
        a.set("n", search as i64);
        a.set(
            "unit",
            col_unit_key(search, "tui-col-unit-pattern", "tui-col-unit-patterns"),
        );
        fragments.push(tr_args(
            if active {
                "tui-col-search-a"
            } else {
                "tui-col-search-i"
            },
            &a,
        ));
    }
    if read > 0 {
        let mut a = FluentArgs::new();
        a.set("n", read as i64);
        a.set(
            "unit",
            col_unit_key(read, "tui-col-unit-file", "tui-col-unit-files"),
        );
        fragments.push(tr_args(
            if active {
                "tui-col-read-a"
            } else {
                "tui-col-read-i"
            },
            &a,
        ));
    }
    if list > 0 {
        let mut a = FluentArgs::new();
        a.set("n", list as i64);
        a.set(
            "unit",
            col_unit_key(list, "tui-col-unit-dir", "tui-col-unit-dirs"),
        );
        fragments.push(tr_args(
            if active {
                "tui-col-list-a"
            } else {
                "tui-col-list-i"
            },
            &a,
        ));
    }
    if bash > 0 {
        let mut a = FluentArgs::new();
        a.set("n", bash as i64);
        a.set(
            "unit",
            col_unit_key(bash, "tui-col-unit-cmd", "tui-col-unit-cmds"),
        );
        fragments.push(tr_args(
            if active {
                "tui-col-bash-a"
            } else {
                "tui-col-bash-i"
            },
            &a,
        ));
    }
    if fragments.is_empty() {
        return tr("tui-col-tools");
    }
    let sep = tr("tui-col-sep");
    let mut out = String::new();
    for (i, frag) in fragments.iter().enumerate() {
        if i > 0 {
            out.push_str(&sep);
            out.push_str(frag);
        } else {
            out.push_str(&capitalize_first(frag));
        }
    }
    out
}

fn layout_collapsed_tool_group(
    group_fold_id: u64,
    blocks: &[CollapsibleToolBlock],
    expanded: bool,
    w: usize,
    expanded_tool_folds: &std::collections::HashSet<u64>,
    entries: &[TranscriptEntry],
    live: WorkspaceLiveLayout,
) -> Vec<Line<'static>> {
    if expanded {
        let mut out: Vec<Line<'static>> = Vec::new();
        for b in blocks {
            match b {
                CollapsibleToolBlock::ReadBatch {
                    fold_id: fid,
                    parts,
                } => {
                    let child_active = is_active_tool_fold(entries, *fid, live);
                    let mut block =
                        layout_read_tool_batch(*fid, parts.as_slice(), true, w, child_active, live);
                    out.append(&mut block);
                }
                CollapsibleToolBlock::Turn {
                    fold_id: fid,
                    name,
                    args,
                    tool_name,
                    body,
                    is_error,
                    ..
                } => {
                    let child_active = is_active_tool_fold(entries, *fid, live);
                    let mut block = layout_tool_turn_block(
                        *fid,
                        name.as_str(),
                        args.as_str(),
                        tool_name.as_deref(),
                        body.as_str(),
                        *is_error,
                        expanded_tool_folds.contains(fid),
                        w,
                        child_active,
                        live,
                    );
                    out.append(&mut block);
                }
            }
        }
        return out;
    }

    let is_active = is_active_tool_fold(entries, group_fold_id, live);
    let (s, r, l, b) = collapsed_group_counts(blocks);
    let mut summary = build_collapsed_summary_line(s, r, l, b, is_active);
    if is_active && b > 0 {
        if let Some(secs) = live.working_elapsed_secs {
            if secs >= 2 {
                let mut a = FluentArgs::new();
                a.set("s", secs);
                summary.push_str(&tr_args("tui-col-bash-active-secs", &a));
            }
        }
    }
    if is_active {
        summary.push('…');
    }
    let summary_line = format!("{} {}", summary, tr("tui-expand-hint"));
    let (bullet_style, text_style) = assistant_tool_header_styles(!is_active, is_active, live);
    let mut out =
        wrap_plain_bullet_prefixed("⏺ ", bullet_style, summary_line.as_str(), text_style, w);

    let paths = collect_read_paths_from_blocks(blocks);
    if !paths.is_empty() {
        let take = paths.len().min(MAX_COLLAPSED_PATH_PREVIEWS);
        for p in paths.iter().take(take) {
            out.extend(wrap_plain_prefixed("   ⎿  ", p.as_str(), style_dim(), w));
        }
        if paths.len() > MAX_COLLAPSED_PATH_PREVIEWS {
            let extra = paths.len() - MAX_COLLAPSED_PATH_PREVIEWS;
            let mut a = FluentArgs::new();
            a.set("n", extra as i64);
            out.push(Line::from(Span::styled(
                tr_args("tui-read-more-paths", &a),
                style_dim(),
            )));
        }
    } else if is_active {
        if let Some(hint) = latest_display_hint_for_blocks(blocks) {
            out.extend(wrap_plain_prefixed("   ⎿  ", hint.as_str(), style_dim(), w));
        }
    }
    out
}

/// 将连续的可折叠工具块合并为一条 `CollapsedToolGroup`（对齐 Claude `collapseReadSearchGroups`）。
/// 在 `coalesce_read_tool_batches` 之后调用。`User` / `AssistantMarkdown` 等会打断合并。
pub(crate) fn collapse_tool_groups(entries: &mut Vec<TranscriptEntry>, next_fold_id: &mut u64) {
    let old = std::mem::take(entries);
    let mut i = 0usize;
    while i < old.len() {
        let e = &old[i];
        if matches!(e, TranscriptEntry::CollapsedToolGroup { .. }) {
            entries.push(e.clone());
            i += 1;
            continue;
        }
        if !transcript_entry_is_collapsible(e) {
            entries.push(e.clone());
            i += 1;
            continue;
        }
        let first = old[i].clone();
        let Some(first_block) = entry_into_collapsible_block(first) else {
            entries.push(old[i].clone());
            i += 1;
            continue;
        };
        let mut group = vec![first_block];
        i += 1;
        while i < old.len() {
            let next_e = &old[i];
            if matches!(next_e, TranscriptEntry::CollapsedToolGroup { .. }) {
                break;
            }
            if !transcript_entry_is_collapsible(next_e) {
                break;
            }
            let next = old[i].clone();
            let Some(b) = entry_into_collapsible_block(next) else {
                break;
            };
            group.push(b);
            i += 1;
        }
        if group.len() == 1 {
            entries.push(collapsible_block_into_entry(
                group.pop().expect("one block"),
            ));
        } else {
            *next_fold_id = next_fold_id.saturating_add(1);
            entries.push(TranscriptEntry::CollapsedToolGroup {
                fold_id: *next_fold_id,
                blocks: group,
            });
        }
    }
}

/// Claude Code Workspace：`ToolCall/Result` 归并 → `FileRead` 批处理 → 折叠摘要（单一路径，避免 sync/consume 分叉）。
pub(crate) fn apply_tool_transcript_pipeline(
    entries: &mut Vec<TranscriptEntry>,
    next_fold_id: &mut u64,
) {
    normalize_transcript_global(entries, next_fold_id);
    coalesce_read_tool_batches(entries, next_fold_id);
    collapse_tool_groups(entries, next_fold_id);
}

/// Ctrl+O：自底部起展开下一个折叠块；已全部展开则全部收起。
pub(crate) fn ctrl_o_fold_cycle(
    entries: &[TranscriptEntry],
    expanded: &mut std::collections::HashSet<u64>,
) {
    let ids: Vec<u64> = entries
        .iter()
        .filter_map(|e| match e {
            TranscriptEntry::ToolTurn { fold_id, .. } => Some(*fold_id),
            TranscriptEntry::ReadToolBatch { fold_id, .. } => Some(*fold_id),
            TranscriptEntry::CollapsedToolGroup { fold_id, .. } => Some(*fold_id),
            _ => None,
        })
        .collect();
    for id in ids.iter().rev() {
        if !expanded.contains(id) {
            expanded.insert(*id);
            return;
        }
    }
    for id in ids {
        expanded.remove(&id);
    }
}

/// Workspace 语义块：按终端宽度排版为物理行后再滚动（与 ratatui 自动换行脱钩）。
#[derive(Clone)]
pub(crate) enum TranscriptEntry {
    User(String),
    AssistantMarkdown(String),
    ToolCall {
        tool_use_id: String,
        name: String,
        args: String,
    },
    ToolResult {
        tool_use_id: String,
        /// 来自 `Message.metadata["tool_name"]`（若有则顶栏显示工具名而非冗长 id）。
        tool_name: Option<String>,
        body: String,
        is_error: bool,
    },
    /// 已归并的「调用 + 结果」，支持折叠展示（对齐 Claude Code）。
    ToolTurn {
        fold_id: u64,
        name: String,
        args: String,
        #[allow(dead_code)] // 折叠块与后续 tool_result 关联用，当前渲染未读
        tool_use_id: String,
        tool_name: Option<String>,
        body: String,
        is_error: bool,
    },
    /// 同一轮内多次 `FileRead` 合并展示（Claude：`Read N files (ctrl+o to expand)`）。
    ReadToolBatch {
        fold_id: u64,
        /// 每项：`args` JSON、`body` 原文、`is_error`
        parts: Vec<(String, String, bool)>,
    },
    /// 连续 Read/Grep/Glob/Bash 等合并为一条摘要（Claude：`collapsed_read_search`）。
    CollapsedToolGroup {
        fold_id: u64,
        blocks: Vec<CollapsibleToolBlock>,
    },
    Plain(Vec<Line<'static>>),
}

/// 将 Workspace 条目转为纯文本，供 **仅备用屏模式** 下退出后写入主缓冲（`ANYCODE_TUI_ALT_SCREEN=0` 时主缓冲原生滚动，无需 echo）。
pub(crate) fn transcript_dump_plain_text(entries: &[TranscriptEntry]) -> String {
    use std::fmt::Write;

    fn plain_lines(lines: &[Line<'static>]) -> String {
        let mut s = String::new();
        for line in lines {
            for span in &line.spans {
                let _ = write!(s, "{}", span.content);
            }
            let _ = writeln!(s);
        }
        s
    }

    fn dump_read_parts(out: &mut String, parts: &[(String, String, bool)]) {
        for (args, body, is_err) in parts {
            let _ = writeln!(out, "args:\n{args}");
            if *is_err {
                let _ = writeln!(out, "error:\n{body}");
            } else {
                let _ = writeln!(out, "{body}");
            }
            let _ = writeln!(out);
        }
    }

    fn dump_collapsible_block(out: &mut String, b: &CollapsibleToolBlock) {
        match b {
            CollapsibleToolBlock::Turn {
                name,
                args,
                body,
                is_error,
                tool_name,
                ..
            } => {
                let label = tool_name.as_deref().unwrap_or(name.as_str());
                let _ = writeln!(out, "[{label}]");
                let _ = writeln!(out, "{args}");
                if *is_error {
                    let _ = writeln!(out, "error:\n{body}");
                } else {
                    let _ = writeln!(out, "{body}");
                }
                let _ = writeln!(out);
            }
            CollapsibleToolBlock::ReadBatch { parts, .. } => dump_read_parts(out, parts),
        }
    }

    let mut out = String::new();
    let _ = writeln!(out, "── anyCode session ──");
    for e in entries {
        match e {
            TranscriptEntry::User(t) => {
                let _ = writeln!(out, "▸ user\n{}", t.trim_end());
                let _ = writeln!(out);
            }
            TranscriptEntry::AssistantMarkdown(t) => {
                let _ = writeln!(out, "▸ assistant\n{}", t.trim_end());
                let _ = writeln!(out);
            }
            TranscriptEntry::ToolCall {
                name,
                args,
                tool_use_id,
            } => {
                let _ = writeln!(out, "▸ tool call {name} (id {tool_use_id})\n{args}");
                let _ = writeln!(out);
            }
            TranscriptEntry::ToolResult {
                tool_name,
                tool_use_id,
                body,
                is_error,
            } => {
                let label = tool_name.as_deref().unwrap_or(tool_use_id.as_str());
                let _ = writeln!(out, "▸ tool result {label}");
                if *is_error {
                    let _ = writeln!(out, "error:\n{body}");
                } else {
                    let _ = writeln!(out, "{body}");
                }
                let _ = writeln!(out);
            }
            TranscriptEntry::ToolTurn {
                name,
                args,
                body,
                is_error,
                tool_name,
                ..
            } => {
                let label = tool_name.as_deref().unwrap_or(name.as_str());
                let _ = writeln!(out, "▸ tool {label}");
                let _ = writeln!(out, "{args}");
                if *is_error {
                    let _ = writeln!(out, "error:\n{body}");
                } else {
                    let _ = writeln!(out, "{body}");
                }
                let _ = writeln!(out);
            }
            TranscriptEntry::ReadToolBatch { parts, .. } => {
                let _ = writeln!(out, "▸ read batch ({} part(s))", parts.len());
                dump_read_parts(&mut out, parts);
            }
            TranscriptEntry::CollapsedToolGroup { blocks, .. } => {
                let _ = writeln!(out, "▸ collapsed tools");
                for b in blocks {
                    dump_collapsible_block(&mut out, b);
                }
            }
            TranscriptEntry::Plain(lines) => {
                let _ = write!(out, "{}", plain_lines(lines));
            }
        }
    }
    out
}

pub(crate) fn message_to_entries(msg: &Message) -> Vec<TranscriptEntry> {
    match msg.role {
        MessageRole::User => match &msg.content {
            MessageContent::Text(t) => vec![TranscriptEntry::User(t.trim_end().to_string())],
            _ => vec![TranscriptEntry::Plain(vec![Line::from(Span::styled(
                "> <non-text>",
                style_error(),
            ))])],
        },
        MessageRole::Assistant => {
            let mut out: Vec<TranscriptEntry> = vec![];
            let content_text = match &msg.content {
                MessageContent::Text(t) => t.clone(),
                _ => String::new(),
            };

            if let Some(raw) = msg.metadata.get(ANYCODE_TOOL_CALLS_METADATA_KEY) {
                if let Ok(calls) = serde_json::from_value::<Vec<ToolCall>>(raw.clone()) {
                    for c in calls {
                        let args_str = serde_json::to_string_pretty(&c.input)
                            .or_else(|_| serde_json::to_string(&c.input))
                            .unwrap_or_else(|_| "<unserializable>".to_string());
                        out.push(TranscriptEntry::ToolCall {
                            tool_use_id: c.id.clone(),
                            name: c.name.clone(),
                            args: args_str,
                        });
                    }
                }
            }

            if !content_text.trim().is_empty() {
                out.push(TranscriptEntry::AssistantMarkdown(content_text));
            } else if out.is_empty() {
                out.push(TranscriptEntry::Plain(vec![Line::from(Span::styled(
                    "⏺ <empty>",
                    style_dim(),
                ))]));
            }
            out
        }
        MessageRole::Tool => {
            if let MessageContent::ToolResult {
                tool_use_id,
                content,
                is_error,
            } = &msg.content
            {
                let tool_name = msg
                    .metadata
                    .get("tool_name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());
                vec![TranscriptEntry::ToolResult {
                    tool_use_id: tool_use_id.clone(),
                    tool_name,
                    body: content.clone(),
                    is_error: *is_error,
                }]
            } else {
                vec![TranscriptEntry::Plain(vec![Line::from(Span::styled(
                    "<unexpected tool message>",
                    style_error(),
                ))])]
            }
        }
        MessageRole::System => vec![],
    }
}

/// 时间上最后一条带非空正文的 assistant（跳过末尾空占位），供 turn 收尾与 runtime 返回值交叉校验。
pub(crate) fn last_nonempty_assistant_text(msgs: &[Message]) -> Option<String> {
    msgs.iter().rev().find_map(|m| {
        if m.role != MessageRole::Assistant {
            return None;
        }
        match &m.content {
            MessageContent::Text(t) => {
                let tr = t.trim();
                if tr.is_empty() {
                    None
                } else {
                    Some(tr.to_string())
                }
            }
            _ => None,
        }
    })
}

/// 自尾部向前跳过工具块与 Written：若已存在与 `body` 语义一致的 `AssistantMarkdown`，则无需再补「总体输出」。
pub(crate) fn transcript_tail_closing_matches(entries: &[TranscriptEntry], body: &str) -> bool {
    if body.trim().is_empty() {
        return true;
    }
    for e in entries.iter().rev() {
        match e {
            TranscriptEntry::AssistantMarkdown(s) => {
                return assistant_markdown_meaningful_eq(s, body);
            }
            TranscriptEntry::Plain(_) => continue,
            TranscriptEntry::CollapsedToolGroup { .. }
            | TranscriptEntry::ToolTurn { .. }
            | TranscriptEntry::ReadToolBatch { .. }
            | TranscriptEntry::ToolCall { .. }
            | TranscriptEntry::ToolResult { .. } => continue,
            TranscriptEntry::User(_) => break,
        }
    }
    false
}

/// `messages[exec_prev_len..]` → 截断尾部锚点后重放并跑工具流水线（TUI 实时同步专用）。
pub(crate) fn rebuild_live_turn_tail(
    transcript: &mut Vec<TranscriptEntry>,
    tail_start: usize,
    fold_id_base: u64,
    next_fold_id: &mut u64,
    messages: &[Message],
    exec_prev_len: usize,
) {
    let slice = messages.get(exec_prev_len..).unwrap_or(&[]);
    *next_fold_id = fold_id_base;
    transcript.truncate(tail_start);
    for m in slice {
        transcript.extend(message_to_entries(m));
    }
    apply_tool_transcript_pipeline(transcript, next_fold_id);
}

pub(crate) fn layout_workspace(
    entries: &[TranscriptEntry],
    content_width: usize,
    expanded_tool_folds: &std::collections::HashSet<u64>,
    live: WorkspaceLiveLayout,
) -> Vec<Line<'static>> {
    let w = content_width.max(8);
    let mut out: Vec<Line<'static>> = Vec::new();
    for (ei, e) in entries.iter().enumerate() {
        if ei > 0 && workspace_gap_before(&entries[ei - 1], e) {
            out.push(Line::from(""));
        }
        match e {
            TranscriptEntry::User(s) => {
                let t = s.trim_end();
                if t.is_empty() {
                    continue;
                }
                let mut md_lines =
                    render_markdown_styled(t, w, style_user().add_modifier(Modifier::BOLD));
                if let Some(first) = md_lines.first_mut() {
                    let mut spans = vec![Span::styled(
                        "> ",
                        style_user().add_modifier(Modifier::BOLD),
                    )];
                    spans.extend(first.spans.clone());
                    *first = Line::from(spans);
                }
                out.append(&mut md_lines);
                if should_show_turn_status_after_user(entries, ei, live) {
                    out.push(turn_status_line(live));
                }
            }
            TranscriptEntry::AssistantMarkdown(text) => {
                let unwrapped = unwrap_single_content_json(text);
                let t = unwrapped.trim_end();
                if t.is_empty() {
                    out.push(Line::from(Span::styled("⏺ <empty>", style_dim())));
                    continue;
                }
                let mut md_lines = render_markdown(t, w);
                if let Some(first) = md_lines.first_mut() {
                    let tail_reply = ei + 1 == entries.len();
                    let bullet = pulse_dim_assistant_bold(live, tail_reply);
                    let mut spans = vec![Span::styled("⏺ ", bullet)];
                    spans.extend(first.spans.clone());
                    *first = Line::from(spans);
                }
                out.append(&mut md_lines);
            }
            TranscriptEntry::ToolCall {
                tool_use_id,
                name,
                args,
            } => {
                let mut block: Vec<Line<'static>> = Vec::new();
                block.extend(wrap_plain_prefixed(
                    "⏵ ",
                    name.as_str(),
                    style_tool().add_modifier(Modifier::BOLD),
                    w,
                ));
                block.extend(wrap_plain_prefixed(
                    "   ",
                    &format!("id {tool_use_id}"),
                    style_dim(),
                    w,
                ));
                block.extend(wrap_plain_prefixed("   ", args.as_str(), style_dim(), w));
                push_lines_truncated(&mut out, block, MAX_TOOL_BLOCK_LINES);
            }
            TranscriptEntry::ToolTurn {
                fold_id,
                name,
                args,
                tool_use_id: _,
                tool_name,
                body,
                is_error,
            } => {
                let is_active = is_active_tool_fold(entries, *fold_id, live);
                let block = layout_tool_turn_block(
                    *fold_id,
                    name.as_str(),
                    args.as_str(),
                    tool_name.as_deref(),
                    body.as_str(),
                    *is_error,
                    expanded_tool_folds.contains(fold_id),
                    w,
                    is_active,
                    live,
                );
                push_lines_truncated(&mut out, block, MAX_TOOL_BLOCK_LINES);
            }
            TranscriptEntry::ReadToolBatch { fold_id, parts } => {
                let is_active = is_active_tool_fold(entries, *fold_id, live);
                let block = layout_read_tool_batch(
                    *fold_id,
                    parts.as_slice(),
                    expanded_tool_folds.contains(fold_id),
                    w,
                    is_active,
                    live,
                );
                push_lines_truncated(&mut out, block, MAX_TOOL_BLOCK_LINES);
            }
            TranscriptEntry::CollapsedToolGroup { fold_id, blocks } => {
                let expanded = expanded_tool_folds.contains(fold_id);
                let block = layout_collapsed_tool_group(
                    *fold_id,
                    blocks.as_slice(),
                    expanded,
                    w,
                    expanded_tool_folds,
                    entries,
                    live,
                );
                push_lines_truncated(&mut out, block, MAX_TOOL_BLOCK_LINES);
            }
            TranscriptEntry::ToolResult {
                tool_use_id,
                tool_name,
                body,
                is_error,
            } => {
                let block = layout_tool_result_block(
                    tool_name.as_deref(),
                    tool_use_id.as_str(),
                    body.as_str(),
                    *is_error,
                    w,
                );
                push_lines_truncated(&mut out, block, MAX_TOOL_BLOCK_LINES);
            }
            TranscriptEntry::Plain(lines) => {
                for line in lines {
                    out.extend(wrap_ratatui_line(line.clone(), w));
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod transcript_tests {
    use super::*;

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
    fn assistant_markdown_meaningful_eq_matches_content_json_wrapper() {
        let stored = "{\"content\":\"done\"}";
        assert!(assistant_markdown_meaningful_eq(stored, "done"));
        assert!(!assistant_markdown_meaningful_eq(stored, "other"));
    }

    #[test]
    fn assistant_markdown_meaningful_eq_unwraps_candidate_json() {
        let stored = "{\"content\":\"summary\"}";
        let candidate = "{\"content\":\"summary\"}";
        assert!(assistant_markdown_meaningful_eq(stored, candidate));
    }

    #[test]
    fn transcript_tail_closing_matches_skips_collapsed_tool_group() {
        let entries = vec![
            TranscriptEntry::AssistantMarkdown("hi".into()),
            TranscriptEntry::CollapsedToolGroup {
                fold_id: 1,
                blocks: vec![],
            },
        ];
        assert!(transcript_tail_closing_matches(&entries, "hi"));
        assert!(!transcript_tail_closing_matches(&entries, "bye"));
    }

    #[test]
    fn coalesce_merges_consecutive_file_read_turns() {
        let mut entries = vec![
            TranscriptEntry::ToolTurn {
                fold_id: 1,
                name: "FileRead".into(),
                args: r#"{"file_path":"/a"}"#.into(),
                tool_use_id: "u1".into(),
                tool_name: Some("FileRead".into()),
                body: "body1".into(),
                is_error: false,
            },
            TranscriptEntry::ToolTurn {
                fold_id: 2,
                name: "FileRead".into(),
                args: r#"{"file_path":"/b"}"#.into(),
                tool_use_id: "u2".into(),
                tool_name: Some("FileRead".into()),
                body: "body2".into(),
                is_error: false,
            },
        ];
        let mut next = 100u64;
        coalesce_read_tool_batches(&mut entries, &mut next);
        assert_eq!(entries.len(), 1);
        match &entries[0] {
            TranscriptEntry::ReadToolBatch { fold_id, parts } => {
                assert_eq!(*fold_id, 101);
                assert_eq!(parts.len(), 2);
                assert_eq!(parts[0].1, "body1");
                assert_eq!(parts[1].1, "body2");
            }
            _ => unreachable!("test: expected ReadToolBatch"),
        }
    }

    #[test]
    fn coalesce_single_file_read_stays_tool_turn() {
        let mut entries = vec![TranscriptEntry::ToolTurn {
            fold_id: 7,
            name: "FileRead".into(),
            args: "{}".into(),
            tool_use_id: "u1".into(),
            tool_name: None,
            body: "x".into(),
            is_error: false,
        }];
        let mut next = 100u64;
        coalesce_read_tool_batches(&mut entries, &mut next);
        assert_eq!(entries.len(), 1);
        match &entries[0] {
            TranscriptEntry::ToolTurn { fold_id, .. } => assert_eq!(*fold_id, 7),
            _ => unreachable!("test: expected ToolTurn"),
        }
    }

    #[test]
    fn coalesce_does_not_merge_file_read_separated_by_other_tool() {
        let mut entries = vec![
            TranscriptEntry::ToolTurn {
                fold_id: 1,
                name: "FileRead".into(),
                args: "{}".into(),
                tool_use_id: "u1".into(),
                tool_name: None,
                body: "a".into(),
                is_error: false,
            },
            TranscriptEntry::ToolTurn {
                fold_id: 2,
                name: "Bash".into(),
                args: "{}".into(),
                tool_use_id: "u2".into(),
                tool_name: None,
                body: "out".into(),
                is_error: false,
            },
            TranscriptEntry::ToolTurn {
                fold_id: 3,
                name: "FileRead".into(),
                args: "{}".into(),
                tool_use_id: "u3".into(),
                tool_name: None,
                body: "b".into(),
                is_error: false,
            },
        ];
        let mut next = 100u64;
        coalesce_read_tool_batches(&mut entries, &mut next);
        assert_eq!(entries.len(), 3);
    }

    #[test]
    fn collapse_merges_consecutive_bash_turns() {
        let mut entries = vec![
            TranscriptEntry::ToolTurn {
                fold_id: 1,
                name: "Bash".into(),
                args: r#"{"command":"ls"}"#.into(),
                tool_use_id: "a".into(),
                tool_name: None,
                body: "out".into(),
                is_error: false,
            },
            TranscriptEntry::ToolTurn {
                fold_id: 2,
                name: "Bash".into(),
                args: r#"{"command":"pwd"}"#.into(),
                tool_use_id: "b".into(),
                tool_name: None,
                body: "out2".into(),
                is_error: false,
            },
        ];
        let mut next = 200u64;
        collapse_tool_groups(&mut entries, &mut next);
        assert_eq!(entries.len(), 1);
        match &entries[0] {
            TranscriptEntry::CollapsedToolGroup { fold_id, blocks } => {
                assert_eq!(*fold_id, 201);
                assert_eq!(blocks.len(), 2);
            }
            _ => unreachable!("test: expected CollapsedToolGroup"),
        }
    }

    #[test]
    fn collapse_single_bash_stays_tool_turn() {
        let mut entries = vec![TranscriptEntry::ToolTurn {
            fold_id: 9,
            name: "Bash".into(),
            args: "{}".into(),
            tool_use_id: "a".into(),
            tool_name: None,
            body: "x".into(),
            is_error: false,
        }];
        let mut next = 300u64;
        collapse_tool_groups(&mut entries, &mut next);
        assert_eq!(entries.len(), 1);
        match &entries[0] {
            TranscriptEntry::ToolTurn { fold_id, .. } => assert_eq!(*fold_id, 9),
            _ => unreachable!("test: expected ToolTurn"),
        }
    }

    #[test]
    fn collapse_assistant_breaks_group() {
        let mut entries = vec![
            TranscriptEntry::ToolTurn {
                fold_id: 1,
                name: "Bash".into(),
                args: "{}".into(),
                tool_use_id: "a".into(),
                tool_name: None,
                body: "o1".into(),
                is_error: false,
            },
            TranscriptEntry::AssistantMarkdown("summary".into()),
            TranscriptEntry::ToolTurn {
                fold_id: 2,
                name: "Bash".into(),
                args: "{}".into(),
                tool_use_id: "b".into(),
                tool_name: None,
                body: "o2".into(),
                is_error: false,
            },
        ];
        let mut next = 400u64;
        collapse_tool_groups(&mut entries, &mut next);
        assert_eq!(entries.len(), 3);
        assert!(matches!(entries[0], TranscriptEntry::ToolTurn { .. }));
        assert!(matches!(entries[1], TranscriptEntry::AssistantMarkdown(_)));
        assert!(matches!(entries[2], TranscriptEntry::ToolTurn { .. }));
    }

    #[test]
    fn layout_collapsed_summary_running_when_executing() {
        let blocks = vec![
            CollapsibleToolBlock::Turn {
                fold_id: 1,
                name: "Bash".into(),
                args: r#"{"command":"npm test"}"#.into(),
                tool_use_id: "a".into(),
                tool_name: None,
                body: "".into(),
                is_error: false,
            },
            CollapsibleToolBlock::Turn {
                fold_id: 2,
                name: "Bash".into(),
                args: r#"{"command":"ls"}"#.into(),
                tool_use_id: "b".into(),
                tool_name: None,
                body: "".into(),
                is_error: false,
            },
        ];
        let entries = vec![TranscriptEntry::CollapsedToolGroup {
            fold_id: 42,
            blocks,
        }];
        let folds = std::collections::HashSet::new();
        let lines = layout_workspace(
            &entries,
            100,
            &folds,
            WorkspaceLiveLayout {
                executing: true,
                working_elapsed_secs: Some(3),
                ..Default::default()
            },
        );
        let joined: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
            .collect();
        assert!(
            joined.contains("Running") || joined.contains("正在执行"),
            "expected active bash phrasing, got {joined}"
        );
        assert!(
            joined.contains('3') && (joined.contains('s') || joined.contains('秒')),
            "expected elapsed seconds hint, got {joined}"
        );
        assert!(joined.contains('…'));
    }

    #[test]
    fn layout_collapsed_summary_ran_when_idle() {
        let blocks = vec![
            CollapsibleToolBlock::Turn {
                fold_id: 1,
                name: "Bash".into(),
                args: "{}".into(),
                tool_use_id: "a".into(),
                tool_name: None,
                body: "".into(),
                is_error: false,
            },
            CollapsibleToolBlock::Turn {
                fold_id: 2,
                name: "Bash".into(),
                args: "{}".into(),
                tool_use_id: "b".into(),
                tool_name: None,
                body: "".into(),
                is_error: false,
            },
        ];
        let entries = vec![TranscriptEntry::CollapsedToolGroup {
            fold_id: 42,
            blocks,
        }];
        let folds = std::collections::HashSet::new();
        let lines = layout_workspace(&entries, 100, &folds, WorkspaceLiveLayout::default());
        let joined: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
            .collect();
        assert!(
            joined.contains("Ran") || joined.contains("已执行"),
            "expected completed bash phrasing, got {joined}"
        );
        assert!(!joined.contains("Running") && !joined.contains("正在执行"));
    }
}
