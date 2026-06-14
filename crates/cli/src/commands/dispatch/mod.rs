//! CLI command dispatch.

use crate::app_config::{load_runtime_config, LoadOpts};
use crate::cli_args;
use crate::cli_args::{ChannelCommands, Commands};
use tracing::info;
use tracing_subscriber::fmt;
use tracing_subscriber::EnvFilter;

mod channel_cmds;
mod ops;
mod task_cmds;

pub(crate) use crate::app_config::load_config_for_session;

pub(crate) async fn load_config_with_cwd_overlays(
    config_path: Option<std::path::PathBuf>,
    ignore_approval: bool,
) -> anyhow::Result<crate::app_config::Config> {
    load_runtime_config(LoadOpts {
        config_file: config_path,
        ignore_approval,
        workspace_overlay: true,
        workspace_overlay_dir: None,
        wechat_bridge: false,
    })
    .await
}

pub(crate) fn resolve_working_dir(directory: Option<std::path::PathBuf>) -> std::path::PathBuf {
    directory.unwrap_or_else(|| std::env::current_dir().unwrap())
}

pub(crate) fn tracing_env_filter(repl_quiet: bool, debug: bool) -> EnvFilter {
    if std::env::var_os("RUST_LOG").is_some() {
        return EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            if debug {
                EnvFilter::new("debug")
            } else if repl_quiet {
                EnvFilter::new("error")
            } else {
                EnvFilter::new("info")
            }
        });
    }
    if debug {
        EnvFilter::new("debug")
    } else if repl_quiet {
        // Interactive quiet: only errors should reach terminal to avoid polluting prompt area.
        EnvFilter::new("error")
    } else {
        EnvFilter::new("info")
    }
}

pub(crate) async fn run_cli() -> anyhow::Result<()> {
    let args = cli_args::parse_args();

    // Default interactive terminal / channel bots: keep stdout clean; INFO logs pollute the UI.
    let interactive_quiet = matches!(
        args.command,
        None | Some(Commands::Channel {
            sub: ChannelCommands::Telegram { .. },
        }) | Some(Commands::Channel {
            sub: ChannelCommands::Discord { .. },
        })
    );

    // Logs on stderr; stdout for ratatui / REPL banner and task output.
    let subscriber = fmt::Subscriber::builder()
        .with_env_filter(tracing_env_filter(interactive_quiet, args.debug))
        .with_writer(std::io::stderr)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    if !interactive_quiet || args.debug {
        info!("anyCode v{} starting...", anycode_core::VERSION);
        info!("anyCode CLI ready");
    }

    let ignore_approval = cli_args::session_ignore_approval(args.ignore_approval);

    match args.command {
        Some(Commands::Channel { sub }) => {
            channel_cmds::handle(sub, args.config.clone(), args.debug, ignore_approval).await?;
        }
        Some(
            cmd @ (Commands::Config
            | Commands::Enable { .. }
            | Commands::Disable { .. }
            | Commands::Mode { .. }
            | Commands::Status { .. }
            | Commands::Statusline { .. }
            | Commands::Setup { .. }
            | Commands::Memory { .. }
            | Commands::Eval { .. }
            | Commands::Doctor { .. }
            | Commands::Cron { .. }
            | Commands::Audit { .. }
            | Commands::Workspace { .. }
            | Commands::Project { .. }
            | Commands::Workflow { .. }
            | Commands::Model { .. }
            | Commands::Mcp { .. }
            | Commands::Wechat { .. }
            | Commands::Dashboard { .. }),
        ) => {
            ops::dispatch(cmd, args.config.clone(), args.debug, ignore_approval).await?;
        }
        Some(
            cmd @ (Commands::Scheduler { .. }
            | Commands::Run { .. }
            | Commands::Skills { .. }
            | Commands::TestSecurity { .. }),
        ) => {
            task_cmds::dispatch(cmd, args.config.clone(), ignore_approval).await?;
        }
        None => task_cmds::dispatch_default(&args, ignore_approval).await?,
    }

    Ok(())
}
