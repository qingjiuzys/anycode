//! 流式 Inline REPL 主区视口：对齐 `claude-code-rust` `src/ui/chat.rs` 的思路——
//! 按视口宽度折行、前缀和定位全局滚动、`Paragraph::scroll` 视口裁剪、可选平滑滚动与右侧滚动条。

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Text};
use ratatui::widgets::Paragraph;

use crate::md_render::wrap_string_to_width;
use crate::repl::inline::{
    sanitize_stream_transcript_visual_noise, scrub_stream_transcript_llm_raw_dumps,
    stream_transcript_line_style,
};
use crate::repl::line_state::StreamTranscriptLayoutCache;
use crate::term::palette;

const SCROLL_EASE: f32 = 0.3;
const OVERSCROLL_CLAMP_EASE: f32 = 0.2;
const SCROLL_EPS: f32 = 0.01;
/// ADR 006: extra display rows built above/below the viewport to reduce pop-in while scrolling.
const VIRTUAL_SCROLL_OVERSCAN_ROWS: usize = 32;

fn virtual_scroll_overscan(viewport_h: usize) -> usize {
    VIRTUAL_SCROLL_OVERSCAN_ROWS.max(viewport_h / 4)
}

/// 与主区绘制一致：清洗后按 `width` 折行的总行数（单测断言用）。
#[cfg(test)]
fn stream_transcript_total_rows(raw: &str, width: u16) -> usize {
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

/// Incremental scrub/sanitize: rebuild `layout_cache.cleaned` only when transcript bytes change.
fn ensure_stream_transcript_cleaned(
    raw: &str,
    viewport_w: u16,
    layout_cache: &mut StreamTranscriptLayoutCache,
) {
    let raw_hash = hash_raw(raw, viewport_w);
    if layout_cache.raw_hash != raw_hash {
        let scrubbed = scrub_stream_transcript_llm_raw_dumps(raw);
        layout_cache.cleaned = sanitize_stream_transcript_visual_noise(&scrubbed);
        layout_cache.raw_hash = raw_hash;
        layout_cache.key = 0;
    }
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
        push_wrapped_logical_line(line, w, &mut lines);
    }
    Text::from(lines)
}

fn push_wrapped_logical_line(line: &str, w: usize, out: &mut Vec<Line<'static>>) {
    let t = line.trim_start();
    let style = stream_transcript_line_style(t, line);
    for row in wrap_string_to_width(line, w) {
        out.push(Line::from(ratatui::text::Span::styled(row, style)));
    }
}

