//! 粘贴限制与轻量字符串/滚动工具函数。

use ratatui::text::Line;

/// 单次粘贴最大字符数（防极端大块拖垮 UI）。
pub(crate) const MAX_PASTE_CHARS: usize = 120_000;

pub(crate) fn sanitize_paste(text: String) -> (String, bool) {
    let over_len = text.chars().count() > MAX_PASTE_CHARS;
    let clean: String = text
        .chars()
        .filter(|c| *c != '\0')
        .take(MAX_PASTE_CHARS)
        .collect();
    (clean, over_len)
}

pub(crate) fn transcript_first_visible(len: usize, available: usize, scroll_up: usize) -> usize {
    if len == 0 || available == 0 {
        return 0;
    }
    let max_first = len.saturating_sub(available);
    max_first.saturating_sub(scroll_up)
}

/// 短内容在视口内**顶对齐**（下方补空行至 `avail_h`），用于主缓冲空会话：避免品牌区被推到近底栏、上方出现大块空白。
pub(crate) fn top_align_viewport_lines(
    lines: Vec<Line<'static>>,
    avail_h: usize,
) -> Vec<Line<'static>> {
    if avail_h == 0 {
        return Vec::new();
    }
    if lines.len() >= avail_h {
        return lines.into_iter().take(avail_h).collect();
    }
    let mut out = lines;
    let pad = avail_h.saturating_sub(out.len());
    out.extend(std::iter::repeat_n(Line::from(""), pad));
    out
}

/// 对话视口最终行列表：主区为**满高**（与底栏两段布局），短内容在视口内**底对齐**（上方补空行），避免再叠一层 `[Min(0)][短主区][底栏]` 在对话与底栏之间制造第二条大块空白带。
/// 向上滚动时（`transcript_scroll_up > 0`）不补行，保持视口顶对齐。
pub(crate) fn bottom_align_viewport_lines(
    lines: Vec<Line<'static>>,
    avail_h: usize,
    transcript_scroll_up: usize,
) -> Vec<Line<'static>> {
    if transcript_scroll_up > 0 {
        return lines;
    }
    if lines.len() >= avail_h {
        return lines;
    }
    let pad = avail_h.saturating_sub(lines.len());
    let mut out = Vec::with_capacity(avail_h);
    out.extend(std::iter::repeat_n(Line::from(""), pad));
    out.extend(lines);
    out
}

pub(crate) fn trim_or_default(s: &str) -> &str {
    let t = s.trim();
    if t.is_empty() {
        ""
    } else {
        t
    }
}

pub(crate) fn truncate_preview(s: &str, max_chars: usize) -> String {
    let t = s.trim();
    if t.chars().count() <= max_chars {
        t.to_string()
    } else {
        format!("{}…", t.chars().take(max_chars).collect::<String>())
    }
}
