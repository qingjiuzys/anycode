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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GitChangeKind {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
    TypeChanged,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitFileChange {
    pub path: String,
    /// Original path for renames (otherwise equal to `path`).
    pub old_path: String,
    pub kind: GitChangeKind,
    pub staged: bool,
    /// Character-style status code from `git status --porcelain` (e.g. "M", "A").
    pub status: String,
    pub insertions: u32,
    pub deletions: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitFileDiff {
    pub path: String,
    pub kind: GitChangeKind,
    pub diff: String,
    pub insertions: u32,
    pub deletions: u32,
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

/// Parse one `git status --porcelain` line into a file change.
/// Format: `XY path` (with possible ` -> ` for renames).
fn parse_porcelain_line(line: &str) -> Option<GitFileChange> {
    let bytes = line.as_bytes();
    if bytes.len() < 4 {
        return None;
    }
    let x = bytes[0] as char;
    let y = bytes[1] as char;
    let path_part = &line[3..];

    let (kind, path, old_path) = if x == 'R' || y == 'R' {
        let (old, new) = path_part.split_once(" -> ")?;
        (
            GitChangeKind::Renamed,
            new.trim().to_string(),
            old.trim().to_string(),
        )
    } else if x == '?' {
        (
            GitChangeKind::Untracked,
            path_part.to_string(),
            String::new(),
        )
    } else {
        let kind = match (x, y) {
            ('A', _) => GitChangeKind::Added,
            ('D', _) | (_, 'D') => GitChangeKind::Deleted,
            ('T', _) | (_, 'T') => GitChangeKind::TypeChanged,
            _ => GitChangeKind::Modified,
        };
        (kind, path_part.to_string(), String::new())
    };

    Some(GitFileChange {
        path,
        old_path,
        kind,
        staged: x != '?' && x != ' ',
        status: format!("{}{}", x, y),
        insertions: 0,
        deletions: 0,
    })
}

/// List changed files in the working tree, like an IDE source-control view.
/// Groups staged + unstaged changes and reports per-file +/- counts.
pub fn git_changes(root: &Path) -> Result<Vec<GitFileChange>> {
    if !is_git_repo(root) {
        return Ok(Vec::new());
    }
    let porcelain = run_git(root, &["status", "--porcelain"])?;
    if !git_ok(&porcelain) {
        anyhow::bail!("git status failed: {}", git_stderr(&porcelain));
    }
    let mut changes: Vec<GitFileChange> = git_stdout(&porcelain)
        .lines()
        .filter_map(parse_porcelain_line)
        .collect();

    // Per-file numstat: `git diff --numstat` (unstaged) + `git diff --cached --numstat` (staged).
    for args in [
        &["diff", "--numstat"][..],
        &["diff", "--cached", "--numstat"][..],
    ] {
        let out = run_git(root, args)?;
        if !git_ok(&out) {
            continue;
        }
        for line in git_stdout(&out).lines() {
            let mut parts = line.splitn(3, '\t');
            let (Some(add), Some(del), Some(path)) = (parts.next(), parts.next(), parts.next())
            else {
                continue;
            };
            let path = path.trim();
            let add = add.trim().parse::<u32>().unwrap_or(0);
            let del = del.trim().parse::<u32>().unwrap_or(0);
            if let Some(c) = changes.iter_mut().find(|c| c.path == path) {
                c.insertions += add;
                c.deletions += del;
            }
        }
    }

    Ok(changes)
}

/// Produce the unified diff text for a single changed file (staged + unstaged
/// combined). Untracked files render as an all-additions diff.
pub fn git_file_diff(root: &Path, path: &str, kind: GitChangeKind) -> Result<GitFileDiff> {
    let mut insertions = 0u32;
    let mut deletions = 0u32;
    let mut diff = String::new();

    if kind == GitChangeKind::Untracked {
        // No git diff exists for untracked files; show the full file as added.
        let full = match std::fs::read_to_string(root.join(path)) {
            Ok(s) => s,
            Err(e) => anyhow::bail!("read untracked file {}: {}", path, e),
        };
        let line_count = full.lines().count() as u32;
        insertions = line_count;
        diff.push_str(&format!("diff --git a/{path} b/{path}\n"));
        diff.push_str("new file mode 100644\n");
        diff.push_str("--- /dev/null\n");
        diff.push_str(&format!("+++ b/{path}\n"));
        diff.push_str(&format!("@@ -0,0 +1,{} @@\n", line_count.max(1)));
        for line in full.lines() {
            diff.push_str(&format!("+{}\n", line));
        }
        if full.is_empty() {
            diff.push_str("+\n");
        }
    } else {
        for args in [
            &["diff", "--", path][..],
            &["diff", "--cached", "--", path][..],
        ] {
            let out = run_git(root, args)?;
            if git_ok(&out) {
                diff.push_str(&String::from_utf8_lossy(&out.stdout));
            }
        }
        // Tally +/- from the combined diff text.
        for line in diff.lines() {
            if line.starts_with('+') && !line.starts_with("+++") {
                insertions += 1;
            } else if line.starts_with('-') && !line.starts_with("---") {
                deletions += 1;
            }
        }
    }

    Ok(GitFileDiff {
        path: path.to_string(),
        kind,
        diff,
        insertions,
        deletions,
    })
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

    #[test]
    fn parse_porcelain_basic() {
        let m = parse_porcelain_line(" M src/main.rs").unwrap();
        assert_eq!(m.kind, GitChangeKind::Modified);
        assert_eq!(m.path, "src/main.rs");
        assert!(!m.staged);
        assert_eq!(m.status, " M");

        let a = parse_porcelain_line("A  new.rs").unwrap();
        assert_eq!(a.kind, GitChangeKind::Added);
        assert_eq!(a.path, "new.rs");
        assert!(a.staged);

        let u = parse_porcelain_line("?? untracked.txt").unwrap();
        assert_eq!(u.kind, GitChangeKind::Untracked);
        assert_eq!(u.path, "untracked.txt");
    }

    #[test]
    fn parse_porcelain_rename() {
        let r = parse_porcelain_line("R  old.rs -> new.rs").unwrap();
        assert_eq!(r.kind, GitChangeKind::Renamed);
        assert_eq!(r.old_path, "old.rs");
        assert_eq!(r.path, "new.rs");
        assert!(r.staged);
    }

    #[test]
    fn parse_porcelain_deleted_staged_vs_unstaged() {
        // 已暂存删除：X='D'，路径即被删文件。
        let staged = parse_porcelain_line("D  gone.rs").unwrap();
        assert_eq!(staged.kind, GitChangeKind::Deleted);
        assert_eq!(staged.path, "gone.rs");
        assert!(staged.staged);
        assert_eq!(staged.status, "D ");

        // 未暂存删除：X=' '、Y='D' → 转换规则 `(_, 'D')` 也归为 Deleted。
        let unstaged = parse_porcelain_line(" D gone.rs").unwrap();
        assert_eq!(unstaged.kind, GitChangeKind::Deleted);
        assert_eq!(unstaged.path, "gone.rs");
        assert!(!unstaged.staged);
        assert_eq!(unstaged.status, " D");
    }

    #[test]
    fn parse_porcelain_type_changed_both_columns() {
        let staged = parse_porcelain_line("T  link").unwrap();
        assert_eq!(staged.kind, GitChangeKind::TypeChanged);
        assert!(staged.staged);

        let unstaged = parse_porcelain_line(" T link").unwrap();
        assert_eq!(unstaged.kind, GitChangeKind::TypeChanged);
        assert!(!unstaged.staged);
    }

    #[test]
    fn parse_porcelain_combined_xy() {
        // AM：工作区中新增后又修改 → X='A'，规则 `('A', _)` 归为 Added 且已暂存。
        let am = parse_porcelain_line("AM staged_and_modified.rs").unwrap();
        assert_eq!(am.kind, GitChangeKind::Added);
        assert_eq!(am.path, "staged_and_modified.rs");
        assert!(am.staged);

        // MM：暂存后又未暂存修改 → 落 default 归为 Modified。
        let mm = parse_porcelain_line("MM both.rs").unwrap();
        assert_eq!(mm.kind, GitChangeKind::Modified);
        assert!(mm.staged);
    }

    #[test]
    fn parse_porcelain_short_line_returns_none() {
        assert!(parse_porcelain_line("").is_none());
        assert!(parse_porcelain_line(" M").is_none()); // 不足 4 字节（缺路径）
    }
}
