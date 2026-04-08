//! Workspace 折叠摘要与终端宽度下整块排版。

use crate::i18n::{tr, tr_args};
use crate::md_tui::{
    render_markdown, render_markdown_styled, wrap_plain_bullet_prefixed, wrap_plain_prefixed,
    wrap_ratatui_line,
};
use fluent_bundle::FluentArgs;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use serde_json::Value;
use std::path::Path;

use super::tool_render::{
    assistant_tool_header_styles, file_path_from_file_read_args, layout_read_tool_batch,
    layout_tool_result_block, layout_tool_turn_block, push_lines_truncated,
    unwrap_single_content_json, MAX_COLLAPSED_PATH_PREVIEWS, MAX_TOOL_BLOCK_LINES,
};
use super::types::{CollapsibleToolBlock, TranscriptEntry, WorkspaceLiveLayout};
use crate::tui::styles::*;

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
