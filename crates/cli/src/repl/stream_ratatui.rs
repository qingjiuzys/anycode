//! 流式 REPL 终端层：**同线程** crossterm `poll`/`read` + ratatui。
//!
//! - **默认（主缓冲，跟随终端但非整屏）**：[`Viewport::Inline`] 高度为**当前终端可视行数 × 55%**（下限 10 行，极矮终端铺满），**不占满整屏**；行数随窗口/字体变化时重算并重建视口，即**跟随终端尺寸**。长文在区内滚动。
//! - **可选整屏**：`ANYCODE_STREAM_REPL_ALT_SCREEN=1` 或与全屏 TUI 对齐的全局备用屏开关 → DEC 备用屏 + `Terminal::new()`（见 [`crate::tui::run::terminal_guard::stream_repl_use_alternate_screen`]）。
//! - **长文**：Tokio 经 [`crate::repl::StreamReplRenderMsg`] 将增量送入本线程，[`drain_stream_repl_render_scrollback`] + [`crate::repl::stream_term::flush_stream_scrollback_staging`] 用 `Terminal::insert_before` 推入宿主 **scrollback**；主区同步 `transcript`。**不**捕鼠，滚轮交给终端。
//!
//! Inline 高度**不**随正文变长而增大（避免反复 `Terminal::with_options` → scrollback 叠层）；仅在**终端行数变化**导致目标高度变化时重建终端（宽度变化由 `draw` 内 `autoresize` 处理）。
//! ratatui 0.24 下 Inline 的 `resize`/`autoresize` 会用换行「顶屏」，拖拽窗口时若每帧都绘制会把视口内容反复顶入宿主 scrollback；故主缓冲下对终端尺寸做 **150ms 防抖**，爆发式 Resize 期间跳过绘制，静稳后再 `paint`（与全屏 TUI 同源 [`crate::resize_debounce`]）。
//!
//! 主区为应用内视口：长文用 **PgUp / PgDn** 按**当前主区高度**分页（随终端缩放），`Ctrl+Home` / `Ctrl+End` 跳到最旧/最新（见 [`crate::repl::stream_events::handle_event`]）。
//! 退出时：备用屏仍可按 `ANYCODE_STREAM_EXIT_SCROLLBACK_DUMP` 把 transcript 打到 shell；**主缓冲**下正文已在 `insert_before` 历史中，默认**不再**整段 dump（避免与 scrollback 叠重复）；需要时显式 `ANYCODE_STREAM_EXIT_SCROLLBACK_DUMP=full`。

use std::io::Write;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossterm::cursor::Hide;
use crossterm::event::{self, EnableBracketedPaste, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{enable_raw_mode, size as terminal_size, EnterAlternateScreen};

use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::repl::stream_paint::draw_stream_frame;
use crate::repl::stream_term::{
    flush_stream_scrollback_staging, new_stream_terminal, repl_event_debug_line,
    resume_terminal_after_subprocess, shutdown_stream_terminal, stream_repl_inline_viewport_rows,
    suspend_terminal_for_subprocess,
};
use crate::repl::{
    apply_stream_approval_key, apply_stream_user_question_key, drain_stream_repl_render_scrollback,
    handle_event, stream_repl_accept_key_event, ReplCtl, ReplLineState, StreamReplRenderMsg,
};
use crate::resize_debounce::ResizeDebounce;

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

/// 主缓冲 Inline：防抖 + 视口行数变化时重建终端，再 [`paint_stream_frame`]；备用屏无 Inline 顶屏问题，始终绘制。
fn paint_stream_repl_maybe_skipped(
    resize_debounce: &mut ResizeDebounce,
    inline_viewport_rows: &mut Option<u16>,
    use_alternate_screen: bool,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    state: &Arc<Mutex<ReplLineState>>,
    render_rx: &Receiver<StreamReplRenderMsg>,
    scrollback_staging: &mut Vec<String>,
) -> anyhow::Result<()> {
    if !use_alternate_screen {
        if let Ok(term_dims) = terminal_size() {
            if resize_debounce.update(term_dims) {
                return Ok(());
            }
            let target = stream_repl_inline_viewport_rows(term_dims.1);
            if *inline_viewport_rows != Some(target) {
                if let Ok(nt) = new_stream_terminal(false) {
                    *terminal = nt;
                    *inline_viewport_rows = Some(target);
                }
            }
        }
    }
    paint_stream_frame(
        terminal,
        state,
        render_rx,
        scrollback_staging,
        use_alternate_screen,
    )
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

/// 在专用线程中运行：crossterm 输入 + ratatui 绘制（与 TUI 一致栈）。
pub(crate) fn run_stream_repl_ui_thread(
    state: Arc<Mutex<ReplLineState>>,
    to_async: tokio::sync::mpsc::UnboundedSender<StreamReplUiMsg>,
    ctrl_rx: Receiver<StreamReplAsyncCtl>,
    render_rx: Receiver<StreamReplRenderMsg>,
    repl_debug_events: bool,
    use_alternate_screen: bool,
) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut out = std::io::stdout();
    if use_alternate_screen {
        execute!(out, EnterAlternateScreen)?;
    }
    execute!(out, Hide, EnableBracketedPaste)?;

    let (_, init_h) = terminal_size()?;
    let mut inline_viewport_rows: Option<u16> = if use_alternate_screen {
        None
    } else {
        Some(stream_repl_inline_viewport_rows(init_h))
    };

    let mut terminal: Option<Terminal<CrosstermBackend<std::io::Stdout>>> =
        Some(new_stream_terminal(use_alternate_screen)?);

    let mut exit = false;
    let mut scrollback_staging: Vec<String> = Vec::new();
    let mut resize_debounce = ResizeDebounce::new();
    while !exit {
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
                        let _ = paint_stream_repl_maybe_skipped(
                            &mut resize_debounce,
                            &mut inline_viewport_rows,
                            use_alternate_screen,
                            t,
                            &state,
                            &render_rx,
                            &mut scrollback_staging,
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
                    paint_stream_repl_maybe_skipped(
                        &mut resize_debounce,
                        &mut inline_viewport_rows,
                        use_alternate_screen,
                        t,
                        &state,
                        &render_rx,
                        &mut scrollback_staging,
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
                    paint_stream_repl_maybe_skipped(
                        &mut resize_debounce,
                        &mut inline_viewport_rows,
                        use_alternate_screen,
                        t,
                        &state,
                        &render_rx,
                        &mut scrollback_staging,
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

        paint_stream_repl_maybe_skipped(
            &mut resize_debounce,
            &mut inline_viewport_rows,
            use_alternate_screen,
            t,
            &state,
            &render_rx,
            &mut scrollback_staging,
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
