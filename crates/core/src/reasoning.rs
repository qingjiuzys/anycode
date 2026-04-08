//! Strip `<thought>` / `<thinking>` blocks that some models emit (must not appear in terminal UX).

use regex::Regex;
use std::sync::OnceLock;

/// Remove `<thought>...</thought>` and `<thinking>...</thinking>` (case-insensitive, multiline).
pub fn strip_llm_reasoning_xml_blocks(text: &str) -> String {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r"(?is)<(?:thought|thinking)\b[^>]*>.*?</(?:thought|thinking)>")
            .expect("reasoning block strip regex")
    });
    re.replace_all(text, "").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_thought_and_trims() {
        let s = "<thought>x</thought>\n\nHello";
        assert_eq!(strip_llm_reasoning_xml_blocks(s).trim(), "Hello");
    }
}
