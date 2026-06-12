//! Host hook for outbound WeChat text (injected by CLI bootstrap).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeChatMediaDelivery {
    InlineText,
    CdnMedia,
    PathNote,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeChatSendResult {
    pub ok: bool,
    pub message_chars: usize,
    pub channel: &'static str,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeChatMediaSendResult {
    pub ok: bool,
    pub path: String,
    pub file_name: String,
    pub bytes: u64,
    pub delivery: WeChatMediaDelivery,
    pub channel: &'static str,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct WeChatOutboundHostError(pub String);

impl std::fmt::Display for WeChatOutboundHostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for WeChatOutboundHostError {}

#[async_trait]
pub trait WeChatOutboundHost: Send + Sync {
    async fn send_text(&self, message: String)
        -> Result<WeChatSendResult, WeChatOutboundHostError>;

    async fn send_media(
        &self,
        path: String,
        caption: Option<String>,
    ) -> Result<WeChatMediaSendResult, WeChatOutboundHostError>;
}

pub type WeChatOutboundHostArc = Arc<dyn WeChatOutboundHost>;
