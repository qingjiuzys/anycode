//! Built-in cron scheduler and headless daemon entrypoints.
//!
//! Third-party IM channel bridges (WeChat / Telegram / Discord) were removed;
//! conversations happen in the local Workbench only. The scheduler keeps
//! running persisted cron jobs from `~/.anycode/tasks/orchestration.json`.

mod app_config;
mod artifact_summary;
mod builtin_agents;
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

pub use daemon_runner::run_builtin_scheduler;
