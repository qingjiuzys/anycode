//! Stream REPL：执行态 `transcript` 刷新（[`tick_executing_stream_transcript`]，**仅轴心线程**）、scrollback 通道；
//! **审批/选题 drain** 与 **回合摘要过期** 亦在轴心 [`crate::repl::run_stream_repl_ui_thread`] 每帧调用。
//! 主 `select!`、斜杠分发与回合 join 在 [`crate::tasks::tasks_repl::stream_repl_tokio_worker`]（`current_thread` 运行时线程）。

use std::sync::{Arc, Mutex};
use std::time::Instant;

use tokio::sync::mpsc;

use crate::repl::{ReplLineState, StreamReplRenderMsg};
use crate::term::transcript::build_stream_turn_plain;
use crate::term::{ApprovalDecision, PendingApproval, PendingUserQuestion};

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

/// 执行中：按 `messages` 重建主区 `transcript`（与视口一致）。**仅在轴心线程**调用（与 `poll`/`draw` 同线程）；
/// 依赖 `ReplLineState::stream_exec_*`（由 Tokio worker 在 `append_user_spawn_turn` 成功后写入）。
///
/// **主缓冲 + 宿主 scrollback**：执行中**不向** `StreamReplRenderMsg::ScrollbackChunk` 推内容；
/// 回合结束后由 `tasks_repl` join 路径一次性写入本回合 `transcript[anchor..]`。
pub(crate) fn tick_executing_stream_transcript(state: &Arc<Mutex<ReplLineState>>) {
    let (msgs, exec_prev, anchor) = {
        let Ok(st) = state.lock() else {
            return;
        };
        let Some(m) = st.stream_exec_messages.as_ref() else {
            return;
        };
        (
            Arc::clone(m),
            st.stream_exec_prev_len,
            st.stream_exec_transcript_anchor,
        )
    };
    let Ok(guard) = msgs.try_lock() else {
        return;
    };
    let w = state
        .lock()
        .map(|s| s.stream_viewport_width.max(40))
        .unwrap_or(80) as usize;
    let plain =
        normalize_stream_plain_for_transcript(build_stream_turn_plain(exec_prev, &guard, w, true));
    drop(guard);
    if let Ok(st) = state.lock() {
        if let Ok(mut t) = st.transcript.lock() {
            t.truncate(anchor);
            t.push_str(&plain);
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
