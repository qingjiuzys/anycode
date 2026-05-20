//! Lightweight tool output sanitizer metadata (does not silently rewrite business content).

use regex::Regex;
use std::sync::LazyLock;

static RE_API_KEY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(api[_-]?key|token|secret)\s*[:=]\s*\S+").expect("regex"));
static RE_BEARER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)bearer\s+[A-Za-z0-9\-._~+/]+=*").expect("regex"));

#[derive(Debug, Clone, Default)]
pub(crate) struct SanitizerReport {
    pub redacted_secret_patterns: u32,
    pub marked_prompt_injection: bool,
}

pub(crate) fn sanitize_tool_output(text: &str) -> (String, SanitizerReport) {
    let mut report = SanitizerReport::default();
    let mut out = text.to_string();
    if RE_API_KEY.is_match(&out) {
        out = RE_API_KEY.replace_all(&out, "$1: [REDACTED]").into_owned();
        report.redacted_secret_patterns += 1;
    }
    if RE_BEARER.is_match(&out) {
        out = RE_BEARER
            .replace_all(&out, "Bearer [REDACTED]")
            .into_owned();
        report.redacted_secret_patterns += 1;
    }
    let lower = out.to_ascii_lowercase();
    if lower.contains("ignore previous instructions") || lower.contains("system prompt override") {
        report.marked_prompt_injection = true;
    }
    (out, report)
}

#[cfg(test)]
mod tests {
    use super::sanitize_tool_output;

    #[test]
    fn redacts_api_key_like_patterns() {
        let (out, r) = sanitize_tool_output("api_key=sk-secret-value");
        assert!(out.contains("[REDACTED]"));
        assert!(r.redacted_secret_patterns > 0);
    }
}
