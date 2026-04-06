//! 多通道消息（与 `anycode-channels` 对齐的领域类型）。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 通道类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ChannelType {
    CLI,
    IDE,
    WhatsApp,
    Telegram,
    Discord,
    Slack,
    Teams,
    Web,
    WeChat,
}

/// 通道消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessage {
    pub channel_type: ChannelType,
    pub channel_id: String,
    pub user_id: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub reply_to: Option<String>,
}
