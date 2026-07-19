//! Shared async shell runners for Bash / PowerShell (timeout + optional background).

use anycode_core::{CoreError, DiskTaskOutput};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use uuid::Uuid;

pub const DEFAULT_TIMEOUT_MS: u64 = 120_000;
pub const MIN_TIMEOUT_MS: u64 = 1_000;
pub const MAX_TIMEOUT_MS: u64 = 600_000;

pub fn clamp_timeout_ms(raw: u64) -> u64 {
    raw.clamp(MIN_TIMEOUT_MS, MAX_TIMEOUT_MS)
}

pub fn tasks_disk() -> Option<DiskTaskOutput> {
    dirs::home_dir().map(|h| DiskTaskOutput::new(h.join(".anycode").join("tasks")))
}

pub fn nested_output_log_path(task_id: Uuid) -> Option<String> {
    tasks_disk().map(|d| d.output_path(task_id).to_string_lossy().into_owned())
}

#[derive(Debug)]
pub struct ShellCapture {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
}

fn apply_cwd(cmd: &mut Command, wd: Option<&Path>) {
    if let Some(wd) = wd {
        cmd.current_dir(wd);
    }
}

#[cfg(unix)]
fn configure_process_group(cmd: &mut Command) {
    // Own process group so timeout/cancel can kill the whole tree (dev servers, etc.).
    cmd.process_group(0);
}

#[cfg(not(unix))]
fn configure_process_group(_cmd: &mut Command) {}

fn kill_process_group(child: &mut tokio::process::Child) {
    #[cfg(unix)]
    if let Some(pid) = child.id() {
        // Negative PGID = signal entire group started with process_group(0).
        let _ = std::process::Command::new("kill")
            .args(["-KILL", &format!("-{pid}")])
            .status();
    }
    let _ = child.start_kill();
}

/// Run a program with wall-clock timeout; stdout/stderr captured into memory.
pub async fn run_foreground(
    program: &str,
    args: &[&str],
    cwd: Option<&Path>,
    timeout_ms: u64,
) -> Result<ShellCapture, CoreError> {
    let timeout = Duration::from_millis(clamp_timeout_ms(timeout_ms));
    let mut cmd = Command::new(program);
    cmd.args(args);
    cmd.kill_on_drop(true);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    apply_cwd(&mut cmd, cwd);
    configure_process_group(&mut cmd);

    let mut child = cmd.spawn().map_err(CoreError::IoError)?;
    let mut stdout_pipe = child.stdout.take();
    let mut stderr_pipe = child.stderr.take();

    let stdout_task = tokio::spawn(async move {
        let mut buf = Vec::new();
        if let Some(mut r) = stdout_pipe.take() {
            let _ = r.read_to_end(&mut buf).await;
        }
        buf
    });
    let stderr_task = tokio::spawn(async move {
        let mut buf = Vec::new();
        if let Some(mut r) = stderr_pipe.take() {
            let _ = r.read_to_end(&mut buf).await;
        }
        buf
    });

    match tokio::time::timeout(timeout, child.wait()).await {
        Ok(Ok(status)) => {
            let stdout = stdout_task.await.unwrap_or_default();
            let stderr = stderr_task.await.unwrap_or_default();
            Ok(ShellCapture {
                stdout: String::from_utf8_lossy(&stdout).into_owned(),
                stderr: String::from_utf8_lossy(&stderr).into_owned(),
                exit_code: status.code(),
                timed_out: false,
            })
        }
        Ok(Err(e)) => {
            stdout_task.abort();
            stderr_task.abort();
            Err(CoreError::IoError(e))
        }
        Err(_) => {
            kill_process_group(&mut child);
            let _ = child.wait().await;
            stdout_task.abort();
            stderr_task.abort();
            Ok(ShellCapture {
                stdout: String::new(),
                stderr: String::new(),
                exit_code: None,
                timed_out: true,
            })
        }
    }
}

pub struct BackgroundChild {
    pub log_path: PathBuf,
    pub child: tokio::process::Child,
}

/// Spawn a long-running shell with stdio appended to `~/.anycode/tasks/<id>/output.log`.
pub fn spawn_background_child(
    program: &str,
    args: &[&str],
    cwd: Option<&Path>,
    task_id: Uuid,
) -> Result<BackgroundChild, CoreError> {
    let disk = tasks_disk().ok_or_else(|| {
        CoreError::IoError(std::io::Error::other(
            "HOME unavailable for background shell log path",
        ))
    })?;
    let log_path = disk.ensure_initialized(task_id)?;
    let out_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(CoreError::IoError)?;
    let err_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(CoreError::IoError)?;

    let mut cmd = Command::new(program);
    cmd.args(args);
    cmd.kill_on_drop(true);
    cmd.stdout(Stdio::from(out_file));
    cmd.stderr(Stdio::from(err_file));
    cmd.stdin(Stdio::null());
    apply_cwd(&mut cmd, cwd);
    configure_process_group(&mut cmd);

    let child = cmd.spawn().map_err(CoreError::IoError)?;
    Ok(BackgroundChild { log_path, child })
}

pub fn kill_background_child(child: &mut tokio::process::Child) {
    kill_process_group(child);
}
