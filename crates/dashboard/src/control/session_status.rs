//! Canonical `TaskResult` / `TerminationReason` -> session status mapping.
//!
//! Chat, trigger, cron and automation records must all persist the same
//! terminal status semantics. Never hardcode `"completed"` at call sites.

use anycode_core::{TaskResult, TerminationReason, NESTED_TASK_COOPERATIVE_CANCEL_ERROR};

pub const STATUS_COMPLETED: &str = "completed";
pub const STATUS_FAILED: &str = "failed";
pub const STATUS_CANCELLED: &str = "cancelled";

/// Map an agent-loop termination reason to the persisted session status.
#[must_use]
pub fn session_status_for_termination(reason: TerminationReason) -> &'static str {
    match reason {
        TerminationReason::Cancelled => STATUS_CANCELLED,
        TerminationReason::MaxTurns
        | TerminationReason::MaxTools
        | TerminationReason::Budget
        | TerminationReason::RefusalNoTool
        | TerminationReason::Error => STATUS_FAILED,
        TerminationReason::Completed | TerminationReason::Partial => STATUS_COMPLETED,
    }
}

/// Map a `TaskResult` to the persisted session status.
///
/// Cooperative cancellation surfaces as `TaskResult::Failure` with the
/// canonical cancel error string; it must persist as `cancelled`, not `failed`.
#[must_use]
pub fn session_status_for_task_result(result: &TaskResult) -> &'static str {
    match result {
        TaskResult::Success { .. } | TaskResult::Partial { .. } => STATUS_COMPLETED,
        TaskResult::Failure { error, details } => {
            let cancelled = error == NESTED_TASK_COOPERATIVE_CANCEL_ERROR
                || details.as_deref() == Some(TerminationReason::Cancelled.as_str());
            if cancelled {
                STATUS_CANCELLED
            } else {
                STATUS_FAILED
            }
        }
    }
}

/// Summary text worth persisting alongside the status (best-effort).
#[must_use]
pub fn session_summary_for_task_result(result: &TaskResult) -> &str {
    match result {
        TaskResult::Success { output, .. } => output,
        TaskResult::Failure { error, .. } => error,
        TaskResult::Partial { success, .. } => success,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn termination_reasons_map_to_expected_statuses() {
        assert_eq!(
            session_status_for_termination(TerminationReason::Completed),
            STATUS_COMPLETED
        );
        assert_eq!(
            session_status_for_termination(TerminationReason::Partial),
            STATUS_COMPLETED
        );
        assert_eq!(
            session_status_for_termination(TerminationReason::Cancelled),
            STATUS_CANCELLED
        );
        for reason in [
            TerminationReason::MaxTurns,
            TerminationReason::MaxTools,
            TerminationReason::Budget,
            TerminationReason::RefusalNoTool,
            TerminationReason::Error,
        ] {
            assert_eq!(session_status_for_termination(reason), STATUS_FAILED);
        }
    }

    #[test]
    fn task_result_success_maps_completed() {
        let result = TaskResult::Success {
            output: "ok".into(),
            artifacts: vec![],
        };
        assert_eq!(session_status_for_task_result(&result), STATUS_COMPLETED);
        assert_eq!(session_summary_for_task_result(&result), "ok");
    }

    #[test]
    fn task_result_failure_maps_failed_with_reason_details() {
        for reason in [
            TerminationReason::Budget,
            TerminationReason::MaxTurns,
            TerminationReason::MaxTools,
            TerminationReason::RefusalNoTool,
        ] {
            let result = TaskResult::Failure {
                error: "boom".into(),
                details: Some(reason.as_str().to_string()),
            };
            assert_eq!(session_status_for_task_result(&result), STATUS_FAILED);
        }
    }

    #[test]
    fn cooperative_cancel_failure_maps_cancelled() {
        let result = TaskResult::Failure {
            error: NESTED_TASK_COOPERATIVE_CANCEL_ERROR.to_string(),
            details: Some("cooperative nested cancel".into()),
        };
        assert_eq!(session_status_for_task_result(&result), STATUS_CANCELLED);
    }

    #[test]
    fn cancelled_details_maps_cancelled() {
        let result = TaskResult::Failure {
            error: "task stopped".into(),
            details: Some(TerminationReason::Cancelled.as_str().to_string()),
        };
        assert_eq!(session_status_for_task_result(&result), STATUS_CANCELLED);
    }
}
