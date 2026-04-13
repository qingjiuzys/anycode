//! 流式 REPL 终端层：ratatui [`Viewport::Inline`] + **同线程** crossterm `poll`/`read`，
//! 与全屏 TUI 主缓冲模式一致（见 `tui::run::loop_inner`），避免 DECSTBM + 手工 blit + 独立读线程。
//!
//! 主区为应用内视口：长文用 **PgUp / PgDn** 滚动，`Ctrl+Home` / `Ctrl+End` 跳到最旧/最新（见 `handle_event`）。
//! 宿主终端 scrollback 仍看不到矩阵内正文；退出时会刷出 transcript 以保留会话。

use std::io::{stdout, Write};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crossterm::cursor::Hide;
use crossterm::event::{
    self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyEventKind, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, size as terminal_size};

use ratatui::backend::CrosstermBackend;
use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::Text;
use ratatui::widgets::Paragraph;
use ratatui::{Terminal, TerminalOptions, Viewport};

use crate::md_tui::wrap_string_to_width;
use crate::repl_inline::{
    apply_stream_approval_key, handle_event, render_repl_dock_to_buffer, repl_dock_height,
    repl_stream_transcript_bottom_padded, sanitize_stream_transcript_visual_noise,
    scrub_stream_transcript_llm_raw_dumps, stream_repl_accept_key_event, ReplCtl, ReplDockLayout,
    ReplLineState,
};
use crate::tui::styles::style_dim;

/// Tokio 侧 → UI 线程：释放终端、结束循环等。
pub(crate) enum StreamReplAsyncCtl {
    Shutdown,
    SuspendForSubprocess(Sender<()>),
    ResumeAfterSubprocess(Sender<()>),
}

/// UI 线程 → Tokio：用户提交、Ctrl+L、EOF。
pub(crate) enum StreamReplUiMsg {
    Submit(String),
    ClearSession,
    Eof,
}

fn repl_event_debug_line(ev: &Event) -> String {
    match ev {
        Event::Paste(s) => format!("Paste(chars={}, redacted)", s.chars().count()),
        Event::Key(k) => format!(
            "Key(code={:?}, kind={:?}, mods={:?})",
            k.code, k.kind, k.modifiers
        ),
        Event::Mouse(m) => format!("Mouse({m:?})"),
        Event::Resize(w, h) => format!("Resize({w},{h})"),
        _ => format!("{ev:?}"),
    }
}

fn blit_dock_to_frame(dst: &mut Buffer, src: &Buffer, origin: Rect) {
    for y in 0..src.area.height {
        for x in 0..src.area.width {
            let cell = src.get(x, y).clone();
            *dst.get_mut(origin.x + x, origin.y + y) = cell;
        }
    }
}

/// Inline 视口主区若不先清空，未改写的 cell 会残留上一帧（易与底栏 `─` 叠成「多一条横线」）。
fn clear_buffer_area(buf: &mut Buffer, area: Rect) {
    let y1 = area.y.min(buf.area.height);
    let y2 = area.y.saturating_add(area.height).min(buf.area.height);
    let x1 = area.x.min(buf.area.width);
    let x2 = area.x.saturating_add(area.width).min(buf.area.width);
    for y in y1..y2 {
        for x in x1..x2 {
            *buf.get_mut(x, y) = Cell::default();
        }
    }
}

fn draw_stream_frame(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    state: &Arc<Mutex<ReplLineState>>,
) -> anyhow::Result<()> {
    terminal.draw(|f| {
        let area = f.size();
        let mut st = match state.lock() {
            Ok(s) => s,
            Err(_) => return,
        };
        let dock_h = repl_dock_height(area, &st, ReplDockLayout);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(dock_h.min(area.height.saturating_sub(1)).max(1)),
            ])
            .split(area);
        let top_cell = chunks[0];
        let dock_screen = chunks[1];

        st.stream_viewport_width = top_cell.width.max(1);
        let wrap_w = top_cell.width.max(1);
        let scroll_up = st.stream_transcript_scroll;
        let transcript_tail = {
            let g = st.transcript.lock().unwrap_or_else(|e| e.into_inner());
            repl_stream_transcript_bottom_padded(g.as_str(), top_cell.height, wrap_w, scroll_up)
        };
        clear_buffer_area(f.buffer_mut(), top_cell);
        // 换行已由 `repl_stream_transcript_bottom_padded` 按 `wrap_w` 折好；勿再 `wrap`，否则与按显示行「上滚」的裁剪不一致。
        let top_par = Paragraph::new(Text::raw(transcript_tail)).style(style_dim());
        f.render_widget(top_par, top_cell);

        let iw = dock_screen.width.max(1);
        let ih = dock_screen.height.max(1);
        let dock_buf_rect = Rect::new(0, 0, iw, ih);
        let mut dock_buf = Buffer::empty(dock_buf_rect);
        let cursor_rel =
            render_repl_dock_to_buffer(&mut dock_buf, dock_buf_rect, &st, ReplDockLayout);
        blit_dock_to_frame(f.buffer_mut(), &dock_buf, dock_screen);

        if let Some((rx, ry)) = cursor_rel {
            let xa = dock_screen.x.saturating_add(rx.min(iw.saturating_sub(1)));
            let ya = dock_screen.y.saturating_add(ry.min(ih.saturating_sub(1)));
            f.set_cursor(xa, ya);
        }
    })?;
    Ok(())
}

