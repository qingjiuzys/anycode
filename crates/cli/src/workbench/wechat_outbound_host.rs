//! `SendWeChatMessage` tool host — real iLink outbound.

use crate::channels::wx::outbound::{send_wechat_media, send_wechat_text};
use anycode_tools::{
    WeChatMediaSendResult, WeChatOutboundHost, WeChatOutboundHostError, WeChatSendResult,
};

pub struct CliWeChatOutboundHost;

fn detail_for_refreshed(target_refreshed: bool) -> String {
    if target_refreshed {
        "sent (context_token refreshed from getupdates)".into()
    } else {
        "sent".into()
    }
}

#[async_trait::async_trait]
impl WeChatOutboundHost for CliWeChatOutboundHost {
    async fn send_text(
        &self,
        message: String,
    ) -> Result<WeChatSendResult, WeChatOutboundHostError> {
        let out = send_wechat_text(None, message)
            .await
            .map_err(|e| WeChatOutboundHostError(format!("{e:#}")))?;
        Ok(WeChatSendResult {
            ok: out.ok,
            message_chars: out.message_chars,
            channel: "wechat",
            detail: detail_for_refreshed(out.target_refreshed),
        })
    }

    async fn send_media(
        &self,
        path: String,
        caption: Option<String>,
    ) -> Result<WeChatMediaSendResult, WeChatOutboundHostError> {
        let cap = caption.as_deref().map(str::trim).filter(|s| !s.is_empty());
        let out = send_wechat_media(None, &path, cap)
            .await
            .map_err(|e| WeChatOutboundHostError(format!("{e:#}")))?;
        Ok(WeChatMediaSendResult {
            ok: out.ok,
            path: out.path,
            file_name: out.file_name,
            bytes: out.bytes,
            delivery: out.delivery,
            channel: "wechat",
            detail: detail_for_refreshed(out.target_refreshed),
        })
    }
}
