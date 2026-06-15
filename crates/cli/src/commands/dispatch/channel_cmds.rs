//! Channel subcommand dispatch.

use crate::app_config::load_config_for_session;
use crate::channels;
use crate::cli_args::ChannelCommands;
use crate::commands;

pub(super) async fn handle(
    sub: ChannelCommands,
    config_path: Option<std::path::PathBuf>,
    debug: bool,
    ignore_approval: bool,
) -> anyhow::Result<()> {
    match sub {
        ChannelCommands::Status { channel, json } => {
            commands::doctor::print_channel(&channel, json)?;
        }
        ChannelCommands::Wechat {
            data_dir,
            run_as_bridge,
            agent,
        } => {
            if run_as_bridge {
                channels::wechat::run_bridged_start(
                    config_path.clone(),
                    agent,
                    data_dir,
                    ignore_approval,
                )
                .await?;
            } else {
                channels::wechat::run_onboard(data_dir, config_path.clone(), debug).await?;
            }
        }
        ChannelCommands::WechatSendTest {
            message,
            data_dir,
            json,
        } => {
            channels::wechat::send_test_message(data_dir, message, json).await?;
        }
        ChannelCommands::WechatSendMediaTest {
            path,
            caption,
            data_dir,
            json,
        } => {
            channels::wechat::send_test_media(data_dir, path, caption, json).await?;
        }
        ChannelCommands::Telegram {
            bot_token,
            chat_id,
            agent,
            directory,
        } => {
            let config = load_config_for_session(config_path.clone(), ignore_approval).await?;
            channels::tg::run_telegram_polling(
                config,
                channels::tg::TelegramRunArgs {
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
            let config = load_config_for_session(config_path.clone(), ignore_approval).await?;
            channels::discord_channel::run_discord_polling(
                config,
                channels::discord_channel::DiscordRunArgs {
                    bot_token,
                    channel_id,
                    agent,
                    directory,
                },
            )
            .await?;
        }
        ChannelCommands::TelegramSetToken { token, chat_id } => {
            channels::tg::persist_credentials(token, chat_id)?;
            println!("Telegram credentials saved (~/.anycode/channels/telegram.json).");
        }
        ChannelCommands::DiscordSetToken { token, channel_id } => {
            channels::discord_channel::persist_credentials(token, channel_id)?;
            println!("Discord credentials saved (~/.anycode/channels/discord.json).");
        }
    }
    Ok(())
}