fn transcript_display_rows(raw: &str, width: u16) -> usize {
    let w = width.max(8) as usize;
    let cleaned =
        sanitize_stream_transcript_visual_noise(&scrub_stream_transcript_llm_raw_dumps(raw));
    cleaned
        .lines()
        .map(|line| wrap_string_to_width(line, w).len().max(1))
        .sum()
}

fn desired_inline_rows(state: &Arc<Mutex<ReplLineState>>) -> anyhow::Result<u16> {
    let (w, h) = terminal_size()?;
    let area = Rect::new(0, 0, w.max(1), h.max(1));
    let st = state.lock().unwrap_or_else(|e| e.into_inner());
    let dock_h = repl_dock_height(area, &st, ReplDockLayout);
    let transcript = st
        .transcript
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let rows = transcript_display_rows(transcript.as_str(), w.max(1));

    let term_h = u32::from(h.max(1));
    let cap_total = (term_h.saturating_mul(65) / 100)
        .max(u32::from(dock_h.saturating_add(1)))
        .min(term_h) as u16;
    let content_extra = (rows as u16).saturating_add(1);
    let desired = dock_h.saturating_add(content_extra).min(cap_total);

    if transcript.trim().is_empty() {
        // 保留 1 行正文区，避免 dock 被压得过扁。
        Ok(dock_h.saturating_add(1).max(1).min(h.max(1)))
    } else {
        Ok(desired.max(dock_h.saturating_add(1)).min(h.max(1)))
    }
}

fn recreate_inline_terminal(
    rows: u16,
) -> anyhow::Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    let backend = CrosstermBackend::new(stdout());
    Ok(Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Inline(rows.max(1)),
        },
    )?)
}

/// 释放 ratatui 终端与子进程前的 raw/粘贴状态（不追加换行）。
fn suspend_terminal_for_subprocess(
    term: &mut Option<Terminal<CrosstermBackend<std::io::Stdout>>>,
) -> anyhow::Result<()> {
    if let Some(mut t) = term.take() {
        disable_raw_mode()?;
        let _ = execute!(stdout(), DisableBracketedPaste);
        let _ = t.show_cursor();
        drop(t);
    }
    Ok(())
}

/// 子进程结束后恢复 Inline 终端与 raw 模式。
fn resume_terminal_after_subprocess(
    state: &Arc<Mutex<ReplLineState>>,
    term: &mut Option<Terminal<CrosstermBackend<std::io::Stdout>>>,
) -> anyhow::Result<()> {
    let rows = desired_inline_rows(state)?;
    let t = recreate_inline_terminal(rows)?;
    enable_raw_mode()?;
    let _ = execute!(stdout(), Hide, EnableBracketedPaste);
    *term = Some(t);
    Ok(())
}

fn transcript_is_empty(state: &Arc<Mutex<ReplLineState>>) -> bool {
    let st = state.lock().unwrap_or_else(|e| e.into_inner());
    let t = st.transcript.lock().unwrap_or_else(|e| e.into_inner());
    t.trim().is_empty()
}

/// 退出 Inline 流式 REPL 时回打 scrollback 的策略（`ANYCODE_STREAM_EXIT_SCROLLBACK_DUMP`）。
#[derive(Clone, Copy, PartialEq, Eq)]
enum StreamExitScrollbackDump {
    /// 不打印。
    None,
    /// 打印完整 `transcript`（默认）。
    Full,
    /// 仅打印「当前自然语言轮」起：自上次 spawn turn 时 `transcript.len()` 起的后缀，减轻与视口重复。
    Anchor,
}

fn stream_exit_scrollback_dump_mode() -> StreamExitScrollbackDump {
    match std::env::var("ANYCODE_STREAM_EXIT_SCROLLBACK_DUMP") {
        Err(_) => StreamExitScrollbackDump::Full,
        Ok(v) => {
            let v = v.trim().to_ascii_lowercase();
            if v.is_empty() || v == "1" || v == "true" || v == "yes" || v == "on" || v == "full" {
                StreamExitScrollbackDump::Full
            } else if v == "0" || v == "false" || v == "no" || v == "off" {
                StreamExitScrollbackDump::None
            } else if v == "anchor" {
                StreamExitScrollbackDump::Anchor
            } else {
                StreamExitScrollbackDump::Full
            }
        }
    }
}

