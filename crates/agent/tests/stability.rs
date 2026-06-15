//! Execution stability regression tests (exception paths).

use anycode_core::QuerySource;
use anycode_llm::{ErrorCategory, RetryStrategy};

#[test]
fn background_llm_does_not_retry_overload() {
    let strategy = RetryStrategy::default();
    assert!(!strategy.should_retry_for_source(QuerySource::Summary, ErrorCategory::RateLimit, 1));
}

#[test]
fn foreground_llm_retries_overload() {
    let strategy = RetryStrategy::default();
    assert!(strategy.should_retry_for_source(QuerySource::MainTurn, ErrorCategory::RateLimit, 1));
}

#[test]
fn retry_chunks_sum_to_total_delay() {
    use std::time::Duration;
    let chunks = RetryStrategy::retry_sleep_chunks(
        Duration::from_secs(65),
        RetryStrategy::RETRY_HEARTBEAT_MS,
    );
    let total: Duration = chunks.iter().copied().sum();
    assert_eq!(total, Duration::from_secs(65));
}
