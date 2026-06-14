//! Maintenance / observability command dispatch.

use super::load_config_with_cwd_overlays;
use crate::app_config::run_onboard_flow;
#[cfg(feature = "mcp-oauth")]
use crate::cli_args::McpCommands;
use crate::cli_args::{Commands, MemoryCommands};
#[cfg(feature = "mcp-oauth")]
use crate::i18n::{tr, tr_args};
use crate::{app_config, cli_args, commands};
#[cfg(feature = "mcp-oauth")]
use fluent_bundle::FluentArgs;

pub(super) async fn dispatch(
    command: Commands,
    config: Option<std::path::PathBuf>,
    debug: bool,
    ignore_approval: bool,
) -> anyhow::Result<()> {
    match command {
        Commands::Config => {
            app_config::run_config_wizard().await?;
        }
        Commands::Enable { feature } => {
            commands::feature::handle_enable(config.clone(), feature).await?;
        }
        Commands::Disable { feature } => {
            commands::feature::handle_disable(config.clone(), feature).await?;
        }
        Commands::Mode { mode } => {
            commands::feature::handle_mode(config.clone(), ignore_approval, mode).await?;
        }
        Commands::Status { json } => {
            let config = load_config_with_cwd_overlays(config.clone(), ignore_approval).await?;
            commands::status::print_status(&config, json)?;
        }
        Commands::Statusline { sub } => {
            let config = load_config_with_cwd_overlays(config.clone(), ignore_approval).await?;
            match sub {
                cli_args::StatuslineCommands::PrintSchema => {
                    commands::statusline::print_schema(&config)?;
                }
            }
        }
        Commands::Setup { channel, data_dir } => {
            run_onboard_flow(config.clone(), data_dir, channel, debug).await?;
        }
        Commands::Memory { sub } => match sub {
            MemoryCommands::Import { dry_run, limit } => {
                let config = load_config_with_cwd_overlays(config.clone(), ignore_approval).await?;
                commands::memory_import::run_import(&config, dry_run, limit).await?;
            }
            MemoryCommands::Prune {
                dry_run,
                apply,
                older_than_days,
                json,
            } => {
                let config = load_config_with_cwd_overlays(config.clone(), ignore_approval).await?;
                commands::memory_retention::run_prune(
                    &config,
                    dry_run,
                    apply,
                    older_than_days,
                    json,
                )
                .await?;
            }
            MemoryCommands::Doctor { json } => {
                let config = load_config_with_cwd_overlays(config.clone(), ignore_approval).await?;
                commands::doctor::print_memory(&config, json)?;
            }
        },
        Commands::Eval { sub } => match sub {
            cli_args::EvalCommands::List { json } => commands::eval::list(json)?,
            cli_args::EvalCommands::Run { json, mock } => commands::eval::run(json, mock)?,
        },
        Commands::Doctor { sub } => {
            let config = load_config_with_cwd_overlays(config.clone(), ignore_approval).await?;
            match sub {
                cli_args::DoctorCommands::All { json } => {
                    commands::doctor::print_all(&config, json)?;
                }
                cli_args::DoctorCommands::Memory { json } => {
                    commands::doctor::print_memory(&config, json)?;
                }
                cli_args::DoctorCommands::Channel { channel, json } => {
                    commands::doctor::print_channel(&channel, json)?;
                }
                cli_args::DoctorCommands::Mcp { json } => {
                    commands::doctor::print_mcp(json)?;
                }
                cli_args::DoctorCommands::Tools { json } => {
                    commands::doctor::print_tools(json)?;
                }
                cli_args::DoctorCommands::WechatHistory { json } => {
                    commands::doctor::print_wechat_history(json)?;
                }
                cli_args::DoctorCommands::Errors { json } => {
                    commands::cli_error::print_taxonomy(json)?;
                }
            }
        }
        Commands::Cron { sub } => match sub {
            cli_args::CronCommands::Runs {
                job,
                session,
                limit,
                json,
            } => {
                commands::cron::print_runs(job, session, limit, json)?;
            }
        },
        Commands::Audit { sub } => match sub {
            cli_args::AuditCommands::Tail {
                task,
                tool,
                limit,
                json,
            } => {
                commands::audit::print_tail(task, tool, limit, json)?;
            }
        },
        Commands::Project { sub } => match sub {
            cli_args::ProjectCommands::Templates { json } => {
                commands::project::list_templates(json)?;
            }
            cli_args::ProjectCommands::Init {
                template,
                path,
                name,
                title,
                org,
                force,
                flutter_create,
            } => {
                commands::project::init(template, path, name, title, org, force, flutter_create)
                    .await?;
            }
        },
        Commands::Workspace { sub } => match sub {
            cli_args::WorkspaceCommands::List { json } => {
                commands::workspace::handle_list(json).await?;
            }
            cli_args::WorkspaceCommands::Status => {
                commands::workspace::handle_status().await?;
            }
            cli_args::WorkspaceCommands::Touch { path } => {
                commands::workspace::handle_touch(path).await?;
            }
            cli_args::WorkspaceCommands::SetMode { mode, path } => {
                commands::workspace::handle_set_mode(mode, path).await?;
            }
            cli_args::WorkspaceCommands::SetChannel { channel, path } => {
                commands::workspace::handle_set_channel(channel, path).await?;
            }
            cli_args::WorkspaceCommands::Label { label, path } => {
                commands::workspace::handle_label(label, path).await?;
            }
        },
        Commands::Workflow { sub } => match sub {
            cli_args::WorkflowCommands::Validate { file, json } => {
                commands::workflow::validate(&file, json)?;
            }
        },
        Commands::Model { command } => match command {
            None => {
                app_config::run_model_interactive(config.clone()).await?;
            }
            Some(cmd) => {
                app_config::run_model_command(cmd, config.clone()).await?;
            }
        },
        Commands::Mcp { sub } => match sub {
            cli_args::McpCommands::Status { json } => commands::mcp::print_status(json)?,
            #[cfg(feature = "mcp-oauth")]
            cli_args::McpCommands::OauthLogin {
                url,
                host,
                port,
                callback_path,
                client_metadata_url,
                scopes,
                no_browser,
                write_token,
                credentials_store,
            } => {
                use anycode_tools::{mcp_oauth_login, McpOAuthLoginOptions};
                use std::time::Duration;
                let cred_path = credentials_store.clone();
                let token = mcp_oauth_login(McpOAuthLoginOptions {
                    mcp_url: url,
                    redirect_host: host,
                    redirect_port: port,
                    callback_path,
                    client_metadata_url,
                    scopes,
                    client_name: Some("anycode".to_string()),
                    open_browser: !no_browser,
                    callback_timeout: Duration::from_secs(15 * 60),
                    credentials_store,
                })
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
                println!("{}\n{token}", tr("oauth-access-token-label"));
                if let Some(path) = cred_path {
                    let mut a = FluentArgs::new();
                    a.set("path", path.display().to_string());
                    eprintln!("{}", tr_args("oauth-creds-saved", &a));
                }
                if let Some(path) = write_token {
                    std::fs::write(&path, format!("{token}\n")).map_err(|e| {
                        let mut a = FluentArgs::new();
                        a.set("path", path.display().to_string());
                        a.set("err", e.to_string());
                        anyhow::anyhow!("{}", tr_args("oauth-write-token-err", &a))
                    })?;
                    let mut a = FluentArgs::new();
                    a.set("path", path.display().to_string());
                    eprintln!("{}", tr_args("oauth-wrote-token", &a));
                }
            }
        },
        Commands::Dashboard { sub, run } => {
            commands::dashboard::run_dashboard_command(sub, run).await?;
        }
        Commands::Wechat { sub } => match sub {
            cli_args::WechatCommands::History { sub } => match sub {
                cli_args::WechatHistoryCommands::Setup { json } => {
                    commands::wechat_history::run_setup_flow(json)?;
                }
                cli_args::WechatHistoryCommands::Status { json } => {
                    commands::wechat_history::run_status(json)?;
                }
            },
        },
        _ => unreachable!(),
    }
    Ok(())
}
