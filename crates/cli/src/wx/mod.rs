//! 微信 iLink 消息桥（纯 Rust，替代 Node wechat-claude-code 常驻进程）。

mod approval;
mod bridge;
mod cdn_media;
mod commands;
mod fields;
mod ilink;
mod permission;
mod store;

pub use bridge::run_wechat_daemon;
pub use store::wcc_data_dir;
