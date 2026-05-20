//! Provider error detection and context-overflow heuristics for LLM responses.

use anycode_core::CoreError;

/// 顶层 `"error": "…"` 为字符串时，排除助手正文里举例用的短 JSON（如 `{"error":"demo"}`）。
fn provider_error_string_smells_like_api(s: &str) -> bool {
    const NEEDLES: &[&str] = &[
        "User location",
        "not supported for the API",
        "FAILED_PRECONDITION",
        "generativelanguage",
        "Incorrect API key",
        "invalid_api_key",
        "invalid request",
        "rate limit",
        "quota",
        "exceeded",
    ];
    NEEDLES.iter().any(|n| s.contains(n))
}

/// OpenAI/Gemini 兼容：HTTP 200 但正文是 error JSON（流式 delta 或非流式 `choices.message.content`）。
fn summary_from_parsed_provider_error_value(err_body: &serde_json::Value) -> Option<String> {
    let err = err_body.get("error")?;
    match err {
        serde_json::Value::String(s) => {
            let s = s.trim();
            if s.is_empty() {
                None
            } else if provider_error_string_smells_like_api(s) {
                Some(s.to_string())
            } else {
                None
            }
        }
        serde_json::Value::Object(_) => {
            let msg = err
                .get("message")
                .and_then(|m| m.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty());
            let status = err.get("status").and_then(|s| s.as_str());
            let code = err.get("code");
            let looks_structured = msg.is_some()
                || status.is_some()
                || matches!(code, Some(serde_json::Value::Number(_)));
            if !looks_structured {
                return None;
            }
            Some(
                msg.map(|s| s.to_string())
                    .or_else(|| status.map(|s| s.to_string()))
                    .unwrap_or_else(|| "provider error object".to_string()),
            )
        }
        _ => None,
    }
}

fn try_parse_provider_error_top_json(t: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(t).ok()?;
    let err_body = match &v {
        serde_json::Value::Array(a) => a.first()?,
        serde_json::Value::Object(_) => &v,
        _ => return None,
    };
    summary_from_parsed_provider_error_value(err_body)
}

fn try_parse_provider_error_after_bom(t: &str) -> Option<String> {
    let u = t.trim().trim_start_matches('\u{feff}');
    try_parse_provider_error_top_json(u)
}

fn heuristic_openai_compat_error_blob(t: &str) -> bool {
    let u = t.trim().trim_start_matches('\u{feff}');
    if u.len() < 25 || u.len() > 512 * 1024 {
        return false;
    }
    if u.contains("generativelanguage.googleapis.com") {
        return true;
    }
    if u.contains("googleapis.com") && u.contains("\"error\"") {
        return true;
    }
    if !(u.starts_with('{') || u.starts_with('[')) {
        return false;
    }
    if !u.contains("\"error\"") {
        return false;
    }
    u.contains("User location is not supported")
        || u.contains("FAILED_PRECONDITION")
        || (u.contains("\"message\"") && u.contains("\"code\""))
}

pub(crate) fn provider_error_from_streamed_assistant_text(text: &str) -> Option<String> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }

    if let Some(s) = try_parse_provider_error_top_json(t) {
        return Some(format!("streamed provider error: {s}"));
    }
    if let Some(s) = try_parse_provider_error_after_bom(t) {
        return Some(format!("streamed provider error: {s}"));
    }

    if heuristic_openai_compat_error_blob(t) {
        return Some(
            "streamed provider error: response body looks like API error JSON (details on stderr)"
                .to_string(),
        );
    }

    if t.contains("\"error\"") && t.contains("User location is not supported") {
        return Some(format!(
            "streamed provider error: {}",
            t.chars().take(600).collect::<String>()
        ));
    }
    if t.contains("FAILED_PRECONDITION") && t.contains("\"error\"") {
        return Some(format!(
            "streamed provider error: {}",
            t.chars().take(600).collect::<String>()
        ));
    }
    None
}

pub(crate) fn error_indicates_context_overflow(msg: &str) -> bool {
    let l = msg.to_ascii_lowercase();
    [
        "context_length_exceeded",
        "context length exceeded",
        "maximum context length",
        "prompt is too long",
        "prompt too long",
        "too many tokens",
        "token limit",
        "context window",
        "max context",
    ]
    .iter()
    .any(|needle| l.contains(needle))
}

pub(crate) fn core_error_is_context_overflow(err: &CoreError) -> bool {
    match err {
        CoreError::LLMError(s) => error_indicates_context_overflow(s),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::provider_error_from_streamed_assistant_text;

    #[test]
    fn detects_context_length_exceeded_in_streamed_json() {
        let j = r#"{"error":{"message":"context_length_exceeded","type":"invalid_request_error"}}"#;
        assert!(provider_error_from_streamed_assistant_text(j).is_some());
    }

    #[test]
    fn detects_overloaded_error_in_streamed_json() {
        let j = r#"{"error":{"message":"The engine is currently overloaded","type":"overloaded_error"}}"#;
        assert!(provider_error_from_streamed_assistant_text(j).is_some());
    }

    #[test]
    fn detects_rate_limit_in_streamed_json() {
        let j = r#"{"error":{"message":"Rate limit exceeded","type":"rate_limit_error"}}"#;
        assert!(provider_error_from_streamed_assistant_text(j).is_some());
    }

    #[test]
    fn detects_array_wrapped_google_json() {
        let j = r#"[{"error":{"code":400,"message":"User location is not supported for the API use.","status":"FAILED_PRECONDITION"}}]"#;
        let e = provider_error_from_streamed_assistant_text(j).expect("should detect");
        assert!(
            e.contains("User location") || e.contains("streamed provider error"),
            "{e}"
        );
    }

    #[test]
    fn ignores_normal_assistant_prose() {
        let j = "Here is an example JSON: {\"error\": \"not a provider envelope\"}";
        assert!(provider_error_from_streamed_assistant_text(j).is_none());
    }

    #[test]
    fn detects_top_level_error_string() {
        let j = r#"{"error":"User location is not supported for the API use."}"#;
        let e = provider_error_from_streamed_assistant_text(j).expect("detect");
        assert!(e.contains("User location") || e.contains("streamed"), "{e}");
    }

    #[test]
    fn bom_before_json_still_parses() {
        let j =
            "\u{feff}[{\"error\":{\"code\":400,\"message\":\"User location is not supported\"}}]";
        assert!(provider_error_from_streamed_assistant_text(j).is_some());
    }

    #[test]
    fn heuristic_truncated_geo_json() {
        let j = r#"[{"error":{"code":400,"message":"User location is not supported","status":"FAILED_PRECONDITION""#;
        assert!(provider_error_from_streamed_assistant_text(j).is_some());
    }
}
