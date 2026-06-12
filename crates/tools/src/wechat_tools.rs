//! Outbound WeChat message tool (real iLink send, not in-session `SendUserMessage`).

use crate::services::ToolServices;
use crate::wechat_outbound_host::WeChatOutboundHostError;
use anycode_core::prelude::*;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use std::time::Instant;

pub struct SendWeChatMessageTool {
    services: Arc<ToolServices>,
    policy: SecurityPolicy,
}

impl SendWeChatMessageTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            services,
            policy: SecurityPolicy::sensitive_mutation(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct SendWeChatIn {
    message: Option<String>,
    text: Option<String>,
    path: Option<String>,
    file: Option<String>,
}

#[derive(Debug)]
struct ParsedWeChatIn {
    message: Option<String>,
    path: Option<String>,
}

fn parse_wechat_input(m: &SendWeChatIn) -> Result<ParsedWeChatIn, CoreError> {
    let message = m
        .message
        .as_deref()
        .or(m.text.as_deref())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let path = m
        .path
        .as_deref()
        .or(m.file.as_deref())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    if message.is_none() && path.is_none() {
        return Err(CoreError::ConfigError(
            "at least one of message (or text) or path (or file) is required".into(),
        ));
    }

    Ok(ParsedWeChatIn { message, path })
}

#[async_trait]
impl Tool for SendWeChatMessageTool {
    fn name(&self) -> &str {
        "SendWeChatMessage"
    }

    fn description(&self) -> &str {
        "Send text and/or a local file (image, video, PDF, office doc, etc.) to the user's bound \
         WeChat chat via the iLink bridge. Use path (or file) for attachments; use message (or \
         text) for plain text or as a caption before the file. Small .md/.txt may be inlined as \
         text; CDN upload max is 10MB. Use this (not SendUserMessage) when the user asks to \
         message them on WeChat. Requires a recent inbound WeChat message to refresh context_token."
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "Plain text to deliver on WeChat, or caption when path is set"
                },
                "text": { "type": "string", "description": "Alias of message" },
                "path": {
                    "type": "string",
                    "description": "Local file path to send (image, video, PDF, office doc, etc.)"
                },
                "file": { "type": "string", "description": "Alias of path" }
            }
        })
    }

    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Default
    }

    fn security_policy(&self) -> Option<&SecurityPolicy> {
        Some(&self.policy)
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let start = Instant::now();
        let m: SendWeChatIn = serde_json::from_value(input.input).unwrap_or(SendWeChatIn {
            message: None,
            text: None,
            path: None,
            file: None,
        });
        let parsed = parse_wechat_input(&m)?;
        let Some(host) = self.services.wechat_outbound_host() else {
            return Ok(ToolOutput {
                result: json!({
                    "ok": false,
                    "error": "WeChat outbound is not configured in this runtime (no WeChatOutboundHost)"
                }),
                error: Some("WeChat outbound host missing".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        };

        if let Some(path) = parsed.path.clone() {
            let caption = parsed.message.clone();
            match host.send_media(path.clone(), caption.clone()).await {
                Ok(r) => Ok(ToolOutput {
                    result: json!({
                        "ok": r.ok,
                        "kind": "media",
                        "channel": r.channel,
                        "path": r.path,
                        "file_name": r.file_name,
                        "bytes": r.bytes,
                        "delivery": r.delivery,
                        "caption": caption,
                        "detail": r.detail,
                    }),
                    error: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
                Err(WeChatOutboundHostError(msg)) => Ok(ToolOutput {
                    result: json!({
                        "ok": false,
                        "kind": "media",
                        "channel": "wechat",
                        "path": path,
                        "caption": caption,
                        "error": msg,
                    }),
                    error: Some(msg),
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
            }
        } else {
            let body = parsed
                .message
                .clone()
                .expect("validated message when path absent");
            match host.send_text(body.clone()).await {
                Ok(r) => Ok(ToolOutput {
                    result: json!({
                        "ok": r.ok,
                        "kind": "text",
                        "channel": r.channel,
                        "message_chars": r.message_chars,
                        "body": body,
                        "detail": r.detail,
                    }),
                    error: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
                Err(WeChatOutboundHostError(msg)) => Ok(ToolOutput {
                    result: json!({
                        "ok": false,
                        "kind": "text",
                        "channel": "wechat",
                        "body": body,
                        "error": msg,
                    }),
                    error: Some(msg),
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_message_only() {
        let got = parse_wechat_input(&SendWeChatIn {
            message: Some("hello".into()),
            text: None,
            path: None,
            file: None,
        })
        .unwrap();
        assert_eq!(got.message.as_deref(), Some("hello"));
        assert!(got.path.is_none());
    }

    #[test]
    fn parse_path_only() {
        let got = parse_wechat_input(&SendWeChatIn {
            message: None,
            text: None,
            path: Some("/tmp/report.pdf".into()),
            file: None,
        })
        .unwrap();
        assert!(got.message.is_none());
        assert_eq!(got.path.as_deref(), Some("/tmp/report.pdf"));
    }

    #[test]
    fn parse_path_and_caption() {
        let got = parse_wechat_input(&SendWeChatIn {
            message: Some("here is the report".into()),
            text: None,
            path: None,
            file: Some("/tmp/report.pdf".into()),
        })
        .unwrap();
        assert_eq!(got.message.as_deref(), Some("here is the report"));
        assert_eq!(got.path.as_deref(), Some("/tmp/report.pdf"));
    }

    #[test]
    fn parse_requires_message_or_path() {
        let err = parse_wechat_input(&SendWeChatIn {
            message: None,
            text: None,
            path: None,
            file: None,
        })
        .unwrap_err();
        assert!(err.to_string().contains("at least one of"));
    }

    #[test]
    fn text_alias_works() {
        let got = parse_wechat_input(&SendWeChatIn {
            message: None,
            text: Some("ping".into()),
            path: None,
            file: None,
        })
        .unwrap();
        assert_eq!(got.message.as_deref(), Some("ping"));
    }
}
