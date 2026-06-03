//! Chat model failover when primary provider returns geo/rate-limit/server errors.

use anycode_core::CoreError;
use anycode_llm::FailoverTrigger;

pub struct FailoverPolicy {
    pub fallback: anycode_core::ModelConfig,
    pub trigger: FailoverTrigger,
}

pub fn error_triggers_failover(err: &CoreError, trigger: FailoverTrigger) -> bool {
    let msg = match err {
        CoreError::LLMError(s) => s.as_str(),
        _ => return false,
    };
    let lower = msg.to_ascii_lowercase();
    let is_geo = lower.contains("user location")
        || lower.contains("failed_precondition")
        || lower.contains("not supported for the api");
    let is_rate = lower.contains("rate limit") || lower.contains("429");
    let is_auth =
        lower.contains("401") || lower.contains("403") || lower.contains("invalid api key");
    let is_server = lower.contains("status=5") || lower.contains("502") || lower.contains("503");
    match trigger {
        FailoverTrigger::Geo => is_geo,
        FailoverTrigger::RateLimit => is_rate,
        FailoverTrigger::AnyError => is_geo || is_rate || is_auth || is_server,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geo_error_triggers() {
        let err = CoreError::LLMError("google API error: User location is not supported".into());
        assert!(error_triggers_failover(&err, FailoverTrigger::Geo));
    }
}
