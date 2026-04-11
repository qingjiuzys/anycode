//! agent turn 的 `JoinHandle` 完成后回填 transcript / 错误状态。

use crate::artifact_summary::claude_turn_written_lines;
use crate::i18n::{tr, tr_args};
use crate::tui::styles::*;
use crate::tui::transcript::{
    apply_tool_transcript_pipeline, last_nonempty_assistant_text, message_to_entries,
    rebuild_live_turn_tail, transcript_tail_closing_matches, TranscriptEntry,
};
use anycode_agent::AgentRuntime;
use anycode_core::{
    strip_llm_reasoning_xml_blocks, AgentType, Message, MessageContent, MessageRole, TurnOutput,
    Usage,
};
use fluent_bundle::FluentArgs;
use ratatui::{
    style::Modifier,
    text::{Line, Span},
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use uuid::Uuid;

/// `/compact` 完成后的后续动作（发送前自动压缩需在压缩后再追加用户消息并启动 turn）。
#[derive(Debug)]
pub(super) enum CompactFollowup {
    ManualSlash,
    AutoThenUserTurn { trimmed: String },
}

async fn apply_compact_success_to_workspace(
    new_msgs: Vec<Message>,
    usage: Usage,
    messages: &Arc<Mutex<Vec<Message>>>,
    transcript: &mut Vec<TranscriptEntry>,
    transcript_gen: &mut u64,
    next_tool_fold_id: &mut u64,
    tool_folds_expanded: &mut HashSet<u64>,
    fold_layout_rev: &mut u64,
    exec_live_tail: &mut Option<(usize, u64)>,
    exec_prev_len: &mut usize,
    last_max_input_tokens: &mut u32,
    last_turn_usage: &mut Option<Usage>,
    transcript_scroll_up: &mut usize,
    done_hint: String,
) {
    *last_max_input_tokens = usage.input_tokens;
    *last_turn_usage = Some(usage);
    let frozen = {
        let mut g = messages.lock().await;
        *g = new_msgs;
        *exec_prev_len = g.len();
        g.clone()
    };
    tool_folds_expanded.clear();
    *exec_live_tail = None;
    *fold_layout_rev = fold_layout_rev.wrapping_add(1);
    rebuild_transcript_from_messages(transcript, &frozen, transcript_gen, next_tool_fold_id);
    apply_tool_transcript_pipeline(transcript, next_tool_fold_id);
    transcript.push(TranscriptEntry::Plain(vec![Line::from(Span::styled(
        done_hint,
        style_dim(),
    ))]));
    *transcript_gen = transcript_gen.wrapping_add(1);
    *transcript_scroll_up = 0;
}

/// 追加用户行到 transcript / messages，并 `spawn` 本轮 agent turn（与 Enter 发送路径一致）。
pub(super) async fn append_user_line_and_spawn_turn(
    trimmed: &str,
    transcript: &mut Vec<TranscriptEntry>,
    transcript_gen: &mut u64,
    transcript_scroll_up: &mut usize,
    exec_live_tail: &mut Option<(usize, u64)>,
    next_tool_fold_id: &mut u64,
    exec_prev_len: &mut usize,
    runtime: &Arc<AgentRuntime>,
    agent_type: &AgentType,
    messages: &Arc<Mutex<Vec<Message>>>,
    working_dir_str: &str,
) -> JoinHandle<anyhow::Result<TurnOutput>> {
    transcript.push(TranscriptEntry::User(trimmed.to_string()));
    apply_tool_transcript_pipeline(transcript, next_tool_fold_id);
    *transcript_gen = transcript_gen.wrapping_add(1);
    *transcript_scroll_up = 0;
    *exec_live_tail = Some((transcript.len(), *next_tool_fold_id));

    let user_msg = Message {
        id: Uuid::new_v4(),
        role: MessageRole::User,
        content: MessageContent::Text(trimmed.to_string()),
        timestamp: chrono::Utc::now(),
        metadata: HashMap::new(),
    };
    {
        let mut g = messages.lock().await;
        g.push(user_msg);
        *exec_prev_len = g.len();
    }

    let task_id = Uuid::new_v4();
    let rt = runtime.clone();
    let at = agent_type.clone();
    let wd = working_dir_str.to_string();
    let msgs = messages.clone();

    tokio::spawn(async move {
        rt.execute_turn_from_messages(task_id, &at, msgs, &wd)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    })
}

/// 等待后台 `compact_session_messages` 结束并更新 UI；若 `AutoThenUserTurn` 则继续 spawn 用户回合。
pub(super) async fn consume_finished_compact(
    handle: JoinHandle<anyhow::Result<(Vec<Message>, Usage)>>,
    followup: CompactFollowup,
    messages: &Arc<Mutex<Vec<Message>>>,
    transcript: &mut Vec<TranscriptEntry>,
    transcript_gen: &mut u64,
    next_tool_fold_id: &mut u64,
    tool_folds_expanded: &mut HashSet<u64>,
    fold_layout_rev: &mut u64,
    exec_live_tail: &mut Option<(usize, u64)>,
    exec_prev_len: &mut usize,
    last_turn_error: &mut Option<String>,
    last_max_input_tokens: &mut u32,
    last_turn_usage: &mut Option<Usage>,
    transcript_scroll_up: &mut usize,
    runtime: &Arc<AgentRuntime>,
    agent_type: &AgentType,
    working_dir_str: &str,
) -> Option<JoinHandle<anyhow::Result<TurnOutput>>> {
    let err_key = match &followup {
        CompactFollowup::ManualSlash => "tui-err-compact-failed",
        CompactFollowup::AutoThenUserTurn { .. } => "tui-err-autocompact-failed",
    };

    let join_res = handle.await;
    let inner = match join_res {
        Ok(v) => v,
        Err(e) => {
            *last_turn_error = Some(e.to_string());
            return None;
        }
    };

    match inner {
        Ok((new_msgs, usage)) => {
            let done_hint = match &followup {
                CompactFollowup::ManualSlash => tr("tui-compact-done"),
                CompactFollowup::AutoThenUserTurn { .. } => tr("tui-auto-compact-done"),
            };
            apply_compact_success_to_workspace(
                new_msgs,
                usage,
                messages,
                transcript,
                transcript_gen,
                next_tool_fold_id,
                tool_folds_expanded,
                fold_layout_rev,
                exec_live_tail,
                exec_prev_len,
                last_max_input_tokens,
                last_turn_usage,
                transcript_scroll_up,
                done_hint,
            )
            .await;
            *last_turn_error = None;
            match followup {
                CompactFollowup::ManualSlash => None,
                CompactFollowup::AutoThenUserTurn { trimmed } => Some(
                    append_user_line_and_spawn_turn(
                        &trimmed,
                        transcript,
                        transcript_gen,
                        transcript_scroll_up,
                        exec_live_tail,
                        next_tool_fold_id,
                        exec_prev_len,
                        runtime,
                        agent_type,
                        messages,
                        working_dir_str,
                    )
                    .await,
                ),
            }
        }
        Err(e) => {
            let mut a = FluentArgs::new();
            a.set("err", e.to_string());
            *last_turn_error = Some(tr_args(err_key, &a));
            None
        }
    }
}

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
    handle: JoinHandle<anyhow::Result<TurnOutput>>,
    messages: &Arc<Mutex<Vec<Message>>>,
    exec_prev_len: usize,
    transcript: &mut Vec<TranscriptEntry>,
    transcript_gen: &mut u64,
    next_tool_fold_id: &mut u64,
    last_turn_error: &mut Option<String>,
    last_max_input_tokens: &mut u32,
    last_turn_usage: &mut Option<Usage>,
    // live_anchor: (transcript_tail_start, fold_id_base)，与 sync_transcript 一致；完成时先截断再正式回填。
    live_anchor: Option<(usize, u64)>,
) {
    let result = handle.await;
    if let Some((tail_start, fold_base)) = live_anchor {
        *next_tool_fold_id = fold_base;
        transcript.truncate(tail_start);
    }
    match result {
        Ok(Ok(out)) => {
            let TurnOutput {
                final_text,
                artifacts,
                usage,
            } = out;
            *last_max_input_tokens = usage.max_input_tokens;
            *last_turn_usage = Some(usage.to_usage());
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
                let ft = strip_llm_reasoning_xml_blocks(final_text.trim_end());
                if !ft.trim().is_empty() {
                    ft
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
