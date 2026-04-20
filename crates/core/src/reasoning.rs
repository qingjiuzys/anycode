//! Strip `<thought>` / `<thinking>` / `redacted_thinking` blocks that some models emit
//! (must not appear in terminal UX). Streaming chunks may end inside an opening tag — see
//! [`strip_llm_reasoning_for_display`].

use regex::Regex;
use std::sync::OnceLock;

/// Remove paired reasoning blocks (case-insensitive, multiline), including Zhipu-style
/// `<think>...</think>`.
pub fn strip_llm_reasoning_xml_blocks(text: &str) -> String {
    static RE_THOUGHT: OnceLock<Regex> = OnceLock::new();
    static RE_THINKING: OnceLock<Regex> = OnceLock::new();
    static RE_REDACTED: OnceLock<Regex> = OnceLock::new();
    let re_thought = RE_THOUGHT.get_or_init(|| {
        Regex::new(r"(?is)<thought\b[^>]*>.*?</thought>").expect("thought strip regex")
    });
    let re_thinking = RE_THINKING.get_or_init(|| {
        Regex::new(r"(?is)<thinking\b[^>]*>.*?</thinking>").expect("thinking strip regex")
    });
    let re_redacted = RE_REDACTED.get_or_init(|| {
        // 闭合标签用 concat!：避免源码里 `</think>` 被误写成 `</think>`。
        Regex::new(concat!(
            "(?is)<redacted",
            "_thinking[^>]*>.*?",
            "</redacted",
            "_thinking>"
        ))
        .expect("redacted_thinking strip regex")
    });
    let s = re_thought.replace_all(text, "");
    let s = re_thinking.replace_all(&s, "");
    re_redacted.replace_all(&s, "").to_string()
}

/// Like [`strip_llm_reasoning_xml_blocks`], then drop a **trailing** unclosed reasoning region:
/// from the earliest `<thought` / `<thinking` / `<redacted_thinking` that has no closing tag
/// to end-of-string. Aligns stream/TUI with Claude-style UX (no `⏺ <thought` leaks).
pub fn strip_llm_reasoning_for_display(text: &str) -> String {
    let base = strip_llm_reasoning_xml_blocks(text);
    strip_trailing_unclosed_reasoning_open(&base)
}

fn strip_trailing_unclosed_reasoning_open(s: &str) -> String {
    static RE_OPEN: OnceLock<Regex> = OnceLock::new();
    let re = RE_OPEN.get_or_init(|| {
        Regex::new(r"(?i)<(?:thought|thinking|redacted_thinking)\b").expect("reasoning open regex")
    });
    let mut cut: Option<usize> = None;
    for m in re.find_iter(s) {
        let tail = &s[m.start()..];
        if !tail_has_reasoning_close(tail) {
            cut = Some(cut.map(|c| c.min(m.start())).unwrap_or(m.start()));
        }
    }
    match cut {
        Some(c) => s[..c].trim_end().to_string(),
        None => s.to_string(),
    }
}

fn tail_has_reasoning_close(tail: &str) -> bool {
    let l = tail.to_ascii_lowercase();
    l.contains("</thought>")
        || l.contains("</thinking>")
        || l.contains(concat!("</redacted", "_thinking>"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_thought_and_trims() {
        let s = "<thought>x</thought>\n\nHello";
        assert_eq!(strip_llm_reasoning_xml_blocks(s).trim(), "Hello");
    }

    #[test]
    fn strips_redacted_thinking_pair() {
        // 拼串避免误写标签名（与 GLM 文档一致）。
        let s = ["<redacted", "_thinking>z</redacted", "_thinking>ok"].concat();
        assert_eq!(strip_llm_reasoning_xml_blocks(&s).trim(), "ok");
    }

    #[test]
    fn display_strips_incomplete_open_tag() {
        let s = "Hello\n<thought\nstill streaming";
        let o = strip_llm_reasoning_for_display(s);
        assert_eq!(o.trim(), "Hello");
        assert!(!o.to_lowercase().contains("<thought"));
    }

    #[test]
    fn display_keeps_text_after_closed_block() {
        let s = "<thought>x</thought>Visible<thought partial";
        let o = strip_llm_reasoning_for_display(s);
        assert_eq!(o.trim(), "Visible");
    }
}
