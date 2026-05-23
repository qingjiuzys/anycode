//! Task execution and interactive REPL dispatch.

use super::{load_config_for_session, resolve_working_dir};
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
            let config = load_config_for_session(config.clone(), ignore_approval).await?;
            let working_dir = resolve_working_dir(directory);
            workspace::touch_project_dir(working_dir.clone());
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
            let config = load_config_for_session(config.clone(), ignore_approval).await?;
            let working_dir = resolve_working_dir(directory);
            workspace::touch_project_dir(working_dir.clone());
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
    let cfg = load_config_for_session(args.config.clone(), ignore_approval).await?;
    if let Ok(cwd) = std::env::current_dir() {
        workspace::touch_project_dir(cwd);
    }
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
