//! Goal-oriented retry loop.

use std::path::Path;

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
        let limit = self
            .spec
            .max_attempts_cap
            .or_else(|| (!self.spec.allow_infinite_retries).then_some(3));
        if let Some(cap) = limit {
            if progress.attempts >= cap {
                return false;
            }
        }
        true
    }

    pub fn update(&self, progress: &mut GoalProgress, result: &TaskResult, working_dir: &Path) {
        progress.attempts += 1;
        match result {
            TaskResult::Success { output, .. } => {
                progress.last_output = Some(output.clone());
                progress.completed = self
                    .spec
                    .done_when
                    .as_ref()
                    .map(|rule| self.satisfies_done_when(output, working_dir, rule))
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

    fn satisfies_done_when(&self, output: &str, working_dir: &Path, rule: &str) -> bool {
        if !output.contains(rule) {
            return false;
        }
        workspace_has_marker_line(working_dir, rule)
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
        if let Some(marker) = self.spec.done_when.as_deref() {
            let extra = format!(
                "{extra}\nCompletion requires the exact line `{marker}` in a README.md under the workspace (not only mentioned in chat)."
            );
            match &mut task.context.system_prompt_append {
                Some(existing) => existing.push_str(&extra),
                None => task.context.system_prompt_append = Some(extra),
            }
        } else {
            match &mut task.context.system_prompt_append {
                Some(existing) => existing.push_str(&extra),
                None => task.context.system_prompt_append = Some(extra),
            }
        }
        task
    }
}

fn workspace_has_marker_line(root: &Path, marker: &str) -> bool {
    const MAX_DEPTH: usize = 6;
    fn visit(dir: &Path, marker: &str, depth: usize) -> bool {
        if depth > MAX_DEPTH {
            return false;
        }
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return false,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if path.is_dir() {
                if should_skip_dir(&name) {
                    continue;
                }
                if visit(&path, marker, depth + 1) {
                    return true;
                }
            } else if name == "README.md" {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if content.lines().any(|line| line.trim() == marker) {
                        return true;
                    }
                }
            }
        }
        false
    }
    visit(root, marker, 0)
}

fn should_skip_dir(name: &str) -> bool {
    matches!(
        name,
        ".git" | "target" | "node_modules" | ".dart_tool" | "build" | "dist" | ".idea"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn should_continue_respects_max_attempts_cap_over_default_three() {
        let engine = GoalEngine::new(GoalSpec {
            objective: "x".into(),
            done_when: Some("DONE".into()),
            allow_infinite_retries: false,
            max_attempts_cap: Some(12),
        });
        let mut p = GoalProgress::default();
        p.attempts = 3;
        assert!(engine.should_continue(&p));
        p.attempts = 12;
        assert!(!engine.should_continue(&p));
    }

    #[test]
    fn done_when_requires_readme_line_not_chat_mention_only() {
        let dir = tempdir().unwrap();
        let engine = GoalEngine::new(GoalSpec {
            objective: "x".into(),
            done_when: Some("GOAL_ACCEPTANCE_OK".into()),
            allow_infinite_retries: false,
            max_attempts_cap: Some(5),
        });
        let output = "Next: mark GOAL_ACCEPTANCE_OK in README.md";
        assert!(!engine.satisfies_done_when(output, dir.path(), "GOAL_ACCEPTANCE_OK"));

        let readme = dir.path().join("README.md");
        fs::write(&readme, "# app\nGOAL_ACCEPTANCE_OK\n").unwrap();
        let output = "flutter analyze passed. GOAL_ACCEPTANCE_OK";
        assert!(engine.satisfies_done_when(output, dir.path(), "GOAL_ACCEPTANCE_OK"));
    }
}
