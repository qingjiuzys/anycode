//! Host-only bounded subprocess primitive. No shell interpolation, ambient secrets,
//! detached tasks or claims of OS sandboxing. Do NOT expose this helper as a Tool.
//! Unix process groups are killed together; this is not a cgroup or job object.
use anycode_harness_core::{Error, Result, RunContext};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{io::AsyncReadExt, process::Command};
#[derive(Clone, Debug)]
pub struct CommandSpec {
    pub executable: PathBuf,
    pub arguments: Vec<String>,
    pub cwd: PathBuf,
    pub environment: BTreeMap<String, String>,
    pub timeout: Duration,
    pub output_limit: usize,
}
#[derive(Debug)]
pub struct ProcessResult {
    pub code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}
async fn read_bounded<R: tokio::io::AsyncRead + Unpin>(reader: R, limit: usize) -> Result<Vec<u8>> {
    let mut data = Vec::new();
    reader.take(limit as u64 + 1).read_to_end(&mut data).await?;
    if data.len() > limit {
        return Err(Error::Invalid("subprocess output limit".into()));
    }
    Ok(data)
}

fn kill_process_group(pid: Option<u32>) {
    let Some(pid) = pid else {
        return;
    };
    #[cfg(unix)]
    {
        // Negative PGID = the group created by process_group(0).
        let _ = std::process::Command::new("kill")
            .args(["-KILL", &format!("-{pid}")])
            .status();
    }
    #[cfg(not(unix))]
    let _ = pid;
}

pub async fn run_host_command(
    ctx: &RunContext,
    spec: &CommandSpec,
    allowed_executables: &[PathBuf],
) -> Result<ProcessResult> {
    ctx.check()?;
    if !spec.executable.is_absolute()
        || !spec.cwd.is_absolute()
        || spec.timeout.is_zero()
        || spec.timeout > Duration::from_secs(300)
        || spec.output_limit == 0
        || spec.output_limit > 16 * 1024 * 1024
        || spec.arguments.len() > 256
        || spec
            .arguments
            .iter()
            .any(|s| s.len() > 1024 * 1024 || s.contains('\0'))
    {
        return Err(Error::Invalid("subprocess configuration".into()));
    }
    let executable = std::fs::canonicalize(&spec.executable)?;
    let allowed = allowed_executables
        .iter()
        .filter_map(|p| std::fs::canonicalize(p).ok())
        .any(|p| p == executable);
    if !allowed || !executable.is_file() {
        return Err(Error::Denied("executable is not host allowlisted".into()));
    }
    let cwd = std::fs::canonicalize(&spec.cwd)?;
    if !cwd.is_dir() {
        return Err(Error::Invalid("subprocess cwd".into()));
    }
    let mut command = Command::new(executable);
    command
        .args(&spec.arguments)
        .current_dir(cwd)
        .env_clear()
        .envs(&spec.environment)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    {
        command.process_group(0);
    }
    let mut child = command.spawn()?;
    let pid = child.id();
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| Error::Host("stdout missing".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| Error::Host("stderr missing".into()))?;
    let read = async {
        tokio::try_join!(
            read_bounded(stdout, spec.output_limit),
            read_bounded(stderr, spec.output_limit)
        )
    };
    tokio::select! {
        biased;
        _ = ctx.cancelled() => {
            kill_process_group(pid);
            let _ = child.kill().await;
            let _ = child.wait().await;
            Err(Error::Cancelled)
        }
        _ = tokio::time::sleep(spec.timeout) => {
            kill_process_group(pid);
            let _ = child.kill().await;
            let _ = child.wait().await;
            Err(Error::Uncertain("subprocess timed out".into()))
        }
        output = read => {
            let (stdout, stderr) = output?;
            let status = child.wait().await?;
            Ok(ProcessResult {
                code: status.code(),
                stdout,
                stderr,
            })
        }
    }
}
pub fn absolute_file(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute() {
        return Err(Error::Invalid("absolute executable required".into()));
    }
    let p = std::fs::canonicalize(path)?;
    if !p.is_file() {
        return Err(Error::Invalid("executable is not a file".into()));
    }
    Ok(p)
}
