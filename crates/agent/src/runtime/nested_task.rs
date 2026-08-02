//! Nested sub-agent execution (`SubAgentExecutor`).

use super::nested_worktree;
use super::AgentRuntime;
use anycode_core::prelude::*;
use async_trait::async_trait;
use std::collections::HashMap;
use uuid::Uuid;

/// 子代理默认追加提示词（D6：对齐 Claude Code `appendSubagentSystemPrompt`）。
/// 提示子代理聚焦任务、避免重复上下文与冗长回执。
const SUBAGENT_SYSTEM_APPEND: &str = "\
你是被父任务派出的子代理，只完成父任务交给你的这个任务。\
请保持专注：不要重复描述父任务已提供的上下文，不要主动扩大任务范围。\
输出只包含任务要求的结果；完成后用简短一句话总结即可。";

#[async_trait]
impl SubAgentExecutor for AgentRuntime {
    async fn run_nested_task(&self, invoke: NestedTaskInvoke) -> Result<NestedTaskRun, CoreError> {
        let mut wd = invoke.working_directory;
        let wt_roots = {
            let iso = invoke
                .isolation
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty());
            if iso.is_some_and(|s| s.eq_ignore_ascii_case("worktree")) {
                let (repo, wt) = nested_worktree::create_nested_worktree(&wd).await?;
                wd = wt.clone();
                Some((repo, wt))
            } else {
                None
            }
        };

        let task = Task {
            id: invoke.task_id.unwrap_or_else(Uuid::new_v4),
            agent_type: invoke.agent_type,
            prompt: invoke.prompt,
            context: TaskContext {
                session_id: Uuid::new_v4(),
                working_directory: wd,
                environment: HashMap::new(),
                user_id: None,
                system_prompt_append: Some(SUBAGENT_SYSTEM_APPEND.to_string()),
                context_injections: invoke.context_injections.clone(),
                nested_model_override: invoke.model.clone(),
                nested_worktree_repo_root: wt_roots.as_ref().map(|(r, _)| r.clone()),
                nested_worktree_path: wt_roots.as_ref().map(|(_, p)| p.clone()),
                nested_cancel: invoke.cancel.clone(),
                channel_progress_tx: None,
                live_trace_tx: None,
                tool_deny_names: invoke.tool_deny_names.clone(),
                tool_deny_prefixes: invoke.tool_deny_prefixes.clone(),
                user_vision_images: vec![],
                budget: nested_budget_from_env(),
                loop_limits: anycode_core::resolve_agent_loop_limits(None, None),
                chat_turn: anycode_core::current_chat_turn(),
            },
            created_at: chrono::Utc::now(),
        };
        let task_id = task.id;
        let result = self.execute_task(task).await?;
        Ok(NestedTaskRun { task_id, result })
    }
}

fn nested_budget_from_env() -> TaskBudget {
    TaskBudget {
        token_budget_total: std::env::var("ANYCODE_NESTED_TOKEN_BUDGET")
            .or_else(|_| std::env::var("ANYCODE_TASK_TOKEN_BUDGET"))
            .ok()
            .and_then(|v| v.parse::<u32>().ok()),
        cost_budget_cny: std::env::var("ANYCODE_NESTED_COST_BUDGET_CNY")
            .or_else(|_| std::env::var("ANYCODE_TASK_COST_BUDGET_CNY"))
            .ok()
            .and_then(|v| v.parse::<f64>().ok()),
        max_duration_secs: std::env::var("ANYCODE_NESTED_MAX_DURATION_SECS")
            .or_else(|_| std::env::var("ANYCODE_TASK_MAX_DURATION_SECS"))
            .ok()
            .and_then(|v| v.parse::<u64>().ok()),
        ..TaskBudget::default()
    }
}
