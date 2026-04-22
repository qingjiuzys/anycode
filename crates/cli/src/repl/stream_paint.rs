//! 流式 REPL ratatui 单帧绘制（主区 + dock）。

use ratatui::backend::CrosstermBackend;
use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::Terminal;
use std::sync::{Arc, Mutex};

use crate::repl::stream_viewport::{prepare_stream_transcript_paragraph, render_stream_scrollbar};
use crate::repl::{render_repl_dock_to_buffer, repl_dock_height, ReplDockLayout, ReplLineState};

fn blit_dock_to_frame(dst: &mut Buffer, src: &Buffer, origin: Rect) {
    for y in 0..src.area.height {
        for x in 0..src.area.width {
            let cell = src.get(x, y).clone();
            *dst.get_mut(origin.x + x, origin.y + y) = cell;
        }
    }
}

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

/// 与 [`draw_stream_frame`] 相同的竖切 + 主区正文宽（scrollbar 列），保证 `ReplLineState` 与 ratatui 视口一致。
fn stream_frame_layout(st: &ReplLineState, area: Rect) -> (Rect, Rect, u16, bool) {
    let dock_h = repl_dock_height(area, st, ReplDockLayout);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(dock_h.min(area.height.saturating_sub(1)).max(1)),
        ])
        .split(area);
    let top_cell = chunks[0];
    let dock_screen = chunks[1];
    let full_w = top_cell.width.max(1);
    let (body_w, show_rail) = if full_w >= 2 {
        (full_w.saturating_sub(1), true)
    } else {
        (full_w, false)
    };
    (top_cell, dock_screen, body_w, show_rail)
}

/// 首帧 `paint` 之前、`tick_executing_stream_transcript` 已可能运行：用终端当前 `size()` 回写宽高，避免沿用默认 80×0。
pub(crate) fn sync_stream_repl_viewport_from_area(state: &Arc<Mutex<ReplLineState>>, area: Rect) {
    let Ok(mut st) = state.lock() else {
        return;
    };
    let (top_cell, _, body_w, _) = stream_frame_layout(&st, area);
    st.stream_transcript_viewport_h = top_cell.height.max(1);
    st.stream_viewport_width = body_w;
}

pub(crate) fn draw_stream_frame(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    state: &Arc<Mutex<ReplLineState>>,
) -> anyhow::Result<()> {
    terminal.draw(|f| {
        let area = f.size();
        let mut st = match state.lock() {
            Ok(s) => s,
            Err(_) => return,
        };
        clear_buffer_area(f.buffer_mut(), area);
        let (top_cell, dock_screen, body_w, show_rail) = stream_frame_layout(&st, area);
        st.stream_transcript_viewport_h = top_cell.height.max(1);
        st.stream_viewport_width = body_w;
        let transcript_raw = st
            .transcript
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let auto = st.stream_repl_auto_scroll_follow;
        let scroll_up = st.stream_transcript_scroll;
        let mut pos = st.stream_repl_scroll_pos;
        let mut tgt = st.stream_repl_scroll_target;
        let (top_par, _g_off, _max_sc, total_rows, at_bottom) = prepare_stream_transcript_paragraph(
            transcript_raw.as_str(),
            body_w,
            top_cell.height,
            &mut st.stream_transcript_layout,
            auto,
            scroll_up,
            &mut pos,
            &mut tgt,
        );
        st.stream_repl_scroll_pos = pos;
        st.stream_repl_scroll_target = tgt;
        st.stream_repl_auto_scroll_follow = at_bottom;

        if show_rail {
            let hchunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(top_cell);
            let body_rect = hchunks[0];
            let rail_rect = hchunks[1];
            f.render_widget(top_par, body_rect);
            render_stream_scrollbar(
                f.buffer_mut(),
                rail_rect,
                total_rows,
                top_cell.height as usize,
                pos,
            );
        } else {
            f.render_widget(top_par, top_cell);
        }

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
