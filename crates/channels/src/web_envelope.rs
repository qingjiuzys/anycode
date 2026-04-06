//! WebSocket 文本帧约定：`v=1` 信封，便于版本演进与 `ping`/`pong`。

use anycode_core::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnycodeWsEnvelopeV1 {
    pub v: u32,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub payload: serde_json::Value,
}

pub fn outbound_channel_message_json(msg: &ChannelMessage) -> Result<String, serde_json::Error> {
    let env = AnycodeWsEnvelopeV1 {
        v: 1,
        kind: "channel_message".to_string(),
        payload: serde_json::to_value(msg)?,
    };
    serde_json::to_string(&env)
}

pub fn is_ping_json(text: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .map(|v| {
            v.get("v").and_then(|x| x.as_u64()) == Some(1)
                && v.get("type").and_then(|x| x.as_str()) == Some("ping")
        })
        .unwrap_or(false)
}

pub fn pong_json() -> String {
    json!({"v":1,"type":"pong","payload":{}}).to_string()
}

/// 解析入站文本：信封 `channel_message`、裸 [`ChannelMessage`]、或任意文本回退。
pub fn parse_inbound_text_to_channel_message(text: &str, channel_id: &str) -> ChannelMessage {
    if let Ok(env) = serde_json::from_str::<AnycodeWsEnvelopeV1>(text) {
        if env.v == 1 && env.kind == "channel_message" {
            if let Ok(msg) = serde_json::from_value(env.payload) {
                return msg;
            }
        }
    }
    if let Ok(msg) = serde_json::from_str::<ChannelMessage>(text) {
        return msg;
    }
    ChannelMessage {
        channel_type: ChannelType::Web,
        channel_id: channel_id.to_string(),
        user_id: channel_id.to_string(),
        content: text.to_string(),
        timestamp: chrono::Utc::now(),
        reply_to: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_detection() {
        assert!(is_ping_json(r#"{"v":1,"type":"ping","payload":{}}"#));
        assert!(!is_ping_json(r#"{"hello":1}"#));
    }

    #[test]
    fn envelope_roundtrip_channel_message() {
        let msg = ChannelMessage {
            channel_type: ChannelType::Web,
            channel_id: "c1".into(),
            user_id: "u1".into(),
            content: "hi".into(),
            timestamp: chrono::Utc::now(),
            reply_to: None,
        };
        let json = outbound_channel_message_json(&msg).unwrap();
        let back = parse_inbound_text_to_channel_message(&json, "c1");
        assert_eq!(back.content, "hi");
    }
}
