//! 粘贴限制与轻量字符串/滚动工具函数。

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
