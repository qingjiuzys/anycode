//! Source-aware HTTP retry with chunked sleep and overload tracking.

use anycode_core::{CoreError, LlmRetryObserver, QuerySource};
use reqwest::StatusCode;
use std::time::Duration;
use tracing::warn;

use crate::retry_strategy::{ErrorCategory, ProviderRetryConfig, RetryStrategy};

/// Outcome of a retriable HTTP attempt.
pub struct RetryAttemptOutcome {
    pub should_retry: bool,
    pub delay: Duration,
    pub category: ErrorCategory,
    pub consecutive_overload: u32,
    pub should_fallback: bool,
}

/// Evaluate whether to retry after a non-success HTTP response.
#[must_use]
pub fn evaluate_http_retry(
    provider: &ProviderRetryConfig,
    source: QuerySource,
    status: StatusCode,
    error_text: &str,
    attempt: u32,
    consecutive_overload: u32,
    retry_after_ms: Option<u64>,
) -> RetryAttemptOutcome {
    let strategy = RetryStrategy::new(provider.base_config.clone());
    let category = strategy.categorize_error(status, error_text);
    let mut overload = consecutive_overload;
    if category == ErrorCategory::RateLimit {
        overload = overload.saturating_add(1);
    } else if category != ErrorCategory::ServerError {
        overload = 0;
    }
    let should_retry = provider.is_status_retryable(status)
        && strategy.should_retry_for_source(source, category, attempt);
    let delay = if should_retry {
        strategy.compute_delay(attempt, retry_after_ms)
    } else {
        Duration::ZERO
    };
    let should_fallback = overload >= RetryStrategy::MAX_CONSECUTIVE_OVERLOAD_BEFORE_FALLBACK
        && source.is_foreground();
    RetryAttemptOutcome {
        should_retry,
        delay,
        category,
        consecutive_overload: overload,
        should_fallback,
    }
}

/// Evaluate network-layer retry (no HTTP status).
#[must_use]
pub fn evaluate_network_retry(
    provider: &ProviderRetryConfig,
    source: QuerySource,
    attempt: u32,
) -> RetryAttemptOutcome {
    let strategy = RetryStrategy::new(provider.base_config.clone());
    let category = ErrorCategory::NetworkError;
    let should_retry = strategy.should_retry_for_source(source, category, attempt);
    let delay = if should_retry {
        strategy.compute_delay(attempt, None)
    } else {
        Duration::ZERO
    };
    RetryAttemptOutcome {
        should_retry,
        delay,
        category,
        consecutive_overload: 0,
        should_fallback: false,
    }
}

/// Sleep for retry delay in heartbeat-sized chunks; emit observer + tracing.
pub async fn sleep_retry_delay(
    delay: Duration,
    attempt: u32,
    model: &str,
    source: QuerySource,
    observer: Option<&dyn LlmRetryObserver>,
) {
    if delay.is_zero() {
        return;
    }
    let chunks = RetryStrategy::retry_sleep_chunks(delay, RetryStrategy::RETRY_HEARTBEAT_MS);
    for chunk in chunks {
        if let Some(obs) = observer {
            obs.on_api_retry(attempt, chunk.as_millis() as u64, model, source);
        }
        warn!(
            attempt,
            delay_ms = chunk.as_millis() as u64,
            model,
            source = source.as_str(),
            "LLM API retry wait"
        );
        tokio::time::sleep(chunk).await;
    }
}

/// Parse Retry-After header value (seconds) into milliseconds.
#[must_use]
pub fn retry_after_header_ms(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    headers
        .get("retry-after")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .map(|secs| secs.saturating_mul(1000))
}

/// Build overload error after retries exhausted.
pub fn retry_exhausted_error(provider_label: &str, detail: &str) -> CoreError {
    CoreError::LLMError(format!(
        "{provider_label} request failed after retries: {detail}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_skips_rate_limit_retry() {
        let provider = ProviderRetryConfig::openai();
        let out = evaluate_http_retry(
            &provider,
            QuerySource::Title,
            StatusCode::TOO_MANY_REQUESTS,
            "",
            1,
            0,
            None,
        );
        assert!(!out.should_retry);
    }

    #[test]
    fn foreground_retries_rate_limit() {
        let provider = ProviderRetryConfig::openai();
        let out = evaluate_http_retry(
            &provider,
            QuerySource::MainTurn,
            StatusCode::TOO_MANY_REQUESTS,
            "",
            1,
            0,
            None,
        );
        assert!(out.should_retry);
        assert_eq!(out.consecutive_overload, 1);
    }

    #[test]
    fn overload_triggers_fallback_after_threshold() {
        let provider = ProviderRetryConfig::openai();
        let out = evaluate_http_retry(
            &provider,
            QuerySource::MainTurn,
            StatusCode::TOO_MANY_REQUESTS,
            "",
            1,
            RetryStrategy::MAX_CONSECUTIVE_OVERLOAD_BEFORE_FALLBACK,
            None,
        );
        assert!(out.should_fallback);
    }

    #[test]
    fn retry_sleep_chunks_split_long_waits() {
        let chunks = RetryStrategy::retry_sleep_chunks(
            Duration::from_secs(75),
            RetryStrategy::RETRY_HEARTBEAT_MS,
        );
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0], Duration::from_secs(30));
        assert_eq!(chunks[1], Duration::from_secs(30));
        assert_eq!(chunks[2], Duration::from_secs(15));
    }
}
