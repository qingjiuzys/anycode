//! anyCode CLI entry.

mod app_config;
mod artifact_summary;
mod ask_user_host;
mod bootstrap;
mod builtin_agents;
mod channel_task;
mod channels;
mod cli_args;
mod commands;
mod copilot_auth;
mod cron_failure;
mod eval_manifest;
mod i18n;
mod md_render;
mod repl;
mod repl_banner;
mod repl_clipboard;
mod scheduler;
mod session_transcript_export;
mod setup_memory;
mod slash_commands;
mod task_builders;
mod tasks;
mod term;
mod tool_policy;
mod vision_prompt;
mod workbench;
mod workspace;

pub(crate) use workbench::{dashboard_record, workbench_approval};

#[tokio::main]
async fn main() {
    if let Err(err) = commands::dispatch::run_cli().await {
        commands::cli_error::emit_and_exit(&err);
    }
}
