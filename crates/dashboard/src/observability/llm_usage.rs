//! Normalize LLM token usage from trace log lines into index-tier `llm_usage` events.

use crate::log_parser::ParsedLine;
use serde_json::{json, Value};

pub const EVENT_TYPE: &str = "llm_usage";

/// Build a normalized usage payload from a parsed `llm_response_end` line.
#[must_use]
pub fn usage_payload_from_parsed(parsed: &ParsedLine) -> Option<Value> {
    if parsed.event_type != "llm_response_end" {
        return None;
    }
    let turn = parsed
        .payload
        .get("turn")
        .and_then(|v| v.as_str().map(str::to_string))
        .or_else(|| {
            parsed
                .payload
                .get("turn")
                .and_then(|v| v.as_i64())
                .map(|n| n.to_string())
        })
        .unwrap_or_else(|| "0".into());
    let input_tokens = token_int(&parsed.payload, "input_tokens");
    let output_tokens = token_int(&parsed.payload, "output_tokens");
    let elapsed_ms = token_int(&parsed.payload, "elapsed_ms");
    let cache_read_tokens = token_int(&parsed.payload, "cache_read_tokens");
    let cache_creation_tokens = token_int(&parsed.payload, "cache_creation_tokens");
    Some(json!({
        "turn": turn,
        "input_tokens": input_tokens,
        "output_tokens": output_tokens,
        "elapsed_ms": elapsed_ms,
        "cache_read_tokens": cache_read_tokens,
        "cache_creation_tokens": cache_creation_tokens,
    }))
}

#[must_use]
pub fn usage_dedup_key(turn: &str) -> String {
    format!("{EVENT_TYPE}:turn:{turn}")
}

fn token_int(payload: &Value, key: &str) -> i64 {
    payload
        .get(key)
        .and_then(|v| {
            v.as_i64()
                .or_else(|| v.as_u64().map(|n| n as i64))
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log_parser::parse_line;

    #[test]
    fn builds_usage_payload_from_llm_response_end() {
        let parsed = parse_line(
            "[llm_response_end] turn=2 elapsed_ms=1200 input_tokens=100 output_tokens=50",
        )
        .unwrap();
        let payload = usage_payload_from_parsed(&parsed).unwrap();
        assert_eq!(payload["turn"], "2");
        assert_eq!(payload["input_tokens"], 100);
        assert_eq!(payload["output_tokens"], 50);
        assert_eq!(payload["elapsed_ms"], 1200);
    }
}
