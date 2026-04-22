//! 流式 REPL 终端层：**轴心线程** crossterm `poll`/`read` + ratatui。
//!
//! - **默认（备用屏全屏）**：DEC 备用屏 + `Terminal::new()`（见 [`crate::term::terminal_guard::stream_repl_use_alternate_screen`]）；**处理** `Event::Resize`，与 ratatui `draw` 内 `autoresize` 一致。
//! - **遗留主缓冲 Inline**：`ANYCODE_TERM_REPL_INLINE_LEGACY=1` 等关闭备用屏时，[`Viewport::Inline`] 高度在**启动时**按可视行数 × 55%；**会话内忽略** `Resize`。长文可经 [`StreamReplRenderMsg`] + [`crate::repl::stream_term::flush_stream_scrollback_staging`] **`insert_before`** 进宿主 scrollback。
//! - **不**捕鼠，滚轮交给终端（Inline 下尤甚）。
//!
//! 主区为应用内视口：长文用 **PgUp / PgDn** 分页，`Ctrl+Home` / `Ctrl+End` 跳到最旧/最新（见 [`crate::repl::stream_events::handle_event`]）。
//! 退出时：备用屏仍可按 `ANYCODE_TERM_EXIT_SCROLLBACK_DUMP` 把 transcript 打到 shell；**主缓冲 Inline** 下正文可能已在 `insert_before` 历史中，默认退出 dump 策略见 [`crate::repl::stream_term::shutdown_stream_terminal`]。

