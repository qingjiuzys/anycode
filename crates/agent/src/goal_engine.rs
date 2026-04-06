//! Goal-oriented retry loop.

use anycode_core::{GoalProgress, GoalSpec, Task, TaskResult};

#[derive(Debug, Clone)]
pub struct GoalEngine {
    pub spec: GoalSpec,
}

impl GoalEngine {
    pub fn new(spec: GoalSpec) -> Self {
        Self { spec }
    }

    pub fn should_continue(&self, progress: &GoalProgress) -> bool {
        if progress.completed {
            return false;
        }
        self.spec.allow_infinite_retries || progress.attempts < 3
    }

    pub fn update(&self, progress: &mut GoalProgress, result: &TaskResult) {
        progress.attempts += 1;
        match result {
            TaskResult::Success { output, .. } => {
                progress.last_output = Some(output.clone());
                progress.completed = self
                    .spec
                    .done_when
                    .as_ref()
                    .map(|rule| output.contains(rule))
                    .unwrap_or(true);
            }
            TaskResult::Failure { error, details } => {
                progress.last_error = Some(details.clone().unwrap_or_else(|| error.clone()));
            }
            TaskResult::Partial { success, remaining } => {
                progress.last_output = Some(success.clone());
                progress.last_error = Some(remaining.clone());
            }
        }
    }

    pub fn prime_task(&self, mut task: Task) -> Task {
        let extra = format!(
            "\n\n## Goal\nObjective: {}\nDone when: {}",
            self.spec.objective,
            self.spec
                .done_when
                .clone()
                .unwrap_or_else(|| "the objective is fully satisfied".to_string())
        );
        match &mut task.context.system_prompt_append {
            Some(existing) => existing.push_str(&extra),
            None => task.context.system_prompt_append = Some(extra),
        }
        task
    }
}
