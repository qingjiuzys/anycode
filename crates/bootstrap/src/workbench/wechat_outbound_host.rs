//! `SendWeChatMessage` tool host — delegates to channel bridge when linked.

use anycode_tools::{
    WeChatMediaSendResult, WeChatOutboundHost, WeChatOutboundHostError, WeChatSendResult,
};

pub struct CliWeChatOutboundHost;

#[async_trait::async_trait]
impl WeChatOutboundHost for CliWeChatOutboundHost {
    async fn send_text(
        &self,
        _message: String,
    ) -> Result<WeChatSendResult, WeChatOutboundHostError> {
        Err(WeChatOutboundHostError(
            "WeChat outbound is provided by anycode-channel-bridge when a bridge is running".into(),
        ))
    }

    async fn send_media(
        &self,
        _path: String,
        _caption: Option<String>,
    ) -> Result<WeChatMediaSendResult, WeChatOutboundHostError> {
        Err(WeChatOutboundHostError(
            "WeChat outbound is provided by anycode-channel-bridge when a bridge is running".into(),
        ))
    }
}