/// 全局显示行 `row` 所在的逻辑行下标（`prefix_row[i]` 单调）。
fn logical_line_at_global_row(cache: &StreamTranscriptLayoutCache, row: usize) -> usize {
    let n = cache.prefix_row.len();
    if n == 0 {
        return 0;
    }
    if row >= cache.total_rows {
        return n.saturating_sub(1);
    }
    let mut lo = 0usize;
    let mut hi = n;
    while lo + 1 < hi {
        let mid = lo + (hi - lo) / 2;
        if cache.prefix_row[mid] <= row {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

/// 仅展开 `[global_start, global_end)` 范围内的显示行（虚拟滚动窗口）。
fn stream_cleaned_to_wrapped_styled_text_window(
    cleaned: &str,
    width: u16,
    cache: &StreamTranscriptLayoutCache,
    global_start: usize,
    global_end: usize,
) -> (Text<'static>, usize) {
    let logical: Vec<&str> = cleaned.lines().collect();
    if logical.is_empty() || global_start >= global_end {
        return (Text::from(Vec::<Line<'static>>::new()), 0);
    }
    let start_logical = logical_line_at_global_row(cache, global_start);
    let window_base = cache.prefix_row.get(start_logical).copied().unwrap_or(0);
    let w = width.max(1) as usize;
    let mut out: Vec<Line<'static>> = Vec::new();
    let mut built_global = window_base;

    for (i, line) in logical.iter().enumerate().skip(start_logical) {
        let t = line.trim_start();
        let style = stream_transcript_line_style(t, line);
        for row in wrap_string_to_width(line, w) {
            if built_global >= global_end && !out.is_empty() {
                break;
            }
            out.push(Line::from(ratatui::text::Span::styled(row, style)));
            built_global += 1;
        }
        let next_global = cache
            .prefix_row
            .get(i + 1)
            .copied()
            .unwrap_or(cache.total_rows);
        if next_global >= global_end && built_global >= global_end {
            break;
        }
    }
    (Text::from(out), window_base)
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
    let smooth = std::env::var("ANYCODE_TERM_SMOOTH_SCROLL")
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

/// 准备主区绘制：返回 `Paragraph`（视口窗口或短内容全量折行）、全局滚动偏移、是否贴底。
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
    let view = prepare_stream_transcript_view(
        raw,
        viewport_w,
        viewport_h,
        layout_cache,
        auto_scroll,
        scroll_up,
        scroll_pos,
        scroll_target,
    );
    let para = Paragraph::new(view.text).scroll((paragraph_scroll_y(view.local_scroll), 0));
    (
        para,
        view.global_off,
        view.max_scroll,
        view.total_rows,
        view.at_bottom,
    )
}

struct StreamTranscriptView {
    text: Text<'static>,
    local_scroll: usize,
    global_off: usize,
    max_scroll: usize,
    total_rows: usize,
    at_bottom: bool,
}

fn prepare_stream_transcript_view(
    raw: &str,
    viewport_w: u16,
    viewport_h: u16,
    layout_cache: &mut StreamTranscriptLayoutCache,
    auto_scroll: bool,
    scroll_up: usize,
    scroll_pos: &mut f32,
    scroll_target: &mut f32,
) -> StreamTranscriptView {
    ensure_stream_transcript_cleaned(raw, viewport_w, layout_cache);
    let cleaned = layout_cache.cleaned.clone();
    let key = layout_cache.raw_hash;
    update_stream_layout_cache(&cleaned, viewport_w, key, layout_cache);

    if cleaned.trim().is_empty() {
        *scroll_pos = 0.0;
        *scroll_target = 0.0;
        return StreamTranscriptView {
            text: Text::from(Vec::new()),
            local_scroll: 0,
            global_off: 0,
            max_scroll: 0,
            total_rows: 0,
            at_bottom: true,
        };
    }

    let total_rows = layout_cache.total_rows;
    let vh = viewport_h.max(1) as usize;

    let (global_off, _local_scroll, max_scroll, at_bottom) = compute_stream_scroll(
        total_rows,
        vh,
        auto_scroll,
        scroll_up,
        scroll_pos,
        scroll_target,
    );

    let overscan = virtual_scroll_overscan(vh);
    let use_virtual = total_rows > vh.saturating_add(overscan.saturating_mul(2));

    let (text, window_base) = if cleaned.trim().is_empty() {
        (Text::from(Vec::<Line<'static>>::new()), 0)
    } else if use_virtual {
        let win_start = global_off.saturating_sub(overscan);
        let win_end = (global_off + vh + overscan).min(total_rows);
        stream_cleaned_to_wrapped_styled_text_window(
            &cleaned,
            viewport_w,
            layout_cache,
            win_start,
            win_end,
        )
    } else {
        (
            stream_cleaned_to_wrapped_styled_text(&cleaned, viewport_w),
            0,
        )
    };
    let local_scroll = global_off.saturating_sub(window_base);
    StreamTranscriptView {
        text,
        local_scroll,
        global_off,
        max_scroll,
        total_rows,
        at_bottom,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repl::line_state::StreamTranscriptLayoutCache;

    fn synthetic_raw_transcript(logical_lines: usize) -> String {
        (0..logical_lines)
            .map(|i| format!("bench line {i}: {}", "x".repeat(48)))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn text_plain_rows(text: &Text<'static>, skip: usize, take: usize) -> Vec<String> {
        text.lines
            .iter()
            .skip(skip)
            .take(take)
            .map(|line| {
                line.spans
                    .iter()
                    .map(|s| s.content.to_string())
                    .collect::<String>()
            })
            .collect()
    }

    fn full_visible_rows(
        cleaned: &str,
        width: u16,
        viewport_h: usize,
        global_off: usize,
    ) -> Vec<String> {
        let all = stream_cleaned_to_wrapped_styled_text(cleaned, width);
        text_plain_rows(&all, global_off, viewport_h)
    }

    fn prepare_visible_plain_rows(
        raw: &str,
        width: u16,
        viewport_h: u16,
        cache: &mut StreamTranscriptLayoutCache,
        auto_scroll: bool,
        scroll_up: usize,
        pos: &mut f32,
        tgt: &mut f32,
    ) -> (Vec<String>, usize, usize, usize, bool) {
        let view = prepare_stream_transcript_view(
            raw,
            width,
            viewport_h,
            cache,
            auto_scroll,
            scroll_up,
            pos,
            tgt,
        );
        let visible = text_plain_rows(&view.text, view.local_scroll, viewport_h as usize);
        (
            visible,
            view.global_off,
            view.max_scroll,
            view.total_rows,
            view.at_bottom,
        )
    }

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
        // Default smooth scroll lerps scroll_pos toward the target each frame; one call is not guaranteed
        // to land exactly on desired_first unless NO_COLOR/smooth-disable env is set. Converge like the UI loop.
        let mut local_last = 0usize;
        for _ in 0..200 {
            let (_g, local, _, _) = compute_stream_scroll(20, 5, false, 3, &mut pos, &mut tgt);
            local_last = local;
            if local == 12 {
                break;
            }
        }
        assert_eq!(
            local_last, 12,
            "expected scroll_up=3 from bottom (max_scroll=15) to anchor at visual row offset 12"
        );
    }

    #[test]
    fn virtual_window_matches_full_render_while_scrolling() {
        let raw = synthetic_raw_transcript(400);
        let width = 60u16;
        let viewport_h = 20u16;
        let scrubbed = scrub_stream_transcript_llm_raw_dumps(&raw);
        let cleaned = sanitize_stream_transcript_visual_noise(&scrubbed);
        let scroll_positions = [0, 50, 120, 200, 350];

        for scroll_up in scroll_positions {
            let mut cache = StreamTranscriptLayoutCache::default();
            let mut pos = 0.0f32;
            let mut tgt = 0.0f32;
            let (windowed, global_off, _, _, _) = prepare_visible_plain_rows(
                &raw, width, viewport_h, &mut cache, false, scroll_up, &mut pos, &mut tgt,
            );
            let full = full_visible_rows(&cleaned, width, viewport_h as usize, global_off);
            assert_eq!(
                windowed, full,
                "scroll_up={scroll_up} global_off={global_off}"
            );
        }
    }

    #[test]
    fn virtual_window_matches_full_render_at_bottom() {
        let raw = synthetic_raw_transcript(500);
        let width = 72u16;
        let viewport_h = 24u16;
        let scrubbed = scrub_stream_transcript_llm_raw_dumps(&raw);
        let cleaned = sanitize_stream_transcript_visual_noise(&scrubbed);
        let mut cache = StreamTranscriptLayoutCache::default();
        let mut pos = 0.0f32;
        let mut tgt = 0.0f32;
        let (windowed, global_off, _, _, at_bottom) = prepare_visible_plain_rows(
            &raw, width, viewport_h, &mut cache, true, 0, &mut pos, &mut tgt,
        );
        assert!(at_bottom);
        let full = full_visible_rows(&cleaned, width, viewport_h as usize, global_off);
        assert_eq!(windowed, full);
    }

    #[test]
    fn resize_invalidates_layout_and_keeps_visible_tail() {
        let raw = synthetic_raw_transcript(300);
        let viewport_h = 18u16;
        let mut cache = StreamTranscriptLayoutCache::default();
        let mut pos = 0.0f32;
        let mut tgt = 0.0f32;

        let (narrow, off_narrow, _, _, _) = prepare_visible_plain_rows(
            &raw, 40, viewport_h, &mut cache, true, 0, &mut pos, &mut tgt,
        );
        let key_before = cache.key;
        let (wide, off_wide, _, _, _) = prepare_visible_plain_rows(
            &raw, 100, viewport_h, &mut cache, true, 0, &mut pos, &mut tgt,
        );
        assert_ne!(key_before, cache.key);
        assert!(off_wide <= off_narrow || cache.total_rows <= viewport_h as usize);
        assert!(!narrow.is_empty());
        assert!(!wide.is_empty());
    }

    #[test]
    fn cleared_transcript_renders_empty_paragraph() {
        let raw = synthetic_raw_transcript(200);
        let mut cache = StreamTranscriptLayoutCache::default();
        let mut pos = 0.0f32;
        let mut tgt = 0.0f32;
        let _ = prepare_visible_plain_rows(&raw, 80, 20, &mut cache, true, 0, &mut pos, &mut tgt);
        let (visible, global_off, max_scroll, total_rows, at_bottom) =
            prepare_visible_plain_rows("", 80, 20, &mut cache, true, 0, &mut pos, &mut tgt);
        assert_eq!(total_rows, 0);
        assert_eq!(max_scroll, 0);
        assert_eq!(global_off, 0);
        assert!(at_bottom);
        assert!(visible.is_empty());
    }

    #[test]
    fn tier_s_virtual_scroll_prepare_completes_quickly() {
        let raw = synthetic_raw_transcript(10_000);
        let mut cache = StreamTranscriptLayoutCache::default();
        let mut pos = 0.0f32;
        let mut tgt = 0.0f32;
        let start = std::time::Instant::now();
        let (_, _, _, total_rows, _) =
            prepare_visible_plain_rows(&raw, 80, 24, &mut cache, false, 500, &mut pos, &mut tgt);
        assert!(start.elapsed() < std::time::Duration::from_secs(2));
        assert!(total_rows >= 10_000);
        assert!(cache.prefix_row.len() >= 10_000);
    }

    #[test]
    fn tier_m_virtual_scroll_prepare_completes() {
        let raw = synthetic_raw_transcript(50_000);
        let mut cache = StreamTranscriptLayoutCache::default();
        let mut pos = 0.0f32;
        let mut tgt = 0.0f32;
        let start = std::time::Instant::now();
        let (visible, _, _, total_rows, _) =
            prepare_visible_plain_rows(&raw, 80, 24, &mut cache, false, 2_000, &mut pos, &mut tgt);
        assert!(start.elapsed() < std::time::Duration::from_secs(10));
        assert!(total_rows >= 50_000);
        assert_eq!(visible.len(), 24);
    }

    /// Tier L: ~100k logical lines; incremental scrub cache must stay hot on repeated prepare.
    #[test]
    fn tier_l_incremental_cache_repeated_prepare() {
        let raw = synthetic_raw_transcript(100_000);
        let mut cache = StreamTranscriptLayoutCache::default();
        let mut pos = 0.0f32;
        let mut tgt = 0.0f32;
        let (_, _, _, total_rows, _) =
            prepare_visible_plain_rows(&raw, 80, 24, &mut cache, true, 0, &mut pos, &mut tgt);
        assert!(total_rows >= 100_000);
        let key_after_first = cache.key;
        let start = std::time::Instant::now();
        for _ in 0..20 {
            let _ =
                prepare_visible_plain_rows(&raw, 80, 24, &mut cache, true, 0, &mut pos, &mut tgt);
        }
        assert_eq!(
            cache.key, key_after_first,
            "layout key stable when raw unchanged"
        );
        assert!(
            start.elapsed() < std::time::Duration::from_secs(3),
            "repeated prepare should reuse scrub/layout cache"
        );
    }
}
