//! Refcounted session activity + periodic keepalive while LLM/tools run.

use super::logging::RunLogger;
use anycode_core::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use tokio::sync::watch;
use tokio::task::JoinHandle;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);

static ACTIVITY_REFCOUNT: AtomicU32 = AtomicU32::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActivityReason {
    ApiCall,
    ToolExec,
    RetryWait,
}

impl ActivityReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::ApiCall => "api_call",
            Self::ToolExec => "tool_exec",
            Self::RetryWait => "retry_wait",
        }
    }
}

/// RAII guard: bumps global activity refcount and emits `[session_keepalive]` every 30s.
pub(crate) struct SessionActivityGuard {
    _stop: watch::Sender<()>,
    _handle: JoinHandle<()>,
}

impl SessionActivityGuard {
    pub fn start(logger: RunLogger, task_id: TaskId, reason: ActivityReason) -> Self {
        let prev = ACTIVITY_REFCOUNT.fetch_add(1, Ordering::SeqCst);
        if prev == 0 {
            logger.session_state(task_id, "running");
        }
        let (stop_tx, stop_rx) = watch::channel(());
        let handle = tokio::spawn(run_heartbeat(logger, task_id, reason, stop_rx));
        Self {
            _stop: stop_tx,
            _handle: handle,
        }
    }
}

impl Drop for SessionActivityGuard {
    fn drop(&mut self) {
        let _ = self._stop.send(());
        let remaining = ACTIVITY_REFCOUNT
            .fetch_sub(1, Ordering::SeqCst)
            .saturating_sub(1);
        if remaining == 0 {
            // Best-effort idle signal on last guard drop — caller may emit explicit state later.
        }
    }
}

async fn run_heartbeat(
    logger: RunLogger,
    task_id: TaskId,
    reason: ActivityReason,
    mut stop: watch::Receiver<()>,
) {
    let mut interval = tokio::time::interval(HEARTBEAT_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            _ = stop.changed() => break,
            _ = interval.tick() => {
                let refcount = ACTIVITY_REFCOUNT.load(Ordering::SeqCst);
                logger.session_keepalive(task_id, reason.as_str(), refcount);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn activity_refcount_increments_and_decrements() {
        let before = ACTIVITY_REFCOUNT.load(Ordering::SeqCst);
        let logger = RunLogger::new(None);
        let task_id = Uuid::new_v4();
        let _g1 = SessionActivityGuard::start(logger.clone(), task_id, ActivityReason::ApiCall);
        assert_eq!(ACTIVITY_REFCOUNT.load(Ordering::SeqCst), before + 1);
        drop(_g1);
        assert_eq!(ACTIVITY_REFCOUNT.load(Ordering::SeqCst), before);
    }
}
