//! ToolCall/ToolResult 归并、FileRead 批处理、可折叠工具组与 Ctrl+O。

use super::types::{CollapsibleToolBlock, TranscriptEntry};

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

/// Ctrl+O：工具块展示开关。
/// - 当前存在任一展开块：全部收起（再次按键可快速回到紧凑视图）
/// - 当前无展开块：自底部起展开一个折叠块
pub(crate) fn ctrl_o_fold_cycle(
    entries: &[TranscriptEntry],
    expanded: &mut std::collections::HashSet<u64>,
) {
    if !expanded.is_empty() {
        expanded.clear();
        return;
    }

    let ids: Vec<u64> = entries
        .iter()
        .filter_map(|e| match e {
            TranscriptEntry::ToolTurn { fold_id, .. } => Some(*fold_id),
            TranscriptEntry::ReadToolBatch { fold_id, .. } => Some(*fold_id),
            TranscriptEntry::CollapsedToolGroup { fold_id, .. } => Some(*fold_id),
            _ => None,
        })
        .collect();
    if let Some(id) = ids.iter().rev().next() {
        expanded.insert(*id);
    }
}
