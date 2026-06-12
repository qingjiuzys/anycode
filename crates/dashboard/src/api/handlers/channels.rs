//! Channel setup probes and settings status (Telegram / Discord).

use super::*;
use anycode_setup::{
    build_channels_settings_view, discord_invite_url, list_telegram_chats,
    save_discord_credentials, save_telegram_credentials, test_discord_channel, verify_discord_bot,
    verify_telegram_bot, DiscordCredentials, TelegramCredentials,
};
use serde::Deserialize;

pub async fn get_settings_channels() -> impl IntoResponse {
    let view = build_channels_settings_view();
    Json(json!({ "channels": view })).into_response()
}

#[derive(Deserialize)]
pub struct ChannelTokenBody {
    pub bot_token: String,
}

#[derive(Deserialize)]
pub struct SetupTelegramBody {
    pub bot_token: String,
    #[serde(default)]
    pub chat_id: Option<String>,
}

#[derive(Deserialize)]
pub struct SetupDiscordBody {
    pub bot_token: String,
    pub channel_id: String,
}

#[derive(Deserialize)]
pub struct DiscordTestBody {
    pub bot_token: String,
    pub channel_id: String,
}

pub async fn post_setup_channels_telegram_verify(
    Json(body): Json<ChannelTokenBody>,
) -> impl IntoResponse {
    match verify_telegram_bot(body.bot_token.trim()).await {
        Ok(bot) => Json(json!({ "ok": true, "bot": bot })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn post_setup_channels_telegram_chats(
    Json(body): Json<ChannelTokenBody>,
) -> impl IntoResponse {
    match list_telegram_chats(body.bot_token.trim()).await {
        Ok(chats) => Json(json!({ "ok": true, "chats": chats })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn post_setup_channels_discord_verify(
    Json(body): Json<ChannelTokenBody>,
) -> impl IntoResponse {
    match verify_discord_bot(body.bot_token.trim()).await {
        Ok(bot) => {
            let invite_url = discord_invite_url(&bot.id);
            Json(json!({ "ok": true, "bot": bot, "invite_url": invite_url })).into_response()
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn post_setup_channels_discord_test(
    Json(body): Json<DiscordTestBody>,
) -> impl IntoResponse {
    match test_discord_channel(body.bot_token.trim(), body.channel_id.trim()).await {
        Ok(result) => Json(json!({ "ok": true, "result": result })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn post_setup_channels_telegram(
    Json(body): Json<SetupTelegramBody>,
) -> impl IntoResponse {
    match save_telegram_credentials(&TelegramCredentials {
        bot_token: body.bot_token,
        chat_id: body.chat_id,
    }) {
        Ok(path) => Json(json!({ "ok": true, "path": path.display().to_string() })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn post_setup_channels_discord(Json(body): Json<SetupDiscordBody>) -> impl IntoResponse {
    match save_discord_credentials(&DiscordCredentials {
        bot_token: body.bot_token,
        channel_id: body.channel_id,
    }) {
        Ok(path) => Json(json!({ "ok": true, "path": path.display().to_string() })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": e.to_string() })),
        )
            .into_response(),
    }
}