fn shutdown_stream_terminal(
    state: &Arc<Mutex<ReplLineState>>,
    term: &mut Option<Terminal<CrosstermBackend<std::io::Stdout>>>,
) -> anyhow::Result<()> {
    if let Some(mut t) = term.take() {
        disable_raw_mode()?;
        let _ = execute!(stdout(), DisableBracketedPaste);
        let _ = t.show_cursor();
        drop(t);
        let mut o = stdout();
        let _ = writeln!(o);
        let mode = stream_exit_scrollback_dump_mode();
        let (transcript, anchor) = {
            let st = state.lock().unwrap_or_else(|e| e.into_inner());
            let t = st.transcript.lock().unwrap_or_else(|e| e.into_inner());
            (t.clone(), st.stream_exit_dump_anchor)
        };
        match mode {
            StreamExitScrollbackDump::None => {}
            StreamExitScrollbackDump::Full => {
                if !transcript.trim().is_empty() {
                    let _ = writeln!(o, "{transcript}");
                }
            }
            StreamExitScrollbackDump::Anchor => {
                let slice = transcript.get(anchor..).unwrap_or("");
                if !slice.trim().is_empty() {
                    let _ = writeln!(o, "{slice}");
                }
            }
        }
    }
    Ok(())
}

/// 在专用线程中运行：crossterm 输入 + ratatui 绘制（与 TUI 一致栈）。
pub(crate) fn run_stream_repl_ui_thread(
    state: Arc<Mutex<ReplLineState>>,
    to_async: tokio::sync::mpsc::UnboundedSender<StreamReplUiMsg>,
    ctrl_rx: Receiver<StreamReplAsyncCtl>,
    repl_debug_events: bool,
) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let _ = execute!(stdout(), Hide, EnableBracketedPaste);

    let mut current_rows = desired_inline_rows(&state)?;
    let mut last_recreate = Instant::now();
    let mut terminal: Option<Terminal<CrosstermBackend<std::io::Stdout>>> =
        Some(recreate_inline_terminal(current_rows)?);

    let mut exit = false;
    while !exit {
        while let Ok(cmd) = ctrl_rx.try_recv() {
            match cmd {
                StreamReplAsyncCtl::Shutdown => {
                    exit = true;
                    break;
                }
                StreamReplAsyncCtl::SuspendForSubprocess(ack) => {
                    suspend_terminal_for_subprocess(&mut terminal)?;
                    let _ = ack.send(());
                }
                StreamReplAsyncCtl::ResumeAfterSubprocess(ack) => {
                    resume_terminal_after_subprocess(&state, &mut terminal)?;
                    if let Some(t) = terminal.as_mut() {
                        let st = state.lock().unwrap_or_else(|e| e.into_inner());
                        let _ = draw_stream_frame(t, &state);
                        drop(st);
                    }
                    let _ = ack.send(());
                }
            }
        }
        if exit {
            break;
        }

        let Some(t) = terminal.as_mut() else {
            std::thread::sleep(Duration::from_millis(16));
            continue;
        };

        if event::poll(Duration::from_millis(16))? {
            while event::poll(Duration::ZERO)? {
                let ev = event::read()?;
                if repl_debug_events {
                    eprintln!("[repl-debug-events] {}", repl_event_debug_line(&ev));
                }
                if let Event::Resize(_, _) = ev {
                    match desired_inline_rows(&state).and_then(|rows| {
                        current_rows = rows;
                        last_recreate = Instant::now();
                        recreate_inline_terminal(rows)
                    }) {
                        Ok(new_t) => {
                            *t = new_t;
                        }
                        Err(_) => {
                            let _ = t.autoresize();
                        }
                    }
                }
                let mut s = state.lock().unwrap_or_else(|e| e.into_inner());
                if let Event::Mouse(me) = &ev {
                    match me.kind {
                        MouseEventKind::ScrollUp => {
                            s.stream_transcript_scroll =
                                s.stream_transcript_scroll.saturating_add(4);
                            drop(s);
                            draw_stream_frame(t, &state)?;
                            continue;
                        }
                        MouseEventKind::ScrollDown => {
                            s.stream_transcript_scroll =
                                s.stream_transcript_scroll.saturating_sub(4);
                            drop(s);
                            draw_stream_frame(t, &state)?;
                            continue;
                        }
                        _ => {}
                    }
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
                    draw_stream_frame(t, &state)?;
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
                    ReplCtl::Eof => {
                        drop(s);
                        let _ = to_async.send(StreamReplUiMsg::Eof);
                        exit = true;
                        break;
                    }
                }
            }
        }

        if exit {
            break;
        }

        if let Ok(rows) = desired_inline_rows(&state) {
            let mut target = current_rows;
            if rows > current_rows {
                target = rows;
            } else if rows < current_rows && transcript_is_empty(&state) {
                // 仅在会话清空后允许收缩，避免生成中抖动。
                target = rows;
            }
            if target != current_rows && last_recreate.elapsed() >= Duration::from_millis(120) {
                if let Ok(new_t) = recreate_inline_terminal(target) {
                    *t = new_t;
                    current_rows = target;
                    last_recreate = Instant::now();
                }
            }
        }

        draw_stream_frame(t, &state)?;
    }

    shutdown_stream_terminal(&state, &mut terminal)?;
    Ok(())
}
