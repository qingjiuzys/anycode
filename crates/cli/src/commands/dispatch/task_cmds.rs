//! Task execution and interactive REPL dispatch.

use super::{load_config_for_session, resolve_working_dir};
use crate::app_config::{load_runtime_config, LoadOpts};
use crate::cli_args::Commands;
use crate::{cli_args, scheduler, tasks, workspace};
use std::io::IsTerminal;

pub(super) async fn dispatch(
    command: Commands,
    config: Option<std::path::PathBuf>,
    ignore_approval: bool,
) -> anyhow::Result<()> {
    match command {
        Commands::Scheduler {
            directory,
            reload_secs,
        } => {
            let working_dir = resolve_working_dir(directory);
            workspace::touch_project_dir(working_dir.clone());
            let config = load_runtime_config(LoadOpts {
                config_file: config.clone(),
                ignore_approval,
                workspace_overlay_dir: Some(working_dir.clone()),
                ..Default::default()
            })
            .await?;
            scheduler::run_builtin_scheduler(
                config,
                working_dir,
                std::time::Duration::from_secs(reload_secs),
                None,
                scheduler::CronDelivery::None,
            )
            .await?;
        }
        Commands::Run {
            agent,
            mode,
            workflow,
            goal,
            done_when,
            max_goal_attempts,
            token_budget,
            cost_budget_usd,
            max_duration_secs,
            prompt,
            directory,
        } => {
            let working_dir = resolve_working_dir(directory);
            workspace::touch_project_dir(working_dir.clone());
            let config = load_runtime_config(LoadOpts {
                config_file: config.clone(),
                ignore_approval,
                workspace_overlay_dir: Some(working_dir.clone()),
                ..Default::default()
            })
            .await?;
            tasks::run_task(
                config,
                agent,
                mode,
                workflow,
                goal,
                done_when,
                max_goal_attempts,
                token_budget,
                cost_budget_usd,
                max_duration_secs,
                prompt,
                working_dir,
            )
            .await?;
        }
        Commands::Skills { sub } => {
            let config = load_config_for_session(config.clone(), ignore_approval).await?;
            tasks::run_skills_command(&config, sub)?;
        }
        Commands::TestSecurity { tool, input } => {
            tasks::test_security_system(tool, input).await?;
        }
        _ => unreachable!(),
    }
    Ok(())
}

pub(super) async fn dispatch_default(
    args: &cli_args::Args,
    ignore_approval: bool,
) -> anyhow::Result<()> {
    let working_dir = args
        .directory
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    let working_dir = std::fs::canonicalize(&working_dir).unwrap_or(working_dir);
    workspace::touch_project_dir(working_dir.clone());
    let cfg = load_runtime_config(LoadOpts {
        config_file: args.config.clone(),
        ignore_approval,
        workspace_overlay_dir: Some(working_dir),
        ..Default::default()
    })
    .await?;
    let default_agent = cfg
        .runtime
        .default_mode
        .default_agent()
        .as_str()
        .to_string();
    let agent = args.agent.clone().unwrap_or(default_agent);
    let directory = args.directory.clone();

    if std::io::stdin().is_terminal() {
        tasks::run_interactive(
            cfg,
            agent,
            directory,
            args.model.clone(),
            ignore_approval,
            args.debug,
            args.repl_debug_events,
            args.resume,
            false,
        )
        .await?;
    } else {
        tasks::run_interactive(
            cfg,
            agent,
            directory,
            args.model.clone(),
            ignore_approval,
            args.debug,
            args.repl_debug_events,
            args.resume,
            true,
        )
        .await?;
    }
    Ok(())
}
