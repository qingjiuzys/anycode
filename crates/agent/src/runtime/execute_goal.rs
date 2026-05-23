//! Goal task execution

use super::AgentRuntime;
use crate::goal_engine::GoalEngine;
use anycode_core::prelude::*;
use std::path::PathBuf;

impl AgentRuntime {
    pub async fn execute_goal_task(
        &self,
        task: Task,
        spec: GoalSpec,
    ) -> Result<(TaskResult, GoalProgress), CoreError> {
        let engine = GoalEngine::new(spec);
        let mut progress = GoalProgress::default();
        let mut current_task = engine.prime_task(task);
        let mut last_result = TaskResult::Failure {
            error: "goal did not run".to_string(),
            details: None,
        };

        let working_dir = PathBuf::from(&current_task.context.working_directory);
        while engine.should_continue(&progress) {
            let result = self.execute_task(current_task.clone()).await?;
            engine.update(&mut progress, &result, &working_dir, None);
            last_result = result.clone();
            if progress.completed {
                break;
            }
            current_task.context.context_injections = vec![format!(
                "## Goal Retry Context\nPrevious attempt count: {}.\nLast error: {:?}\nLast output: {:?}",
                progress.attempts, progress.last_error, progress.last_output
            )];
        }

        Ok((last_result, progress))
    }
}
