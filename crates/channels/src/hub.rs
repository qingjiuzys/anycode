//! 通道统一心智模型（v0.2）
//!
//! 所有入站协议（WeChat / Telegram / Discord）在 CLI 层映射为同一套 [`anycode_core::ChannelMessage`]，
//! 并经 [`anycode`] 的 `channel_task::build_channel_task` 进入单一 `AgentRuntime::execute_task` 路径。
//! 请勿在通道适配器内实现第二套「编码器」编排循环。
