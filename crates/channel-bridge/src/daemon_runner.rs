//! In-process headless daemon: built-in cron scheduler.

use crate::app_config::{load_runtime_config, LoadOpts};
use crate::scheduler;
use crate::workspace::touch_project_dir;
use std::path::PathBuf;
use std::time::Duration;

fn default_working_dir() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Run the built-in cron scheduler until exit (holds `scheduler.lock`).
pub async fn run_builtin_scheduler() -> anyhow::Result<()> {
    let working_dir = default_working_dir();
    touch_project_dir(working_dir.clone());
    let config = load_runtime_config(LoadOpts {
        config_file: None,
        ignore_approval: false,
        workspace_overlay_dir: Some(working_dir.clone()),
        ..Default::default()
    })
    .await?;
    scheduler::run_builtin_scheduler(config, working_dir, Duration::from_secs(30), None).await
}
