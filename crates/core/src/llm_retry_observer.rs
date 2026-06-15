//! Optional callback for LLM retry visibility (dashboard / session logs).

use crate::query_source::QuerySource;

/// Observes source-aware API retries (wired by agent runtime per request).
pub trait LlmRetryObserver: Send + Sync + std::fmt::Debug {
    fn on_api_retry(&self, attempt: u32, delay_ms: u64, model: &str, source: QuerySource);
}
