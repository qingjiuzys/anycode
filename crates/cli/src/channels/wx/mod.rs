//! 微信 iLink 消息桥（纯 Rust，替代 Node wechat-claude-code 常驻进程）。

mod approval;
mod bridge;
mod cdn_media;
mod commands;
mod config_watch;
pub(crate) mod cron_notify;
mod fields;
mod ilink;
pub(crate) mod outbound_queue;
mod permission;
mod store;

pub use bridge::run_wechat_daemon;
pub use ilink::WxSender;
pub use store::wcc_data_dir;
