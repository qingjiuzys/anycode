//! anyCode CLI entry (see `cli_args`, `app_config`, `bootstrap`, `tasks`).

mod app_config;
mod artifact_summary;
mod bootstrap;
mod builtin_agents;
mod channel_task;
mod cli_args;
mod commands;
mod copilot_auth;
mod discord_channel;
mod i18n;
mod md_tui;
mod repl_banner;
mod repl_inline;
mod scheduler;
mod slash_commands;
mod tasks;
mod tg;
mod tui;
mod wechat;
mod wechat_ilink;
mod wechat_service;
mod workspace;
mod wx;

use app_config::{load_config_for_session, run_onboard_flow};
#[cfg(feature = "mcp-oauth")]
use cli_args::McpCommands;
use cli_args::{ChannelCommands, Commands};
#[cfg(feature = "mcp-oauth")]
use fluent_bundle::FluentArgs;
#[cfg(feature = "mcp-oauth")]
use i18n::{tr, tr_args};
use tracing::info;
use tracing_subscriber::fmt;
use tracing_subscriber::EnvFilter;

fn tracing_env_filter(repl_quiet: bool, debug: bool) -> EnvFilter {
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = cli_args::parse_args();

    // Interactive surfaces (fullscreen TUI / repl) should keep terminal clean by
    // default; INFO logs easily pollute the prompt/input area.
    let interactive_quiet = matches!(
        args.command,
        None | Some(Commands::Repl { .. })
            | Some(Commands::Channel {
                sub: ChannelCommands::Telegram { .. },
            })
            | Some(Commands::Channel {
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
        info!("anyCode v0.2.0 starting...");
        info!("anyCode CLI ready");
    }

    let ignore_approval = cli_args::session_ignore_approval(args.ignore_approval);

    match args.command {
        Some(Commands::Config) => {
            app_config::run_config_wizard().await?;
        }
        Some(Commands::Enable { feature }) => {
            commands::feature::handle_enable(args.config.clone(), feature).await?;
        }
        Some(Commands::Disable { feature }) => {
            commands::feature::handle_disable(args.config.clone(), feature).await?;
        }
        Some(Commands::Mode { mode }) => {
            commands::feature::handle_mode(args.config.clone(), ignore_approval, mode).await?;
        }
        Some(Commands::Status { json }) => {
            let mut config = load_config_for_session(args.config.clone(), ignore_approval).await?;
            if let Ok(cwd) = std::env::current_dir() {
                let wd = std::fs::canonicalize(&cwd).unwrap_or(cwd);
                workspace::apply_project_overlays(&mut config, &wd);
            }
            commands::status::print_status(&config, json)?;
        }
        Some(Commands::Statusline { sub }) => {
            let mut config = load_config_for_session(args.config.clone(), ignore_approval).await?;
            if let Ok(cwd) = std::env::current_dir() {
                let wd = std::fs::canonicalize(&cwd).unwrap_or(cwd);
                workspace::apply_project_overlays(&mut config, &wd);
            }
            match sub {
                cli_args::StatuslineCommands::PrintSchema => {
                    commands::statusline::print_schema(&config)?;
                }
            }
        }
        Some(Commands::Setup { channel, data_dir }) => {
            run_onboard_flow(args.config.clone(), data_dir, channel, args.debug).await?;
        }
        Some(Commands::Channel { sub }) => match sub {
            ChannelCommands::Wechat {
                data_dir,
                run_as_bridge,
                agent,
            } => {
                if run_as_bridge {
                    wechat::run_bridged_start(
                        args.config.clone(),
                        agent,
                        data_dir,
                        ignore_approval,
                    )
                    .await?;
                } else {
                    wechat::run_onboard(data_dir, args.config.clone(), args.debug).await?;
                }
            }
            ChannelCommands::Telegram {
                bot_token,
                chat_id,
                agent,
                directory,
            } => {
                let config = load_config_for_session(args.config.clone(), ignore_approval).await?;
                tg::run_telegram_polling(
                    config,
                    tg::TelegramRunArgs {
                        bot_token,
                        chat_id,
                        agent,
                        directory,
                    },
                )
                .await?;
            }
            ChannelCommands::Discord {
                bot_token,
                channel_id,
                agent,
                directory,
            } => {
                let config = load_config_for_session(args.config.clone(), ignore_approval).await?;
                discord_channel::run_discord_polling(
                    config,
                    discord_channel::DiscordRunArgs {
                        bot_token,
                        channel_id,
                        agent,
                        directory,
                    },
                )
                .await?;
            }
            ChannelCommands::TelegramSetToken { token, chat_id } => {
                tg::persist_credentials(token, chat_id)?;
                println!("Telegram credentials saved (~/.anycode/channels/telegram.json).");
            }
            ChannelCommands::DiscordSetToken { token, channel_id } => {
                discord_channel::persist_credentials(token, channel_id)?;
                println!("Discord credentials saved (~/.anycode/channels/discord.json).");
            }
        },
        Some(Commands::Workspace { sub }) => match sub {
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
        Some(Commands::Model { command }) => match command {
            None => {
                app_config::run_model_interactive(args.config.clone()).await?;
            }
            Some(cmd) => {
                app_config::run_model_command(cmd, args.config.clone()).await?;
            }
        },
        Some(Commands::Scheduler {
            directory,
            reload_secs,
        }) => {
            let config = load_config_for_session(args.config.clone(), ignore_approval).await?;
            let working_dir = directory.unwrap_or_else(|| std::env::current_dir().unwrap());
            workspace::touch_project_dir(working_dir.clone());
            scheduler::run_builtin_scheduler(
                config,
                working_dir,
                std::time::Duration::from_secs(reload_secs),
            )
            .await?;
        }
        Some(Commands::Run {
            agent,
            mode,
            workflow,
            goal,
            prompt,
            directory,
        }) => {
            let config = load_config_for_session(args.config.clone(), ignore_approval).await?;
            let working_dir = directory.unwrap_or_else(|| std::env::current_dir().unwrap());
            workspace::touch_project_dir(working_dir.clone());
            tasks::run_task(config, agent, mode, workflow, goal, prompt, working_dir).await?;
        }
        Some(Commands::Repl {
            agent,
            directory,
            model,
        }) => {
            let config = load_config_for_session(args.config.clone(), ignore_approval).await?;
            let touch_dir = directory
                .clone()
                .unwrap_or_else(|| std::env::current_dir().unwrap());
            workspace::touch_project_dir(touch_dir);
            tasks::run_interactive(config, agent, directory, model, ignore_approval).await?;
        }
        Some(Commands::Skills { sub }) => {
            let config = load_config_for_session(args.config.clone(), ignore_approval).await?;
            tasks::run_skills_command(&config, sub)?;
        }
        Some(Commands::TestSecurity { tool, input }) => {
            tasks::test_security_system(tool, input).await?;
        }
        #[cfg(feature = "mcp-oauth")]
        Some(Commands::Mcp { sub }) => match sub {
            McpCommands::OauthLogin {
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
        None => {
            let config = load_config_for_session(args.config.clone(), ignore_approval).await?;
            if let Ok(cwd) = std::env::current_dir() {
                workspace::touch_project_dir(cwd);
            }
            let default_agent = config
                .runtime
                .default_mode
                .default_agent()
                .as_str()
                .to_string();
            tui::run_tui(
                config,
                default_agent,
                None,
                args.model.clone(),
                args.debug,
                args.resume,
            )
            .await?;
        }
    }

    Ok(())
}
