//! Git worktree isolation for nested `Agent` runs (Claude Code `isolation: "worktree"`).

use anycode_core::CoreError;
use std::fs;
use std::process::Command;
use tokio::task::spawn_blocking;
use uuid::Uuid;

/// Create a detached worktree under the OS temp dir; returns `(repo_root, worktree_abs)`.
pub async fn create_nested_worktree(cwd: &str) -> Result<(String, String), CoreError> {
    let cwd = cwd.to_string();
    spawn_blocking(move || {
        let out = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .current_dir(&cwd)
            .output()?;
        if !out.status.success() {
            return Err(CoreError::LLMError(format!(
                "isolation worktree requires a git repo (git rev-parse failed in {}): {}",
                cwd,
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        let repo_root = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let wt = std::env::temp_dir().join(format!("anycode-nested-wt-{}", Uuid::new_v4()));
        let wt_str = wt.to_string_lossy().to_string();
        let status = Command::new("git")
            .arg("-C")
            .arg(&repo_root)
            .args(["worktree", "add", &wt_str, "HEAD"])
            .status()?;
        if !status.success() {
            return Err(CoreError::LLMError(format!(
                "git worktree add failed (repo={}, path={})",
                repo_root, wt_str
            )));
        }
        Ok((repo_root, wt_str))
    })
    .await
    .map_err(|e| CoreError::Other(anyhow::anyhow!("worktree join: {}", e)))?
}

/// RAII: remove nested worktree when the nested `execute_task` scope ends.
pub(crate) struct NestedWorktreeGuard(pub Option<(String, String)>);

impl Drop for NestedWorktreeGuard {
    fn drop(&mut self) {
        let Some((repo_root, wt_path)) = self.0.take() else {
            return;
        };
        let _ = Command::new("git")
            .arg("-C")
            .arg(&repo_root)
            .args(["worktree", "remove", "--force", &wt_path])
            .status();
        let _ = fs::remove_dir_all(&wt_path);
    }
}
