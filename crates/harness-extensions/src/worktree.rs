//! Explicit git worktree leases. No automatic merge, stash, reset or destructive
//! cleanup. Artifacts remain inspectable after cancellation. git is host allowlisted.
use crate::process::{run_host_command, CommandSpec};
use anycode_harness_core::{Error, Result, RunContext};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::Duration,
};
#[derive(Debug)]
pub struct WorktreeLease {
    pub path: PathBuf,
    pub repository: PathBuf,
    pub baseline: String,
}
pub async fn create(
    ctx: &RunContext,
    git: &Path,
    repository: &Path,
    private_root: &Path,
) -> Result<WorktreeLease> {
    ctx.capabilities().require("worktree.create")?;
    ctx.check()?;
    let repository = std::fs::canonicalize(repository)?;
    let root = std::fs::canonicalize(private_root)?;
    if root.starts_with(&repository) {
        return Err(Error::Denied(
            "worktree storage must be outside source repository".into(),
        ));
    }
    let path = root.join(ctx.id().to_string());
    if path.exists() {
        return Err(Error::Conflict("worktree already exists".into()));
    }
    let mut env = BTreeMap::new();
    env.insert("GIT_CONFIG_NOSYSTEM".into(), "1".into());
    env.insert("GIT_CONFIG_GLOBAL".into(), "/dev/null".into());
    env.insert("GIT_TERMINAL_PROMPT".into(), "0".into());
    env.insert("LC_ALL".into(), "C".into());
    let mut spec = CommandSpec {
        executable: git.into(),
        arguments: vec![
            "-c".into(),
            "core.hooksPath=/dev/null".into(),
            "rev-parse".into(),
            "--verify".into(),
            "HEAD".into(),
        ],
        cwd: repository.clone(),
        environment: env,
        timeout: Duration::from_secs(60),
        output_limit: 65536,
    };
    // This helper's /dev/null configuration is Unix-specific. Do not fake Windows isolation.
    #[cfg(not(unix))]
    return Err(Error::Unsupported(
        "native Windows worktree configuration pending".into(),
    ));
    let out = run_host_command(ctx, &spec, &[git.into()]).await?;
    let baseline = String::from_utf8(out.stdout)
        .map_err(|_| Error::Host("git returned non-UTF8 revision".into()))?
        .trim()
        .to_owned();
    if out.code != Some(0)
        || !matches!(baseline.len(), 40 | 64)
        || !baseline.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(Error::Host("git HEAD resolution failed".into()));
    }
    spec.arguments = vec![
        "-c".into(),
        "core.hooksPath=/dev/null".into(),
        "worktree".into(),
        "add".into(),
        "--detach".into(),
        path.to_string_lossy().into_owned(),
        baseline.clone(),
    ];
    let out = run_host_command(ctx, &spec, &[git.into()]).await?;
    if out.code != Some(0) {
        return Err(Error::Uncertain(
            "worktree creation failed; inspect git worktree list before cleanup".into(),
        ));
    }
    Ok(WorktreeLease {
        path,
        repository,
        baseline,
    })
}
