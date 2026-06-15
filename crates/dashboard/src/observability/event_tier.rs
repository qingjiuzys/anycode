//! Event tier: index events live in SQLite; trace events stay in output.log only.

/// Events persisted to `project_events` (conversation index + governance signals).
#[must_use]
pub fn is_index_event_type(event_type: &str) -> bool {
    matches!(
        event_type,
        "task_start"
            | "task_end"
            | "user_prompt"
            | "assistant_response"
            | "gate"
            | "tool_denied"
            | "tool_approval_pending"
            | "tool_approval_resolved"
            | "budget_warning"
            | "budget_degrade"
            | "budget_exceeded"
            | "workflow_step"
            | "plan_step"
            | "session_error"
            | "llm_usage"
    )
}

/// Stack-level execution events read on demand from output.log.
#[must_use]
pub fn is_trace_event_type(event_type: &str) -> bool {
    matches!(
        event_type,
        "turn_start"
            | "turn_end"
            | "llm_request_start"
            | "llm_response_end"
            | "tool_call_input"
            | "tool_call_start"
            | "tool_call_end"
    ) || event_type.starts_with("tool_call")
        || event_type.starts_with("turn_")
        || (event_type.starts_with("llm_") && event_type != LLM_USAGE_EVENT)
}

const LLM_USAGE_EVENT: &str = "llm_usage";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_and_trace_are_disjoint_for_known_types() {
        for ty in [
            "user_prompt",
            "assistant_response",
            "task_end",
            "gate",
            "budget_exceeded",
        ] {
            assert!(is_index_event_type(ty));
            assert!(!is_trace_event_type(ty));
        }
        for ty in [
            "turn_start",
            "llm_request_start",
            "llm_response_end",
            "tool_call_end",
            "tool_call_input",
        ] {
            assert!(is_trace_event_type(ty));
            assert!(!is_index_event_type(ty));
        }
        assert!(is_index_event_type("llm_usage"));
        assert!(!is_trace_event_type("llm_usage"));
    }
}
