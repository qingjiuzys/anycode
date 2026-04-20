//! 流式 REPL：终端生命周期、Inline 行数、退出时 scrollback 回打。

use std::io::{stdout, Write};
use std::sync::{Arc, Mutex};

use crossterm::cursor::Hide;
use crossterm::event::Event;
use crossterm::event::{DisableBracketedPaste, EnableBracketedPaste};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, size as terminal_size, EnterAlternateScreen,
    LeaveAlternateScreen,
};

use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::text::Text;
use ratatui::widgets::{Paragraph, Widget, Wrap};
use ratatui::{Terminal, TerminalOptions, Viewport};
use unicode_width::UnicodeWidthStr;

use crate::repl::line_state::ReplLineState;

pub(crate) fn repl_event_debug_line(ev: &Event) -> String {
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

/// 主缓冲：`Viewport::Inline(h)` 的 **h** 按终端可视行数比例计算（默认 **55%**），非整屏。
pub(crate) fn stream_repl_inline_viewport_rows(term_h: u16) -> u16 {
    let h = u32::from(term_h.max(1));
    let pct = u32::from(stream_repl_inline_height_pct());
    ((h * pct / 100).max(10).min(h)) as u16
}

fn stream_repl_inline_height_pct() -> u16 {
    match std::env::var("ANYCODE_STREAM_REPL_INLINE_PCT") {
        Ok(s) => s
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|&p| (30..=90).contains(&p))
            .unwrap_or(55),
        Err(_) => 55,
    }
}

pub(crate) fn new_stream_terminal(
    use_alternate_screen: bool,
) -> anyhow::Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    let backend = CrosstermBackend::new(stdout());
    if use_alternate_screen {
        Ok(Terminal::new(backend)?)
    } else {
        let (_w, h) = terminal_size()?;
        let rows = stream_repl_inline_viewport_rows(h);
        Ok(Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Inline(rows.max(1)),
            },
        )?)
    }
}

pub(crate) fn suspend_terminal_for_subprocess(
    term: &mut Option<Terminal<CrosstermBackend<std::io::Stdout>>>,
    use_alternate_screen: bool,
) -> anyhow::Result<()> {
    if let Some(mut t) = term.take() {
        disable_raw_mode()?;
        let _ = execute!(stdout(), DisableBracketedPaste);
        let _ = t.show_cursor();
        drop(t);
        if use_alternate_screen {
            let _ = execute!(stdout(), LeaveAlternateScreen);
        }
    }
    Ok(())
}

pub(crate) fn resume_terminal_after_subprocess(
    term: &mut Option<Terminal<CrosstermBackend<std::io::Stdout>>>,
    use_alternate_screen: bool,
) -> anyhow::Result<()> {
    if use_alternate_screen {
        execute!(stdout(), EnterAlternateScreen)?;
    }
    enable_raw_mode()?;
    let _ = execute!(stdout(), Hide, EnableBracketedPaste);
    *term = Some(new_stream_terminal(use_alternate_screen)?);
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StreamExitScrollbackDump {
    None,
    Full,
    Anchor,
}

fn parse_exit_scrollback_dump_var(v: &str) -> StreamExitScrollbackDump {
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

fn stream_exit_scrollback_dump_mode() -> StreamExitScrollbackDump {
    match std::env::var("ANYCODE_STREAM_EXIT_SCROLLBACK_DUMP") {
        Err(_) => StreamExitScrollbackDump::Full,
        Ok(v) => parse_exit_scrollback_dump_var(&v),
    }
}

/// 退出时是否把 `transcript` 再 `writeln` 到 shell。
///
/// **主缓冲 + `insert_before` 路径**：执行中正文已进入宿主 scrollback，若仍默认 `Full` 会在退出时再打一遍整段 transcript → **重复块**。故未设环境变量时默认 **`None`**；需要留底时显式设 `ANYCODE_STREAM_EXIT_SCROLLBACK_DUMP=full` / `=anchor`。
fn effective_exit_scrollback_dump(
    state: &Arc<Mutex<ReplLineState>>,
    use_alternate_screen: bool,
) -> StreamExitScrollbackDump {
    let host_scrollback = state
        .lock()
        .map(|s| s.stream_repl_host_scrollback)
        .unwrap_or(false);
    if !use_alternate_screen && host_scrollback {
        return match std::env::var("ANYCODE_STREAM_EXIT_SCROLLBACK_DUMP") {
            Err(_) => StreamExitScrollbackDump::None,
            Ok(v) => parse_exit_scrollback_dump_var(&v),
        };
    }
    stream_exit_scrollback_dump_mode()
}

/// 估计 `Terminal::insert_before` 所需高度（按列宽折行；空串为 **0**）。
fn scrollback_wrap_rows(text: &str, width: u16) -> u16 {
    let w = usize::from(width.max(1));
    let mut rows: usize = 0;
    for line in text.split('\n') {
        let dw = line.width();
        rows += if dw == 0 { 1 } else { dw.div_ceil(w) };
    }
    if rows == 0 {
        return 0;
    }
    u16::try_from(rows).unwrap_or(u16::MAX)
}

/// 将 `staging` 中已排队的正文刷入**宿主** scrollback（仅主缓冲 Inline；与 ratatui `draw` 同线程）。
/// 调用方应先 [`crate::repl::drain_stream_repl_render_scrollback`] 将 Tokio 侧 `StreamReplRenderMsg` 收入 `staging`。
pub(crate) fn flush_stream_scrollback_staging(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    state: &Arc<Mutex<ReplLineState>>,
    staging: &mut Vec<String>,
    use_alternate_screen: bool,
) -> anyhow::Result<()> {
    if use_alternate_screen {
        staging.clear();
        return Ok(());
    }
    if staging.is_empty() {
        return Ok(());
    }
    let blob = std::mem::take(staging).concat();
    if blob.is_empty() {
        return Ok(());
    }
    // 与当前终端尺寸对齐（勿仅用 `ReplLineState::stream_viewport_width`）：resize 连发时该字段可能仍
    // 是上一帧 body 宽，`insert_before` 折行行数与 ratatui 视口错位易导致终端上「叠字/重复块」。
    let sz = terminal.size().unwrap_or_else(|_| {
        let w = state
            .lock()
            .map(|s| s.stream_viewport_width.max(40))
            .unwrap_or(80);
        Rect::new(0, 0, w, 24)
    });
    let width = sz.width.max(1);
    let term_h = sz.height.max(1);
    let rows = scrollback_wrap_rows(&blob, width).max(1).min(term_h);
    terminal.insert_before(rows, move |buf| {
        Paragraph::new(Text::raw(blob))
            .wrap(Wrap { trim: true })
            .render(buf.area, buf);
    })?;
    Ok(())
}

pub(crate) fn shutdown_stream_terminal(
    state: &Arc<Mutex<ReplLineState>>,
    term: &mut Option<Terminal<CrosstermBackend<std::io::Stdout>>>,
    use_alternate_screen: bool,
) -> anyhow::Result<()> {
    if let Some(mut t) = term.take() {
        disable_raw_mode()?;
        let _ = execute!(stdout(), DisableBracketedPaste);
        let _ = t.show_cursor();
        drop(t);
    }
    let mut o = stdout();
    if use_alternate_screen {
        let _ = execute!(o, LeaveAlternateScreen);
    }
    let _ = writeln!(o);
    let mode = effective_exit_scrollback_dump(state, use_alternate_screen);
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
    Ok(())
}
