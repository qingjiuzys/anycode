//! IM channel bridges, built-in cron scheduler, and headless daemon entrypoints.

mod app_config;
mod artifact_summary;
mod builtin_agents;
pub mod channel_task;
pub mod channels;
pub mod cron_failure;
pub mod daemon_runner;
mod i18n;
pub mod scheduler;
mod task_builders;
pub mod tasks;
mod tool_policy;
mod workbench;
mod workflow_validate;
mod workspace;

pub use daemon_runner::{
    run_builtin_scheduler, run_discord_bridge, run_telegram_bridge, run_wechat_bridge,
};
