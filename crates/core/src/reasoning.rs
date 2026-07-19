//! Strip `<think>` / `<thought>` / `<thinking>` / `<think>` blocks that
//! some models emit (must not appear in terminal UX or dashboard SSE). Streaming
//! chunks may end inside an opening tag — see [`strip_llm_reasoning_for_display`].

use regex::Regex;
use std::sync::OnceLock;

/// Opening tags stripped when paired with a closing tag (case-insensitive).
const REASONING_CLOSED_TAGS: &[&str] = &["think", "thought", "thinking", "redacted_thinking"];

/// Opening tags considered for trailing unclosed regions (longest first so
/// `thinking` matches before `think`).
const REASONING_UNCLOSED_TAGS: &[&str] = &["redacted_thinking", "thinking", "thought", "think"];

fn closed_block_regex(tag: &str) -> Regex {
    if tag == "redacted_thinking" {
        Regex::new(concat!(
            "(?is)<redacted",
            "_thinking\\b[^>]*>.*?",
            "</redacted",
            "_thinking>"
        ))
        .expect("redacted_thinking strip regex")
    } else {
        Regex::new(&format!(r"(?is)<{tag}\b[^>]*>.*?</{tag}>"))
            .expect("reasoning closed-block regex")
    }
}

fn reasoning_open_regex(tags: &[&str]) -> Regex {
    let alts = tags
        .iter()
        .map(|t| regex::escape(t))
        .collect::<Vec<_>>()
        .join("|");
    Regex::new(&format!(r"(?i)<(?:{alts})\b")).expect("reasoning open regex")
}

fn reasoning_open_tag_regex(tags: &[&str]) -> Regex {
    let alts = tags
        .iter()
        .map(|t| regex::escape(t))
        .collect::<Vec<_>>()
        .join("|");
    Regex::new(&format!(r"(?i)<(?:{alts})\b[^>]*>")).expect("reasoning open tag regex")
}

/// Remove paired reasoning blocks (case-insensitive, multiline), including
/// Zhipu-style `<think>...</think>`.
pub fn strip_llm_reasoning_xml_blocks(text: &str) -> String {
    static CLOSED: OnceLock<Vec<Regex>> = OnceLock::new();
    let patterns = CLOSED.get_or_init(|| {
        REASONING_CLOSED_TAGS
            .iter()
            .map(|tag| closed_block_regex(tag))
            .collect()
    });
    let mut s = text.to_string();
    for re in patterns {
        s = re.replace_all(&s, "").to_string();
    }
    s
}

/// Like [`strip_llm_reasoning_xml_blocks`], then drop a **trailing** unclosed
/// reasoning region from the earliest unmatched open tag to end-of-string.
pub fn strip_llm_reasoning_for_display(text: &str) -> String {
    let base = strip_llm_reasoning_xml_blocks(text);
    strip_trailing_unclosed_reasoning_open(&base)
}

fn strip_trailing_unclosed_reasoning_open(s: &str) -> String {
    static RE_OPEN: OnceLock<Regex> = OnceLock::new();
    let re = RE_OPEN.get_or_init(|| reasoning_open_regex(REASONING_UNCLOSED_TAGS));
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
    for tag in REASONING_UNCLOSED_TAGS {
        if l.contains(&format!("</{tag}>")) {
            return true;
        }
    }
    false
}

/// Text inside the trailing unclosed reasoning block, if any (internal/debug).
pub fn extract_unclosed_reasoning_content(text: &str) -> Option<String> {
    static RE_OPEN: OnceLock<Regex> = OnceLock::new();
    let re = RE_OPEN.get_or_init(|| reasoning_open_tag_regex(REASONING_UNCLOSED_TAGS));
    let mut last_open: Option<std::ops::Range<usize>> = None;
    for m in re.find_iter(text) {
        let tail = &text[m.start()..];
        if !tail_has_reasoning_close(tail) {
            last_open = Some(m.range());
        }
    }
    let open = last_open?;
    let content = text[open.end..].trim();
    if content.is_empty() {
        None
    } else {
        Some(content.to_string())
    }
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
    fn strips_think_pair() {
        let s = concat!("<", "think>plan</", "think>Answer");
        assert_eq!(strip_llm_reasoning_xml_blocks(s).trim(), "Answer");
    }

    #[test]
    fn strips_redacted_thinking_pair() {
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
    fn display_strips_incomplete_think_tag() {
        let s = "Visible<thinking\nhidden";
        let o = strip_llm_reasoning_for_display(s);
        assert_eq!(o.trim(), "Visible");
    }

    #[test]
    fn display_strips_bare_redacted_open_tag() {
        let s = ["<redacted", "_thinking>"].concat();
        assert_eq!(strip_llm_reasoning_for_display(&s).trim(), "");
    }

    #[test]
    fn display_strips_streaming_redacted_thinking_chunk() {
        let s = ["Hello\n<redacted", "_thinking>\nsecret plan"].concat();
        assert_eq!(strip_llm_reasoning_for_display(&s).trim(), "Hello");
    }

    #[test]
    fn mixed_body_survives_after_closed_reasoning() {
        let s = "<thinking>inner</thinking>\n\nFinal answer";
        assert_eq!(strip_llm_reasoning_for_display(s).trim(), "Final answer");
    }

    #[test]
    fn strips_redacted_then_leaves_hello() {
        let s = [
            "<redacted",
            "_thinking>secret</redacted",
            "_thinking>\nHello",
        ]
        .concat();
        assert_eq!(strip_llm_reasoning_for_display(&s).trim(), "Hello");
    }

    #[test]
    fn extract_unclosed_reasoning_tail() {
        let s = ["<redacted", "_thinking>\nplanning ppt\nstill going"].concat();
        assert_eq!(
            extract_unclosed_reasoning_content(&s).as_deref(),
            Some("planning ppt\nstill going")
        );
    }
}
