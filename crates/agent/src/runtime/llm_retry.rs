//! Bridges LLM retry notifications to session logs.

use super::logging::RunLogger;
use anycode_core::{LlmRetryObserver, QuerySource, TaskId};

#[derive(Debug)]
pub(crate) struct AgentLlmRetryObserver {
    logger: RunLogger,
    task_id: TaskId,
}

impl AgentLlmRetryObserver {
    pub(crate) fn new(logger: RunLogger, task_id: TaskId) -> Self {
        Self { logger, task_id }
    }
}

impl LlmRetryObserver for AgentLlmRetryObserver {
    fn on_api_retry(&self, attempt: u32, delay_ms: u64, model: &str, source: QuerySource) {
        self.logger.session_state(self.task_id, "retrying");
        self.logger
            .api_retry(self.task_id, attempt, delay_ms, model, source.as_str());
    }
}

/// Attach per-request retry observer for source-aware API retries.
pub(crate) fn model_config_with_retry_observer(
    base: &anycode_core::ModelConfig,
    logger: RunLogger,
    task_id: TaskId,
) -> anycode_core::ModelConfig {
    let mut cfg = base.clone();
    cfg.retry_observer = Some(std::sync::Arc::new(AgentLlmRetryObserver::new(
        logger, task_id,
    )));
    cfg
}
