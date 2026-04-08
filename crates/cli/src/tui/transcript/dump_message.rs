//! 消息 → transcript 条目、纯文本导出、尾部匹配与 live tail 回放。

use anycode_core::{
    Message, MessageContent, MessageRole, ToolCall, ANYCODE_TOOL_CALLS_METADATA_KEY,
};
use ratatui::text::{Line, Span};

use super::pipeline::apply_tool_transcript_pipeline;
use super::tool_render::assistant_markdown_meaningful_eq;
use super::types::{CollapsibleToolBlock, TranscriptEntry};
use crate::tui::styles::*;

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
