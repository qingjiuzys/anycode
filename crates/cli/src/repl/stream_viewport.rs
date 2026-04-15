//! 流式 Inline REPL 主区视口：对齐 `claude-code-rust` `src/ui/chat.rs` 的思路——
//! 按视口宽度折行、前缀和定位全局滚动、`Paragraph::scroll` 视口裁剪、可选平滑滚动与右侧滚动条。

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Text};
use ratatui::widgets::Paragraph;

use crate::md_tui::wrap_string_to_width;
use crate::repl::inline::{
    sanitize_stream_transcript_visual_noise, scrub_stream_transcript_llm_raw_dumps,
    stream_transcript_line_style, StreamTranscriptLayoutCache,
};
use crate::tui::palette;

const SCROLL_EASE: f32 = 0.3;
const OVERSCROLL_CLAMP_EASE: f32 = 0.2;
const SCROLL_EPS: f32 = 0.01;

/// 与主区绘制一致：清洗后按 `width` 折行的总行数（用于 Inline 视口行数估算）。
pub(crate) fn stream_transcript_total_rows(raw: &str, width: u16) -> usize {
    let scrubbed = scrub_stream_transcript_llm_raw_dumps(raw);
    let cleaned = sanitize_stream_transcript_visual_noise(&scrubbed);
    let w = width.max(1) as usize;
    if cleaned.trim().is_empty() {
        return 0;
    }
    cleaned
        .lines()
        .map(|line| wrap_string_to_width(line, w).len().max(1))
        .sum()
}

fn hash_raw(raw: &str, width: u16) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    raw.hash(&mut h);
    width.hash(&mut h);
    h.finish()
}

/// 清洗后的 transcript：按 `wrap_string_to_width` 与渲染路径一致地统计行高与前缀和。
pub(crate) fn update_stream_layout_cache(
    cleaned: &str,
    width: u16,
    raw_key: u64,
    cache: &mut StreamTranscriptLayoutCache,
) {
    if cache.key == raw_key && cache.width == width {
        return;
    }
    let w = width.max(1) as usize;
    cache.key = raw_key;
    cache.width = width;
    cache.logical_heights.clear();
    cache.prefix_row.clear();
    let mut total = 0usize;
    for line in cleaned.lines() {
        cache.prefix_row.push(total);
        let h = wrap_string_to_width(line, w).len().max(1);
        cache.logical_heights.push(h);
        total = total.saturating_add(h);
    }
    cache.total_rows = total;
}

/// 将清洗后正文展开为「每条显示行一种样式」的 `Text`（与折行算法一致）。
pub(crate) fn stream_cleaned_to_wrapped_styled_text(cleaned: &str, width: u16) -> Text<'static> {
    let w = width.max(1) as usize;
    let mut lines: Vec<Line<'static>> = Vec::new();
    for line in cleaned.lines() {
        let t = line.trim_start();
        let style = stream_transcript_line_style(t, line);
        for row in wrap_string_to_width(line, w) {
            lines.push(Line::from(ratatui::text::Span::styled(row, style)));
        }
    }
    Text::from(lines)
}

fn reduced_motion_effective() -> bool {
    std::env::var_os("NO_COLOR").is_some()
}

/// 计算全局滚动偏移（从内容顶部起的行下标）、本地 `Paragraph::scroll` 偏移与是否贴底。
pub(crate) fn compute_stream_scroll(
    total_rows: usize,
    viewport_h: usize,
    auto_scroll: bool,
    scroll_up: usize,
    scroll_pos: &mut f32,
    scroll_target: &mut f32,
) -> (usize, usize, usize, bool) {
    let vh = viewport_h.max(1);
    let max_scroll = total_rows.saturating_sub(vh);
    let rm = reduced_motion_effective();
    let smooth = std::env::var("ANYCODE_STREAM_SMOOTH_SCROLL")
        .map(|v| {
            let v = v.trim().to_ascii_lowercase();
            v != "0" && v != "false" && v != "off"
        })
        .unwrap_or(true)
        && !rm;

    if auto_scroll {
        *scroll_target = max_scroll as f32;
        *scroll_pos = *scroll_target;
    } else {
        let up = scroll_up.min(max_scroll);
        let desired_first = max_scroll.saturating_sub(up);
        *scroll_target = desired_first as f32;
        let target = *scroll_target;
        let delta = target - *scroll_pos;
        if !smooth || rm || delta.abs() < SCROLL_EPS {
            *scroll_pos = target;
        } else {
            *scroll_pos += delta * SCROLL_EASE;
        }
        let max_f = max_scroll as f32;
        if *scroll_pos > max_f {
            let overshoot = *scroll_pos - max_f;
            *scroll_pos = max_f + overshoot * OVERSCROLL_CLAMP_EASE;
            if (*scroll_pos - max_f).abs() < SCROLL_EPS {
                *scroll_pos = max_f;
            }
        }
    }

    let mut global_off = scroll_pos.round() as usize;
    global_off = global_off.min(max_scroll);
    let at_bottom = (max_scroll > 0 && global_off >= max_scroll) || total_rows <= vh;

    let local = global_off;
    (global_off, local, max_scroll, at_bottom)
}

