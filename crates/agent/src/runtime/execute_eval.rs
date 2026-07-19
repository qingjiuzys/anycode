//! Thin eval scenario entry — delegates to `execute_task` (no second agent loop).

use anycode_core::{
    judge_eval_scenario, AgentLoopLimits, AgentType, EvalResult, EvalScenario, EvalStatus,
    ExecutionTraceEvent, Task, TaskBudget, TaskContext,
};
use uuid::Uuid;

use super::AgentRuntime;

impl AgentRuntime {
    /// Run one eval scenario through the normal task path and judge structured expectations.
    pub async fn execute_eval_scenario(&self, scenario: EvalScenario) -> EvalResult {
        let agent_type = scenario
            .agent
            .as_deref()
            .map(AgentType::new)
            .unwrap_or_else(|| AgentType::new("general-purpose"));
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".into());
        let task = Task {
            id: Uuid::new_v4(),
            agent_type,
            prompt: scenario.prompt.clone(),
            context: TaskContext {
                session_id: Uuid::new_v4(),
                working_directory: cwd,
                environment: Default::default(),
                user_id: None,
                system_prompt_append: None,
                context_injections: vec![],
                nested_model_override: None,
                nested_worktree_path: None,
                nested_worktree_repo_root: None,
                nested_cancel: None,
                channel_progress_tx: None,
                live_trace_tx: None,
                tool_deny_names: vec![],
                tool_deny_prefixes: vec![],
                user_vision_images: vec![],
                budget: TaskBudget::default(),
                loop_limits: AgentLoopLimits::default(),
                chat_turn: None,
            },
            created_at: chrono::Utc::now(),
        };
        match self.execute_task(task).await {
            Ok(result) => {
                let (status, text) = task_result_summary(&result);
                let trace = trace_from_task_result(&result);
                judge_eval_scenario(&scenario, &trace, status, text.as_deref())
            }
            Err(e) if e.is_cooperative_cancel() => {
                let trace = vec![ExecutionTraceEvent::new(
                    "task_end",
                    "info",
                    "cancelled",
                    e.to_string(),
                    serde_json::json!({ "status": "cancelled" }),
                )];
                judge_eval_scenario(&scenario, &trace, "cancelled", None)
            }
            Err(e) => EvalResult {
                scenario_id: scenario.id,
                status: EvalStatus::Error,
                trace: vec![ExecutionTraceEvent::new(
                    "task_error",
                    "error",
                    "runtime error",
                    e.to_string(),
                    serde_json::json!({}),
                )],
                message: e.to_string(),
                final_text: None,
            },
        }
    }
}

fn task_result_summary(result: &anycode_core::TaskResult) -> (&'static str, Option<String>) {
    match result {
        anycode_core::TaskResult::Success { output, .. } => ("completed", Some(output.clone())),
        anycode_core::TaskResult::Partial { success, .. } => ("partial", Some(success.clone())),
        anycode_core::TaskResult::Failure { error, .. } => ("failed", Some(error.clone())),
    }
}

fn trace_from_task_result(result: &anycode_core::TaskResult) -> Vec<ExecutionTraceEvent> {
    let (status, body) = match result {
        anycode_core::TaskResult::Success { output, .. } => ("completed", output.clone()),
        anycode_core::TaskResult::Partial { success, .. } => ("partial", success.clone()),
        anycode_core::TaskResult::Failure { error, .. } => ("failed", error.clone()),
    };
    vec![ExecutionTraceEvent::new(
        "task_end",
        "info",
        format!("task {status}"),
        body.clone(),
        serde_json::json!({ "status": status }),
    )]
}
