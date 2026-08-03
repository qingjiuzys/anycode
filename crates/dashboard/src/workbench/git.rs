use anyhow::{Context, Result};
use serde::Serialize;
use std::path::Path;
use std::process::Output;

#[derive(Debug, Clone, Serialize)]
pub struct GitStatusSummary {
    pub is_repo: bool,
    pub branch: Option<String>,
    pub insertions: u64,
    pub deletions: u64,
    pub changed_files: u32,
    pub ahead: u32,
    pub behind: u32,
    pub has_upstream: bool,
    pub has_changes: bool,
}

fn run_git(cwd: &Path, args: &[&str]) -> Result<Output> {
    std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .with_context(|| format!("spawn git {}", args.join(" ")))
}

fn git_ok(output: &Output) -> bool {
    output.status.success()
}

fn git_stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn git_stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).trim().to_string()
}

pub fn is_git_repo(root: &Path) -> bool {
    if root.join(".git").exists() {
        return true;
    }
    run_git(root, &["rev-parse", "--is-inside-work-tree"])
        .ok()
        .filter(|o| git_ok(o) && git_stdout(o) == "true")
        .is_some()
}

/// Parse `git diff --shortstat` style summary fragments.
pub fn parse_shortstat_line(line: &str) -> (u32, u64, u64) {
    let mut files = 0u32;
    let mut insertions = 0u64;
    let mut deletions = 0u64;
    for part in line.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let mut tokens = part.split_whitespace();
        let Some(n) = tokens.next().and_then(|t| t.parse::<u64>().ok()) else {
            continue;
        };
        let word = tokens.next().unwrap_or("");
        if word.starts_with("file") {
            files = n.min(u32::MAX as u64) as u32;
        } else if word.starts_with("insertion") {
            insertions = n;
        } else if word.starts_with("deletion") {
            deletions = n;
        }
    }
    (files, insertions, deletions)
}

fn merge_shortstat(a: &str, b: &str) -> (u32, u64, u64) {
    let (f1, i1, d1) = parse_shortstat_line(a);
    let (f2, i2, d2) = parse_shortstat_line(b);
    (f1.saturating_add(f2), i1 + i2, d1 + d2)
}

pub fn git_status(root: &Path) -> Result<GitStatusSummary> {
    if !is_git_repo(root) {
        return Ok(GitStatusSummary {
            is_repo: false,
            branch: None,
            insertions: 0,
            deletions: 0,
            changed_files: 0,
            ahead: 0,
            behind: 0,
            has_upstream: false,
            has_changes: false,
        });
    }

    let branch = run_git(root, &["branch", "--show-current"])
        .ok()
        .filter(|o| git_ok(o))
        .map(|o| git_stdout(&o))
        .filter(|s| !s.is_empty());

    let unstaged = run_git(root, &["diff", "--shortstat"])?;
    let staged = run_git(root, &["diff", "--cached", "--shortstat"])?;
    let unstaged_text = if git_ok(&unstaged) {
        git_stdout(&unstaged)
    } else {
        String::new()
    };
    let staged_text = if git_ok(&staged) {
        git_stdout(&staged)
    } else {
        String::new()
    };
    let (mut changed_files, insertions, deletions) = merge_shortstat(&unstaged_text, &staged_text);

    let porcelain = run_git(root, &["status", "--porcelain"])?;
    let mut untracked = 0u32;
    if git_ok(&porcelain) {
        for line in git_stdout(&porcelain).lines() {
            if line.starts_with("??") {
                untracked += 1;
            }
        }
    }
    changed_files = changed_files.saturating_add(untracked);

    let upstream = run_git(root, &["rev-parse", "--abbrev-ref", "@{upstream}"]);
    let has_upstream = upstream
        .as_ref()
        .ok()
        .filter(|o| git_ok(o))
        .map(|o| !git_stdout(o).is_empty())
        .unwrap_or(false);

    let (ahead, behind) = if has_upstream {
        let counts = run_git(
            root,
            &["rev-list", "--left-right", "--count", "@{upstream}...HEAD"],
        )?;
        if git_ok(&counts) {
            let stdout = git_stdout(&counts);
            let parts: Vec<&str> = stdout.split_whitespace().collect();
            let behind = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
            let ahead = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
            (ahead, behind)
        } else {
            (0, 0)
        }
    } else {
        (0, 0)
    };

    let has_changes = changed_files > 0 || insertions > 0 || deletions > 0;

    Ok(GitStatusSummary {
        is_repo: true,
        branch,
        insertions,
        deletions,
        changed_files,
        ahead,
        behind,
        has_upstream,
        has_changes,
    })
}

pub fn git_commit_all(root: &Path, message: &str) -> Result<()> {
    let add = run_git(root, &["add", "-A"])?;
    if !git_ok(&add) {
        anyhow::bail!("git add failed: {}", git_stderr(&add));
    }
    let commit = run_git(root, &["commit", "-m", message])?;
    if !git_ok(&commit) {
        anyhow::bail!("git commit failed: {}", git_stderr(&commit));
    }
    Ok(())
}

pub fn git_push(root: &Path) -> Result<String> {
    let push = run_git(root, &["push"])?;
    if !git_ok(&push) {
        anyhow::bail!("git push failed: {}", git_stderr(&push));
    }
    Ok(git_stderr(&push))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_shortstat_examples() {
        let (f, i, d) =
            parse_shortstat_line("3 files changed, 2154 insertions(+), 233 deletions(-)");
        assert_eq!(f, 3);
        assert_eq!(i, 2154);
        assert_eq!(d, 233);
        let (f2, i2, d2) = parse_shortstat_line("1 file changed, 10 insertions(+)");
        assert_eq!(f2, 1);
        assert_eq!(i2, 10);
        assert_eq!(d2, 0);
    }

    #[test]
    fn merge_shortstat_sums() {
        let (f, i, d) = merge_shortstat(
            "2 files changed, 10 insertions(+), 1 deletion(-)",
            "1 file changed, 5 insertions(+), 2 deletions(-)",
        );
        assert_eq!(f, 3);
        assert_eq!(i, 15);
        assert_eq!(d, 3);
    }
}
