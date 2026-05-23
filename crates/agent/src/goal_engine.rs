//! Goal-oriented retry loop.

use std::path::{Path, PathBuf};
use std::process::Command;

use anycode_core::{
    append_gate_log, DiskTaskOutput, GoalProgress, GoalSpec, Task, TaskId, TaskResult,
};

/// Optional sink for goal acceptance gates into `output.log`.
pub struct GoalGateLogger<'a> {
    pub disk: &'a DiskTaskOutput,
    pub task_id: TaskId,
}

impl<'a> GoalGateLogger<'a> {
    fn log(&self, name: &str, status: &str, command: &str, output: &str) {
        let _ = append_gate_log(self.disk, self.task_id, name, status, command, output);
    }
}

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
        if let Some(cap) = self.spec.max_attempts_cap {
            if progress.attempts >= cap {
                return false;
            }
        }
        true
    }

    pub fn update(
        &self,
        progress: &mut GoalProgress,
        result: &TaskResult,
        working_dir: &Path,
        gate_log: Option<&GoalGateLogger<'_>>,
    ) {
        progress.attempts += 1;
        match result {
            TaskResult::Success { output, .. } => {
                progress.last_output = Some(output.clone());
                progress.completed = match self.spec.done_when.as_ref() {
                    Some(rule) => {
                        match self.evaluate_done_when(output, working_dir, rule, gate_log) {
                            DoneWhenStatus::Complete => true,
                            DoneWhenStatus::Incomplete(reason) => {
                                progress.last_error = Some(reason);
                                false
                            }
                        }
                    }
                    None => true,
                };
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
        matches!(
            self.evaluate_done_when(output, working_dir, rule, None),
            DoneWhenStatus::Complete
        )
    }

    fn evaluate_done_when(
        &self,
        output: &str,
        working_dir: &Path,
        rule: &str,
        gate_log: Option<&GoalGateLogger<'_>>,
    ) -> DoneWhenStatus {
        if !output.contains(rule) {
            if let Some(g) = gate_log {
                g.log(
                    "assistant output marker",
                    "failed",
                    "done_when",
                    "output missing required substring",
                );
            }
            return DoneWhenStatus::Incomplete(format!(
                "assistant output must contain `{rule}` after real verification"
            ));
        }
        let scope = scope_search_root(working_dir, &self.spec.objective);
        let scoped = extract_test_subdir_from_text(&self.spec.objective).is_some();
        let readme_ok = if scoped {
            scope_readme_has_marker(&scope, rule)
        } else {
            scope_readme_has_marker(&scope, rule) || workspace_has_marker_line(&scope, rule)
        };
        if !readme_ok {
            if let Some(g) = gate_log {
                g.log(
                    &format!("README `{rule}`"),
                    "failed",
                    "readme_marker",
                    "marker line not found in scope README",
                );
            }
            let readme = scope.display();
            return DoneWhenStatus::Incomplete(format!(
                "add exact line `{rule}` to {readme}/README.md (engine-verified, not chat-only)"
            ));
        }
        if let Some(g) = gate_log {
            g.log(&format!("README `{rule}`"), "passed", "readme_marker", "");
        }
        if scope.join("pubspec.yaml").is_file() {
            return match flutter_project_verify(&scope, gate_log) {
                Ok(()) => DoneWhenStatus::Complete,
                Err(e) => DoneWhenStatus::Incomplete(e),
            };
        }
        if scope.join("Cargo.toml").is_file() {
            return match rust_project_verify(&scope, gate_log) {
                Ok(()) => DoneWhenStatus::Complete,
                Err(e) => DoneWhenStatus::Incomplete(e),
            };
        }
        DoneWhenStatus::Complete
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
            let flutter_note = if extract_test_subdir_from_text(&self.spec.objective).is_some() {
                "\nFor Flutter goals: `flutter analyze` and `flutter test` must exit 0; `test/widget_test.dart` must use `tester.tap` + `pumpAndSettle` on the onboarding button. The engine re-runs these checks — README/chat alone cannot complete the goal."
            } else {
                ""
            };
            let extra = format!(
                "{extra}\nCompletion requires the exact line `{marker}` in the goal directory README.md (not only mentioned in chat).{flutter_note}"
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

enum DoneWhenStatus {
    Complete,
    Incomplete(String),
}

/// Prefer a `test/...` path named in the goal objective so sibling apps do not satisfy `done_when`.
fn scope_search_root(working_dir: &Path, objective: &str) -> PathBuf {
    if let Some(rel) = extract_test_subdir_from_text(objective) {
        return working_dir.join(rel);
    }
    working_dir.to_path_buf()
}

fn extract_test_subdir_from_text(text: &str) -> Option<&str> {
    for token in text.split(|c: char| c.is_whitespace() || c == '(' || c == ')') {
        let token = token.trim_matches(|c: char| c == ',' || c == '.' || c == '`');
        if token.starts_with("test/") && !token.contains("..") && token.len() > "test/".len() {
            return Some(token);
        }
    }
    None
}

fn scope_readme_has_marker(scope: &Path, marker: &str) -> bool {
    let readme = scope.join("README.md");
    let Ok(content) = std::fs::read_to_string(&readme) else {
        return false;
    };
    content.lines().any(|line| line.trim() == marker)
}

fn flutter_project_verify(
    scope: &Path,
    gate_log: Option<&GoalGateLogger<'_>>,
) -> Result<(), String> {
    if std::env::var_os("ANYCODE_GOAL_SKIP_FLUTTER_VERIFY").is_some() {
        if let Some(g) = gate_log {
            g.log(
                "flutter verify",
                "skipped",
                "ANYCODE_GOAL_SKIP_FLUTTER_VERIFY",
                "",
            );
        }
        return Ok(());
    }
    match flutter_widget_test_requires_onboarding_tap(scope) {
        Ok(()) => {
            if let Some(g) = gate_log {
                g.log("widget_test onboarding tap", "passed", "static_check", "");
            }
        }
        Err(e) => {
            if let Some(g) = gate_log {
                g.log("widget_test onboarding tap", "failed", "static_check", &e);
            }
            return Err(e);
        }
    }
    match run_flutter_cmd(scope, &["analyze"]) {
        Ok(()) => {
            if let Some(g) = gate_log {
                g.log("flutter analyze", "passed", "flutter analyze", "");
            }
        }
        Err(e) => {
            if let Some(g) = gate_log {
                g.log("flutter analyze", "failed", "flutter analyze", &e);
            }
            return Err(format!(
                "flutter analyze failed in {}:\n{e}",
                scope.display()
            ));
        }
    }
    match run_flutter_cmd(scope, &["test"]) {
        Ok(()) => {
            if let Some(g) = gate_log {
                g.log("flutter test", "passed", "flutter test", "");
            }
            Ok(())
        }
        Err(e) => {
            if let Some(g) = gate_log {
                g.log("flutter test", "failed", "flutter test", &e);
            }
            Err(format!("flutter test failed in {}:\n{e}", scope.display()))
        }
    }
}

/// Ensures widget tests exercise onboarding navigation (catches missing root Provider).
fn flutter_widget_test_requires_onboarding_tap(scope: &Path) -> Result<(), String> {
    let path = scope.join("test/widget_test.dart");
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("missing or unreadable {}: {e}", path.display()))?;
    let has_tap = content.contains("tester.tap") || content.contains(".tap(find");
    let has_settle = content.contains("pumpAndSettle");
    if !has_tap || !has_settle {
        return Err(format!(
            "{} must tap the onboarding CTA (tester.tap + pumpAndSettle) so Provider/runtime bugs are caught",
            path.display()
        ));
    }
    Ok(())
}

fn rust_project_verify(scope: &Path, gate_log: Option<&GoalGateLogger<'_>>) -> Result<(), String> {
    if std::env::var_os("ANYCODE_GOAL_SKIP_RUST_VERIFY").is_some() {
        if let Some(g) = gate_log {
            g.log(
                "rust verify",
                "skipped",
                "ANYCODE_GOAL_SKIP_RUST_VERIFY",
                "",
            );
        }
        return Ok(());
    }
    let gates: [(&str, &[&str]); 3] = [
        ("cargo fmt", &["fmt", "--", "--check"]),
        ("cargo clippy", &["clippy", "--", "-D", "warnings"]),
        ("cargo test", &["test", "--workspace"]),
    ];
    for (name, args) in gates {
        match run_cargo_cmd(scope, args) {
            Ok(()) => {
                if let Some(g) = gate_log {
                    g.log(name, "passed", name, "");
                }
            }
            Err(e) => {
                if let Some(g) = gate_log {
                    g.log(name, "failed", name, &e);
                }
                return Err(format!("{name} failed in {}:\n{e}", scope.display()));
            }
        }
    }
    Ok(())
}

fn run_cargo_cmd(scope: &Path, args: &[&str]) -> Result<(), String> {
    let output = Command::new("cargo")
        .args(args)
        .current_dir(scope)
        .output()
        .map_err(|e| format!("failed to spawn cargo {args:?}: {e}"))?;
    if output.status.success() {
        return Ok(());
    }
    let mut msg = String::from_utf8_lossy(&output.stderr).into_owned();
    if msg.trim().is_empty() {
        msg = String::from_utf8_lossy(&output.stdout).into_owned();
    }
    const MAX: usize = 6000;
    if msg.len() > MAX {
        msg.truncate(MAX);
        msg.push_str("\n…(truncated)");
    }
    Err(msg)
}

fn run_flutter_cmd(scope: &Path, args: &[&str]) -> Result<(), String> {
    let output = Command::new("flutter")
        .args(args)
        .current_dir(scope)
        .output()
        .map_err(|e| format!("failed to spawn flutter {args:?}: {e}"))?;
    if output.status.success() {
        return Ok(());
    }
    let mut msg = String::from_utf8_lossy(&output.stderr).into_owned();
    if msg.trim().is_empty() {
        msg = String::from_utf8_lossy(&output.stdout).into_owned();
    }
    const MAX: usize = 6000;
    if msg.len() > MAX {
        msg.truncate(MAX);
        msg.push_str("\n…(truncated)");
    }
    Err(msg)
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
    fn should_continue_unlimited_when_no_cap() {
        let engine = GoalEngine::new(GoalSpec {
            objective: "Build in test/app-99".into(),
            done_when: Some("DONE".into()),
            allow_infinite_retries: true,
            max_attempts_cap: None,
        });
        let mut p = GoalProgress::default();
        p.attempts = 50;
        assert!(engine.should_continue(&p));
    }

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

    #[test]
    fn done_when_scoped_to_objective_test_subdir_not_sibling_readme() {
        let dir = tempdir().unwrap();
        let app01 = dir.path().join("test/app-01");
        let app02 = dir.path().join("test/app-02");
        fs::create_dir_all(&app01).unwrap();
        fs::create_dir_all(&app02).unwrap();
        fs::write(app01.join("README.md"), "GOAL_ACCEPTANCE_OK\n").unwrap();
        fs::write(app02.join("README.md"), "# wip\n").unwrap();

        let engine = GoalEngine::new(GoalSpec {
            objective: "Build MVP in test/app-02 only".into(),
            done_when: Some("GOAL_ACCEPTANCE_OK".into()),
            allow_infinite_retries: false,
            max_attempts_cap: Some(12),
        });
        let output = "Done GOAL_ACCEPTANCE_OK";
        assert!(!engine.satisfies_done_when(output, dir.path(), "GOAL_ACCEPTANCE_OK"));

        fs::write(app02.join("README.md"), "GOAL_ACCEPTANCE_OK\n").unwrap();
        assert!(engine.satisfies_done_when(output, dir.path(), "GOAL_ACCEPTANCE_OK"));
    }

    #[test]
    fn flutter_scope_requires_analyze_and_test_not_readme_alone() {
        let dir = tempdir().unwrap();
        let app02 = dir.path().join("test/app-02");
        fs::create_dir_all(&app02).unwrap();
        fs::write(app02.join("README.md"), "GOAL_ACCEPTANCE_OK\n").unwrap();
        fs::write(
            app02.join("pubspec.yaml"),
            "name: fake\nenvironment:\n  sdk: '>=3.0.0 <4.0.0'\n",
        )
        .unwrap();

        let engine = GoalEngine::new(GoalSpec {
            objective: "Growth MVP in test/app-02".into(),
            done_when: Some("GOAL_ACCEPTANCE_OK".into()),
            allow_infinite_retries: true,
            max_attempts_cap: None,
        });
        let output = "GOAL_ACCEPTANCE_OK";
        assert!(!engine.satisfies_done_when(output, dir.path(), "GOAL_ACCEPTANCE_OK"));

        std::env::set_var("ANYCODE_GOAL_SKIP_FLUTTER_VERIFY", "1");
        assert!(engine.satisfies_done_when(output, dir.path(), "GOAL_ACCEPTANCE_OK"));
        std::env::remove_var("ANYCODE_GOAL_SKIP_FLUTTER_VERIFY");
    }
}
