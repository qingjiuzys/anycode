//! agent turn 的 `JoinHandle` 完成后回填 transcript / 错误状态。

use crate::artifact_summary::claude_turn_written_lines;
use crate::tui::styles::*;
use crate::tui::transcript::{
    apply_tool_transcript_pipeline, last_nonempty_assistant_text, message_to_entries,
    rebuild_live_turn_tail, transcript_tail_closing_matches, TranscriptEntry,
};
use anycode_core::{Artifact, Message};
use ratatui::{
    style::Modifier,
    text::{Line, Span},
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

/// Turn 进行中：用 `messages[exec_prev_len..]` 重放本回合尾部，Workspace 随 LLM/工具逐步更新。
pub(super) fn sync_transcript_with_messages_tail(
    transcript: &mut Vec<TranscriptEntry>,
    tail_start: usize,
    fold_id_base: u64,
    next_tool_fold_id: &mut u64,
    messages: &[Message],
    exec_prev_len: usize,
) {
    rebuild_live_turn_tail(
        transcript,
        tail_start,
        fold_id_base,
        next_tool_fold_id,
        messages,
        exec_prev_len,
    );
}

/// 压缩或 /clear 后：按完整 `messages` 重建 Workspace 条目。
pub(super) fn rebuild_transcript_from_messages(
    transcript: &mut Vec<TranscriptEntry>,
    messages: &[Message],
    transcript_gen: &mut u64,
    next_tool_fold_id: &mut u64,
) {
    transcript.clear();
    for m in messages {
        transcript.extend(message_to_entries(m));
    }
    *next_tool_fold_id = 0;
    *transcript_gen = transcript_gen.wrapping_add(1);
}

pub(super) async fn consume_finished_turn(
    handle: JoinHandle<anyhow::Result<(String, Vec<Artifact>, u32)>>,
    messages: &Arc<Mutex<Vec<Message>>>,
    exec_prev_len: usize,
    transcript: &mut Vec<TranscriptEntry>,
    transcript_gen: &mut u64,
    next_tool_fold_id: &mut u64,
    last_turn_error: &mut Option<String>,
    last_max_input_tokens: &mut u32,
    // live_anchor: (transcript_tail_start, fold_id_base)，与 sync_transcript 一致；完成时先截断再正式回填。
    live_anchor: Option<(usize, u64)>,
) {
    let result = handle.await;
    if let Some((tail_start, fold_base)) = live_anchor {
        *next_tool_fold_id = fold_base;
        transcript.truncate(tail_start);
    }
    match result {
        Ok(Ok((final_text, artifacts, max_in))) => {
            *last_max_input_tokens = max_in;
            let new_msgs = {
                let g = messages.lock().await;
                g.get(exec_prev_len..)
                    .map(|s| s.to_vec())
                    .unwrap_or_default()
            };
            for m in &new_msgs {
                transcript.extend(message_to_entries(m));
            }
            *transcript_gen = transcript_gen.wrapping_add(1);
            let written = claude_turn_written_lines(&artifacts);
            if !written.is_empty() {
                // 对齐 Claude Code：仅展示落盘产物；Bash 已在上方 `⏺ Bash(…)` 工具行中。
                let mut art_lines: Vec<Line<'static>> = vec![Line::from(vec![
                    Span::styled("⏺ ", style_tool()),
                    Span::styled("Written", style_tool().add_modifier(Modifier::BOLD)),
                ])];
                for label in written {
                    art_lines.push(Line::from(vec![
                        Span::styled("  · ", style_dim()),
                        Span::styled(label, style_assistant().add_modifier(Modifier::BOLD)),
                    ]));
                }
                transcript.push(TranscriptEntry::Plain(art_lines));
                *transcript_gen = transcript_gen.wrapping_add(1);
            }
            apply_tool_transcript_pipeline(transcript, next_tool_fold_id);

            // 收尾：优先 runtime 返回的 `final_text`（末条 assistant 常为空占位，仅用 messages 会误取上一条 thinking，导致不补总结）。
            // `final_text` 为空时再回退到本回合最后非空 assistant 正文。
            let closing = {
                let ft = final_text.trim_end();
                if !ft.is_empty() {
                    ft.to_string()
                } else {
                    last_nonempty_assistant_text(&new_msgs).unwrap_or_default()
                }
            };
            let ft = closing.trim();
            if !ft.is_empty() && !transcript_tail_closing_matches(transcript, ft) {
                transcript.push(TranscriptEntry::AssistantMarkdown(
                    closing.trim_end().to_string(),
                ));
                *transcript_gen = transcript_gen.wrapping_add(1);
            }
        }
        Ok(Err(e)) => {
            *last_turn_error = Some(e.to_string());
            transcript.push(TranscriptEntry::Plain(vec![Line::from(Span::styled(
                format!("Turn failed: {e}"),
                style_error(),
            ))]));
            *transcript_gen = transcript_gen.wrapping_add(1);
        }
        Err(e) => {
            *last_turn_error = Some(e.to_string());
            transcript.push(TranscriptEntry::Plain(vec![Line::from(Span::styled(
                format!("Turn join error: {e}"),
                style_error(),
            ))]));
            *transcript_gen = transcript_gen.wrapping_add(1);
        }
    }
}