use std::io::Write;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossterm::cursor::Hide;
use crossterm::event::{self, EnableBracketedPaste, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{enable_raw_mode, EnterAlternateScreen};

use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::repl::stream_app::StreamReplUiSession;
use crate::repl::stream_paint::draw_stream_frame;
use crate::repl::stream_term::{
    flush_stream_scrollback_staging, new_stream_terminal, repl_event_debug_line,
    resume_terminal_after_subprocess, shutdown_stream_terminal, suspend_terminal_for_subprocess,
};
use crate::repl::{
    apply_stream_approval_key, apply_stream_user_question_key, drain_stream_repl_render_scrollback,
    handle_event, stream_repl_accept_key_event, ReplCtl, ReplLineState, StreamReplRenderMsg,
};
/// `poll` 之后单入口：先 `draw`（内含 `autoresize`、回写 `stream_viewport_width`）→ 再 `drain`/`insert_before`，
/// 避免 reflow 当帧仍用旧视口宽算增量却用新终端宽刷 scrollback。
fn paint_stream_frame(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    state: &Arc<Mutex<ReplLineState>>,
    render_rx: &Receiver<StreamReplRenderMsg>,
    scrollback_staging: &mut Vec<String>,
    use_alternate_screen: bool,
) -> anyhow::Result<()> {
    draw_stream_frame(terminal, state)?;
    drain_stream_repl_render_scrollback(render_rx, scrollback_staging);
    flush_stream_scrollback_staging(terminal, state, scrollback_staging, use_alternate_screen)?;
    Ok(())
}

/// Tokio 侧 → UI 线程：释放终端、结束循环等。
pub(crate) enum StreamReplAsyncCtl {
    Shutdown,
    SuspendForSubprocess(Sender<()>),
    ResumeAfterSubprocess(Sender<()>),
}

/// UI 线程 → Tokio：用户提交、Ctrl+L、协作取消、EOF。
pub(crate) enum StreamReplUiMsg {
    Submit(String),
    ClearSession,
    CooperativeCancelTurn,
    Eof,
}

/// 在当前线程运行：crossterm 输入 + ratatui 绘制（轴心）；每帧从 Tokio 队列 drain 审批/选题与回合摘要过期。
pub(crate) fn run_stream_repl_ui_thread(session: StreamReplUiSession) -> anyhow::Result<()> {
    let StreamReplUiSession {
        state,
        to_worker: to_async,
        ctrl_rx,
        render_rx,
        approval_rx,
        question_rx,
        repl_debug_events,
        use_alternate_screen,
    } = session;

    enable_raw_mode()?;
    let mut out = std::io::stdout();
    if use_alternate_screen {
        execute!(out, EnterAlternateScreen)?;
    }
    execute!(out, Hide, EnableBracketedPaste)?;

    let mut terminal: Option<Terminal<CrosstermBackend<std::io::Stdout>>> =
        Some(new_stream_terminal(use_alternate_screen)?);

    if let Some(ref t) = terminal {
        if let Ok(area) = t.size() {
            crate::repl::stream_paint::sync_stream_repl_viewport_from_area(&state, area);
        }
    }

    let mut exit = false;
    let mut scrollback_staging: Vec<String> = Vec::new();
    while !exit {
        crate::tasks::stream_repl_loop::tick_finished_turn_summary_expiry(&state);
        {
            let mut g = approval_rx.lock().unwrap_or_else(|e| e.into_inner());
            crate::tasks::stream_repl_loop::drain_pending_stream_approvals(g.as_mut(), &state);
        }
        {
            let mut g = question_rx.lock().unwrap_or_else(|e| e.into_inner());
            crate::tasks::stream_repl_loop::drain_pending_stream_user_questions(g.as_mut(), &state);
        }

        crate::tasks::stream_repl_loop::tick_executing_stream_transcript(&state);

        while let Ok(cmd) = ctrl_rx.try_recv() {
            match cmd {
                StreamReplAsyncCtl::Shutdown => {
                    exit = true;
                    break;
                }
                StreamReplAsyncCtl::SuspendForSubprocess(ack) => {
                    suspend_terminal_for_subprocess(&mut terminal, use_alternate_screen)?;
                    let _ = ack.send(());
                }
                StreamReplAsyncCtl::ResumeAfterSubprocess(ack) => {
                    resume_terminal_after_subprocess(&mut terminal, use_alternate_screen)?;
                    if let Some(t) = terminal.as_mut() {
                        if let Ok(area) = t.size() {
                            crate::repl::stream_paint::sync_stream_repl_viewport_from_area(
                                &state, area,
                            );
                        }
                        let _ = paint_stream_frame(
                            t,
                            &state,
                            &render_rx,
                            &mut scrollback_staging,
                            use_alternate_screen,
                        );
                    }
                    let _ = ack.send(());
                }
            }
        }
        if exit {
            break;
        }

        let Some(t) = terminal.as_mut() else {
            drain_stream_repl_render_scrollback(&render_rx, &mut scrollback_staging);
            std::thread::sleep(Duration::from_millis(16));
            continue;
        };

        if event::poll(Duration::from_millis(16))? {
            let mut batch: Vec<Event> = Vec::new();
            while event::poll(Duration::ZERO)? {
                batch.push(event::read()?);
            }
            'dispatch: for ev in batch {
                if repl_debug_events {
                    eprintln!("[repl-debug-events] {}", repl_event_debug_line(&ev));
                }
                // 主缓冲 Inline（遗留）：忽略 Resize，避免 Inline 顶屏叠 scrollback。备用屏全屏：交给 ratatui draw 内 autoresize。
                if !use_alternate_screen && matches!(&ev, Event::Resize(_, _)) {
                    continue;
                }
                let mut s = state.lock().unwrap_or_else(|e| e.into_inner());
                if s.pending_user_question.is_some() {
                    if let Event::Key(key) = &ev {
                        if key.kind == KeyEventKind::Release {
                            continue;
                        }
                        if !stream_repl_accept_key_event(key) {
                            continue;
                        }
                        apply_stream_user_question_key(&mut s, *key);
                    }
                    drop(s);
                    paint_stream_frame(
                        t,
                        &state,
                        &render_rx,
                        &mut scrollback_staging,
                        use_alternate_screen,
                    )?;
                    continue;
                }
                if s.pending_approval.is_some() {
                    if let Event::Key(key) = &ev {
                        if key.kind == KeyEventKind::Release {
                            continue;
                        }
                        if !stream_repl_accept_key_event(key) {
                            continue;
                        }
                        apply_stream_approval_key(&mut s, *key);
                    }
                    drop(s);
                    paint_stream_frame(
                        t,
                        &state,
                        &render_rx,
                        &mut scrollback_staging,
                        use_alternate_screen,
                    )?;
                    continue;
                }
                match handle_event(ev, &mut s)? {
                    ReplCtl::Continue => {}
                    ReplCtl::Submit(text) => {
                        drop(s);
                        let _ = to_async.send(StreamReplUiMsg::Submit(text));
                    }
                    ReplCtl::ClearSession => {
                        drop(s);
                        let _ = to_async.send(StreamReplUiMsg::ClearSession);
                    }
                    ReplCtl::CooperativeCancelTurn => {
                        drop(s);
                        let _ = to_async.send(StreamReplUiMsg::CooperativeCancelTurn);
                    }
                    ReplCtl::Eof => {
                        drop(s);
                        let _ = to_async.send(StreamReplUiMsg::Eof);
                        exit = true;
                        break 'dispatch;
                    }
                }
            }
        }

        if exit {
            break;
        }

        paint_stream_frame(
            t,
            &state,
            &render_rx,
            &mut scrollback_staging,
            use_alternate_screen,
        )?;
    }

    shutdown_stream_terminal(&state, &mut terminal, use_alternate_screen)?;
    Ok(())
}

#[cfg(test)]
mod inline_viewport_rows_tests {
    use crate::repl::stream_term::stream_repl_inline_viewport_rows;

    #[test]
    fn inline_rows_default_pct_not_full_screen() {
        assert_eq!(stream_repl_inline_viewport_rows(40), 22);
        assert_eq!(stream_repl_inline_viewport_rows(20), 11);
        assert_eq!(stream_repl_inline_viewport_rows(15), 10);
        assert_eq!(stream_repl_inline_viewport_rows(8), 8);
    }
}
