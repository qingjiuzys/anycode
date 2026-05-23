//! Channel bridge entry points (WeChat, Telegram, Discord).
//!
//! Call sites should use `crate::channels::` (no crate-root re-exports).

pub mod discord;
pub mod discord_ask;
pub mod telegram;
pub mod tg_ask;
pub mod wechat;
pub mod wechat_ilink;
pub mod wechat_service;
pub mod wx;
pub mod wx_ask;

pub(crate) use discord as discord_channel;
pub(crate) use telegram as tg;
