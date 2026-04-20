//! Stream REPL Tokio 侧循环：审批/选题 drain、执行态 transcript 同步、scrollback 通道。
//! 主 `select!` 与斜杠分发仍在 [`crate::tasks::tasks_repl::run_interactive_tty_stream`]。

use std::sync::{Arc, Mutex};
use std::time::Instant;

use tokio::sync::mpsc;

use crate::repl::{ReplLineState, StreamReplRenderMsg};
use crate::tui::transcript::build_stream_turn_plain;
use crate::tui::{ApprovalDecision, PendingApproval, PendingUserQuestion};

use super::repl_line_session::ReplLineSession;

/// 执行态每 tick 重写 transcript，去掉尾部多余 `\n`，避免与主区 padding 叠出「空行带」。
pub(crate) fn normalize_stream_plain_for_transcript(s: String) -> String {
    let t = s.trim_end_matches(['\n', '\r']);
    if t.is_empty() {
        String::new()
    } else {
        format!("{t}\n")
    }
}

/// 顶栏「回合结束摘要」计时到期后清理。
pub(crate) fn tick_finished_turn_summary_expiry(state: &Arc<Mutex<ReplLineState>>) {
    if let Ok(mut st) = state.lock() {
        if let Some(until) = st.finished_turn_summary_until {
            if Instant::now() >= until {
                st.finished_turn_summary_until = None;
                st.finished_turn_summary = None;
            }
        }
    }
}

/// 审批队列 `try_recv` 合入 `ReplLineState::pending_approval`（新项顶替旧项并 deny 旧 reply）。
pub(crate) fn drain_pending_stream_approvals(
    approval_rx: Option<&mut mpsc::Receiver<PendingApproval>>,
    state: &Arc<Mutex<ReplLineState>>,
) {
    let Some(rx) = approval_rx else {
        return;
    };
    loop {
        match rx.try_recv() {
            Ok(r) => {
                let mut st = state.lock().unwrap_or_else(|e| e.into_inner());
                st.approval_menu_selected = 0;
                if let Some(old) = st.pending_approval.replace(r) {
                    let _ = old.reply.send(ApprovalDecision::Deny);
                }
            }
            Err(mpsc::error::TryRecvError::Empty) => break,
            Err(mpsc::error::TryRecvError::Disconnected) => break,
        }
    }
}

/// 用户选题队列 `try_recv` 合入 `pending_user_question`。
pub(crate) fn drain_pending_stream_user_questions(
    question_rx: Option<&mut mpsc::Receiver<PendingUserQuestion>>,
    state: &Arc<Mutex<ReplLineState>>,
) {
    let Some(qrx) = question_rx else {
        return;
    };
    loop {
        match qrx.try_recv() {
            Ok(r) => {
                let mut st = state.lock().unwrap_or_else(|e| e.into_inner());
                st.user_question_menu_selected = 0;
                if let Some(old) = st.pending_user_question.replace(r) {
                    let _ = old.reply.send(Err(()));
                }
            }
            Err(mpsc::error::TryRecvError::Empty) => break,
            Err(mpsc::error::TryRecvError::Disconnected) => break,
        }
    }
}

/// 执行中：按 `messages` 重建主区 `transcript`（与视口一致）。
///
/// **主缓冲 + 宿主 scrollback**：执行中**不向** `StreamReplRenderMsg::ScrollbackChunk` 推内容。
/// ratatui 0.24 的 `Terminal::insert_before` 每次会先 `clear()` 再 `append_lines`，高频增量会把视口正文
/// 反复顶进宿主历史，表现为整段/标题叠行；回合结束后由 `tasks_repl` 一次性写入本回合
/// `transcript[turn_transcript_anchor..]`。
pub(crate) fn tick_executing_stream_transcript(
    line_session: &ReplLineSession,
    state: &Arc<Mutex<ReplLineState>>,
    use_repl_alt: bool,
    exec_prev_len: usize,
    turn_transcript_anchor: usize,
    live_scroll_echo_len: &mut usize,
) {
    if let Ok(guard) = line_session.messages.try_lock() {
        let w = state
            .lock()
            .map(|s| s.stream_viewport_width.max(40))
            .unwrap_or(80) as usize;
        let plain = normalize_stream_plain_for_transcript(build_stream_turn_plain(
            exec_prev_len,
            &guard,
            w,
            true,
        ));
        drop(guard);
        if use_repl_alt {
            if let Ok(st) = state.lock() {
                if let Ok(mut t) = st.transcript.lock() {
                    t.truncate(turn_transcript_anchor);
                    t.push_str(&plain);
                }
            }
            *live_scroll_echo_len = plain.len();
        } else {
            if let Ok(st) = state.lock() {
                if let Ok(mut t) = st.transcript.lock() {
                    t.truncate(turn_transcript_anchor);
                    t.push_str(&plain);
                }
            }
            *live_scroll_echo_len = plain.len();
        }
    }
}

/// 主缓冲下将增量交给 UI 线程（[`crate::repl::drain_stream_repl_render_scrollback`] 消费）。
#[inline]
pub(crate) fn send_scrollback_chunk(
    render_tx: &std::sync::mpsc::Sender<StreamReplRenderMsg>,
    chunk: String,
) {
    if chunk.is_empty() {
        return;
    }
    let _ = render_tx.send(StreamReplRenderMsg::ScrollbackChunk(chunk));
}

/// `/clear` 等与宿主 scrollback 队列对齐的清空信号。
#[inline]
pub(crate) fn clear_scrollback_queue(render_tx: &std::sync::mpsc::Sender<StreamReplRenderMsg>) {
    let _ = render_tx.send(StreamReplRenderMsg::ClearScrollback);
}