#[must_use]
pub(crate) fn paragraph_scroll_y(local: usize) -> u16 {
    u16::try_from(local).unwrap_or(u16::MAX)
}

/// 右侧滚动条（与 `claude-code-rust` `render_scrollbar_overlay` 同款字符）。
pub(crate) fn render_stream_scrollbar(
    buf: &mut Buffer,
    area: Rect,
    content_height: usize,
    viewport_height: usize,
    scroll_pos: f32,
) {
    if area.width == 0 || area.height == 0 || content_height <= viewport_height {
        return;
    }
    let vh = viewport_height.max(1);
    let max_scroll = content_height.saturating_sub(vh);
    if max_scroll == 0 {
        return;
    }
    let track = vh;
    let thumb = ((vh * vh) / content_height).max(1).min(track);
    let max_thumb_top = track.saturating_sub(thumb);
    let t = (scroll_pos / max_scroll as f32).clamp(0.0, 1.0);
    let thumb_top = (t * max_thumb_top as f32).round() as usize;
    let thumb_end = thumb_top.saturating_add(thumb).min(track);

    let rail_style = Style::default().add_modifier(Modifier::DIM);
    let thumb_style = Style::default().fg(palette::assistant_label());

    let rail_x = area.right().saturating_sub(1);
    for row in 0..area.height as usize {
        let y = area.y.saturating_add(row as u16);
        if rail_x < buf.area.width && y < buf.area.height {
            let cell = buf.get_mut(rail_x, y);
            cell.set_symbol("\u{2595}");
            cell.set_style(rail_style);
        }
    }
    for row in thumb_top..thumb_end {
        let y = area.y.saturating_add(row as u16);
        if y < area.y.saturating_add(area.height) && rail_x < buf.area.width && y < buf.area.height
        {
            let cell = buf.get_mut(rail_x, y);
            cell.set_symbol("\u{2590}");
            cell.set_style(thumb_style);
        }
    }
}

/// 准备主区绘制：返回 `Paragraph`（全量折行文本 + wrap）、全局滚动偏移、是否贴底。
pub(crate) fn prepare_stream_transcript_paragraph(
    raw: &str,
    viewport_w: u16,
    viewport_h: u16,
    layout_cache: &mut StreamTranscriptLayoutCache,
    auto_scroll: bool,
    scroll_up: usize,
    scroll_pos: &mut f32,
    scroll_target: &mut f32,
) -> (Paragraph<'static>, usize, usize, usize, bool) {
    let scrubbed = scrub_stream_transcript_llm_raw_dumps(raw);
    let cleaned = sanitize_stream_transcript_visual_noise(&scrubbed);
    let key = hash_raw(&scrubbed, viewport_w);
    update_stream_layout_cache(&cleaned, viewport_w, key, layout_cache);

    let total_rows = layout_cache.total_rows;
    let vh = viewport_h.max(1) as usize;

    let (global_off, local_scroll, max_scroll, at_bottom) = compute_stream_scroll(
        total_rows,
        vh,
        auto_scroll,
        scroll_up,
        scroll_pos,
        scroll_target,
    );

    let text = if cleaned.trim().is_empty() {
        Text::raw(String::new())
    } else {
        stream_cleaned_to_wrapped_styled_text(&cleaned, viewport_w)
    };
    // 每行已按 `wrap_string_to_width` 预折到 `viewport_w`；勿再 `.wrap()`，否则 ratatui `WordWrapper`
    // 会与行高缓存不一致。`Paragraph` 无 wrap 时按行截断，单行宽度 ≤ 视口故不会二次断行。
    let para = Paragraph::new(text).scroll((paragraph_scroll_y(local_scroll), 0));

    (para, global_off, max_scroll, total_rows, at_bottom)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_rows_counts_wrapped_lines() {
        let raw = "ab\n".repeat(5);
        assert_eq!(stream_transcript_total_rows(&raw, 1), 10);
    }

    #[test]
    fn auto_scroll_pins_to_bottom() {
        let mut pos = 0.0f32;
        let mut tgt = 0.0f32;
        let (_g, local, max_sc, at_bottom) =
            compute_stream_scroll(20, 5, true, 0, &mut pos, &mut tgt);
        assert_eq!(max_sc, 15);
        assert_eq!(local, 15);
        assert!(at_bottom);
    }

    #[test]
    fn manual_scroll_respects_scroll_up() {
        let mut pos = 15.0f32;
        let mut tgt = 15.0f32;
        let (_g, local, _, _) = compute_stream_scroll(20, 5, false, 3, &mut pos, &mut tgt);
        assert_eq!(local, 12);
    }
}
