use super::anthropic_stream::AnthropicSseStreamState;
use crate::http_retry::{
    evaluate_http_retry, evaluate_network_retry, retry_after_header_ms, retry_exhausted_error,
    sleep_retry_delay,
};
use crate::retry_strategy::ProviderRetryConfig;
use crate::sse_data_lines::{SseDataLine, SseLineBuffer};
use anycode_core::prelude::*;
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::error;
use uuid::Uuid;

pub struct AnthropicClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl AnthropicClient {
    pub fn new(api_key: String) -> Result<Self, super::super::LLMError> {
        if api_key.is_empty() {
            return Err(super::super::LLMError::MissingApiKey);
        }

        Ok(Self {
            client: Client::new(),
            api_key,
            base_url: "https://api.anthropic.com/v1/messages".to_string(),
        })
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }
}

#[async_trait]
impl LLMClient for AnthropicClient {
    async fn chat(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSchema>,
        config: &ModelConfig,
    ) -> Result<LLMResponse, CoreError> {
        let cache = anthropic_cache_enabled(config);
        let (messages, system) = convert_messages(messages, cache);
        let request = AnthropicRequest {
            model: config.model.clone(),
            system,
            messages,
            tools: if tools.is_empty() {
                None
            } else {
                Some(convert_tools(tools, cache))
            },
            max_tokens: config.max_tokens.unwrap_or(4096),
            temperature: config.temperature,
            stream: false,
        };

        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| self.base_url.clone());
        let auth_key = config
            .api_key
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(self.api_key.as_str());
        const MAX_RETRIES: u32 = 8;
        let provider_cfg = ProviderRetryConfig::anthropic();
        let source = config.query_source;
        let model = config.model.clone();
        let observer = config.retry_observer.as_deref();
        let mut attempt: u32 = 0;
        let mut consecutive_overload = 0u32;
        let mut last_err: String;
        loop {
            attempt += 1;
            let response = match self
                .client
                .post(&base_url)
                .header("x-api-key", auth_key)
                .header("anthropic-version", "2023-06-01")
                .json(&request)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    last_err = e.to_string();
                    let out = evaluate_network_retry(&provider_cfg, source, attempt);
                    if out.should_retry && attempt <= MAX_RETRIES {
                        sleep_retry_delay(out.delay, attempt, &model, source, observer).await;
                        continue;
                    }
                    return Err(retry_exhausted_error("Anthropic", &last_err));
                }
            };

            let status = response.status();
            if status.is_success() {
                let anthropic_response: AnthropicResponse = response
                    .json()
                    .await
                    .map_err(|e| CoreError::LLMError(e.to_string()))?;
                return Ok(convert_response(anthropic_response));
            }

            let retry_after_ms = retry_after_header_ms(response.headers());
            let error_text = response.text().await.unwrap_or_default();
            last_err = format!("API error: {status} - {error_text}");
            let out = evaluate_http_retry(
                &provider_cfg,
                source,
                status,
                &error_text,
                attempt,
                consecutive_overload,
                retry_after_ms,
            );
            consecutive_overload = out.consecutive_overload;
            if out.should_retry && attempt <= MAX_RETRIES {
                sleep_retry_delay(out.delay, attempt, &model, source, observer).await;
                continue;
            }
            return Err(CoreError::LLMError(last_err));
        }
    }

    async fn chat_stream(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSchema>,
        config: &ModelConfig,
    ) -> Result<mpsc::Receiver<StreamEvent>, CoreError> {
        let cache = anthropic_cache_enabled(config);
        let (messages, system) = convert_messages(messages, cache);
        let request = AnthropicRequest {
            model: config.model.clone(),
            system,
            messages,
            tools: if tools.is_empty() {
                None
            } else {
                Some(convert_tools(tools, cache))
            },
            max_tokens: config.max_tokens.unwrap_or(4096),
            temperature: config.temperature,
            stream: true,
        };

        let (tx, rx) = mpsc::channel(100);

        let client = self.client.clone();
        let api_key = config
            .api_key
            .as_ref()
            .filter(|s| !s.is_empty())
            .cloned()
            .unwrap_or_else(|| self.api_key.clone());
        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| self.base_url.clone());

        tokio::spawn(async move {
            let response = match client
                .post(&base_url)
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2023-06-01")
                .json(&request)
                .send()
                .await
            {
                Ok(resp) => resp,
                Err(e) => {
                    error!("Stream request failed: {}", e);
                    let _ = tx.send(StreamEvent::Done).await;
                    return;
                }
            };

            if !response.status().is_success() {
                let status = response.status();
                let body = response
                    .text()
                    .await
                    .unwrap_or_default()
                    .chars()
                    .take(600)
                    .collect::<String>();
                error!("Stream API error: {status} body={body}");
                let _ = tx.send(StreamEvent::Done).await;
                return;
            }

            let mut stream = response.bytes_stream();
            let mut line_buf = SseLineBuffer::new();
            let mut anth = AnthropicSseStreamState::new();

            'read: while let Some(chunk_res) = stream.next().await {
                let chunk = match chunk_res {
                    Ok(c) => c,
                    Err(e) => {
                        error!("Stream error: {}", e);
                        break;
                    }
                };
                let Ok(text) = std::str::from_utf8(&chunk) else {
                    continue;
                };
                for ev in line_buf.push_str(text) {
                    match ev {
                        SseDataLine::Done => break 'read,
                        SseDataLine::Payload(data) => match anth.push_json_str(&data) {
                            Ok(events) => {
                                for e in events {
                                    if tx.send(e).await.is_err() {
                                        return;
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Anthropic stream JSON: {}", e);
                            }
                        },
                    }
                }
            }
            for ev in line_buf.finish() {
                match ev {
                    SseDataLine::Done => break,
                    SseDataLine::Payload(data) => {
                        if let Ok(events) = anth.push_json_str(&data) {
                            for e in events {
                                if tx.send(e).await.is_err() {
                                    return;
                                }
                            }
                        }
                    }
                }
            }
            let _ = tx.send(StreamEvent::Done).await;
        });

        Ok(rx)
    }
}

// ============================================================================
// Anthropic Types
// ============================================================================

#[derive(Debug, Serialize)]
pub(crate) struct AnthropicRequest {
    pub(crate) model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) system: Option<Vec<AnthropicTextBlock>>,
    pub(crate) messages: Vec<AnthropicMessage>,
    pub(crate) tools: Option<Vec<AnthropicTool>>,
    pub(crate) max_tokens: u32,
    pub(crate) temperature: Option<f32>,
    pub(crate) stream: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct AnthropicMessage {
    role: String,
    content: Vec<AnthropicContent>,
}

#[derive(Debug, Serialize)]
pub(crate) struct AnthropicImageSource {
    #[serde(rename = "type")]
    source_type: &'static str,
    media_type: String,
    data: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct AnthropicImageBlock {
    #[serde(rename = "type")]
    block_type: &'static str,
    source: AnthropicImageSource,
}

/// The Anthropic messages API requires a `type` tag on every content block;
/// strict gateways (e.g. Kimi `/coding`) 400 on `{"text": ...}` without it.
#[derive(Debug, Serialize)]
pub(crate) struct AnthropicTextBlock {
    #[serde(rename = "type")]
    block_type: &'static str,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_control: Option<AnthropicCacheControl>,
}

#[derive(Debug, Serialize)]
pub(crate) struct AnthropicToolUseBlock {
    #[serde(rename = "type")]
    block_type: &'static str,
    id: String,
    name: String,
    input: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub(crate) struct AnthropicToolResultBlock {
    #[serde(rename = "type")]
    block_type: &'static str,
    tool_use_id: String,
    content: String,
    is_error: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub(crate) enum AnthropicContent {
    Text(AnthropicTextBlock),
    Image(AnthropicImageBlock),
    ToolUse(AnthropicToolUseBlock),
    ToolResult(AnthropicToolResultBlock),
}

impl AnthropicContent {
    fn text(text: String) -> Self {
        Self::Text(AnthropicTextBlock {
            block_type: "text",
            text,
            cache_control: None,
        })
    }
    fn tool_use(id: String, name: String, input: serde_json::Value) -> Self {
        Self::ToolUse(AnthropicToolUseBlock {
            block_type: "tool_use",
            id,
            name,
            input,
        })
    }
    fn tool_result(tool_use_id: String, content: String, is_error: Option<bool>) -> Self {
        Self::ToolResult(AnthropicToolResultBlock {
            block_type: "tool_result",
            tool_use_id,
            content,
            is_error,
        })
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct AnthropicCacheControl {
    #[serde(rename = "type")]
    control_type: &'static str,
}

impl AnthropicCacheControl {
    fn ephemeral() -> Self {
        Self {
            control_type: "ephemeral",
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct AnthropicTool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_control: Option<AnthropicCacheControl>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AnthropicResponse {
    #[allow(dead_code)] // Responses API 字段，反序列化保留；未映射到 LLMResponse
    id: String,
    #[allow(dead_code)]
    role: String,
    content: Vec<AnthropicResponseContent>,
    #[allow(dead_code)]
    stop_reason: Option<String>,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum AnthropicResponseContent {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        #[serde(default)]
        input: serde_json::Value,
    },
    /// Extended thinking / Token Plan reasoning blocks — ignored for text output
    /// (fields retained for forward-compatible parsing).
    Thinking {
        #[serde(default)]
        #[allow(dead_code)]
        thinking: String,
        #[serde(default)]
        #[allow(dead_code)]
        signature: String,
    },
    /// Forward-compatible catch-all for unknown content block types.
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
    #[serde(default)]
    cache_creation_input_tokens: u32,
    #[serde(default)]
    cache_read_input_tokens: u32,
}

// ============================================================================
// Conversion Functions
// ============================================================================

fn user_message_content(msg: &Message) -> Vec<AnthropicContent> {
    let text = match &msg.content {
        MessageContent::Text(t) => t.as_str(),
        MessageContent::ToolUse { .. } | MessageContent::ToolResult { .. } => "",
    };
    let mut parts = Vec::new();
    if !text.is_empty() {
        parts.push(AnthropicContent::text(text.to_string()));
    }
    for img in anycode_core::vision_images_from_metadata(&msg.metadata) {
        parts.push(AnthropicContent::Image(AnthropicImageBlock {
            block_type: "image",
            source: AnthropicImageSource {
                source_type: "base64",
                media_type: img.mime_type,
                data: img.data_base64,
            },
        }));
    }
    if parts.is_empty() {
        parts.push(AnthropicContent::text(text.to_string()));
    }
    parts
}

pub(crate) fn convert_messages(
    messages: Vec<Message>,
    cache: bool,
) -> (Vec<AnthropicMessage>, Option<Vec<AnthropicTextBlock>>) {
    // 启用缓存时：把 system 消息提取到顶层 `system` 字段（Anthropic 合法缓存点之一）。
    let mut system_blocks: Vec<AnthropicTextBlock> = Vec::new();
    let messages: Vec<Message> = if cache {
        messages
            .into_iter()
            .filter(|msg| {
                if msg.role == MessageRole::System {
                    if let MessageContent::Text(t) = &msg.content {
                        if !t.is_empty() {
                            system_blocks.push(AnthropicTextBlock {
                                block_type: "text",
                                text: t.clone(),
                                cache_control: None,
                            });
                        }
                    }
                    false
                } else {
                    true
                }
            })
            .collect()
    } else {
        messages
    };

    let mut converted: Vec<AnthropicMessage> = messages
        .into_iter()
        .map(|msg| {
            if msg.role == MessageRole::Assistant {
                let mut parts: Vec<AnthropicContent> = vec![];
                if let MessageContent::Text(t) = &msg.content {
                    if !t.is_empty() {
                        parts.push(AnthropicContent::text(t.clone()));
                    }
                }
                if let Some(v) = msg.metadata.get(ANYCODE_TOOL_CALLS_METADATA_KEY) {
                    if let Ok(calls) = serde_json::from_value::<Vec<ToolCall>>(v.clone()) {
                        for c in calls {
                            parts.push(AnthropicContent::tool_use(c.id, c.name, c.input));
                        }
                    }
                }
                if parts.is_empty() {
                    parts.push(AnthropicContent::text(String::new()));
                }
                return AnthropicMessage {
                    role: "assistant".to_string(),
                    content: parts,
                };
            }

            AnthropicMessage {
                role: match msg.role {
                    MessageRole::User => "user".to_string(),
                    MessageRole::System => "system".to_string(),
                    MessageRole::Tool => "user".to_string(),
                    MessageRole::Assistant => {
                        unreachable!("assistant messages are handled above")
                    }
                },
                content: match msg.content {
                    MessageContent::Text(_) if msg.role == MessageRole::User => {
                        user_message_content(&msg)
                    }
                    MessageContent::Text(text) => vec![AnthropicContent::text(text)],
                    MessageContent::ToolUse { name, input } => {
                        vec![AnthropicContent::tool_use(
                            Uuid::new_v4().to_string(),
                            name,
                            input,
                        )]
                    }
                    MessageContent::ToolResult {
                        tool_use_id,
                        content,
                        is_error,
                    } => vec![AnthropicContent::tool_result(
                        tool_use_id,
                        content,
                        Some(is_error),
                    )],
                },
            }
        })
        .collect();

    if cache {
        // 最后一条 user/assistant 消息的最后一个文本块打缓存点（Anthropic 合法位置）。
        if let Some(last) = converted
            .iter_mut()
            .rev()
            .find(|m| m.role == "user" || m.role == "assistant")
        {
            if let Some(AnthropicContent::Text(t)) = last
                .content
                .iter_mut()
                .rev()
                .find(|c| matches!(c, AnthropicContent::Text(_)))
            {
                t.cache_control = Some(AnthropicCacheControl::ephemeral());
            }
        }
        if !system_blocks.is_empty() {
            system_blocks.last_mut().unwrap().cache_control =
                Some(AnthropicCacheControl::ephemeral());
            return (converted, Some(system_blocks));
        }
    }
    (converted, None)
}

/// 是否启用 Anthropic prompt caching：`ModelConfig.prompt_cache` > env 覆盖 > 官方端点自动开启。
fn anthropic_cache_enabled(config: &ModelConfig) -> bool {
    if let Some(v) = config.prompt_cache {
        return v;
    }
    match std::env::var("ANYCODE_ANTHROPIC_PROMPT_CACHE")
        .ok()
        .as_deref()
    {
        Some("0") | Some("false") | Some("no") | Some("off") | Some("disabled") => return false,
        Some("1") | Some("true") | Some("yes") | Some("on") | Some("enabled") => return true,
        _ => {}
    }
    let base = config
        .base_url
        .as_deref()
        .unwrap_or("https://api.anthropic.com");
    base.contains("api.anthropic.com")
}

pub(crate) fn convert_tools(tools: Vec<ToolSchema>, cache: bool) -> Vec<AnthropicTool> {
    tools
        .into_iter()
        .map(|tool| AnthropicTool {
            name: tool.name,
            description: tool.description,
            input_schema: tool.input_schema,
            cache_control: cache.then(AnthropicCacheControl::ephemeral),
        })
        .collect()
}

pub(crate) fn convert_response(response: AnthropicResponse) -> LLMResponse {
    let (text, tool_calls) = response.content.into_iter().fold(
        (String::new(), Vec::new()),
        |(mut text, mut tool_calls), content| match content {
            AnthropicResponseContent::Text { text: t } => {
                text.push_str(&t);
                (text, tool_calls)
            }
            AnthropicResponseContent::ToolUse { id, name, input } => {
                tool_calls.push(ToolCall { id, name, input });
                (text, tool_calls)
            }
            AnthropicResponseContent::Thinking { .. } | AnthropicResponseContent::Other => {
                (text, tool_calls)
            }
        },
    );

    LLMResponse {
        message: Message {
            id: Uuid::new_v4(),
            role: MessageRole::Assistant,
            content: MessageContent::Text(text),
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        },
        tool_calls,
        usage: Usage {
            input_tokens: response.usage.input_tokens,
            output_tokens: response.usage.output_tokens,
            cache_creation_tokens: (response.usage.cache_creation_input_tokens > 0)
                .then_some(response.usage.cache_creation_input_tokens),
            cache_read_tokens: (response.usage.cache_read_input_tokens > 0)
                .then_some(response.usage.cache_read_input_tokens),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_thinking_and_text_blocks_from_token_plan() {
        let json = r#"{
            "id": "msg_1",
            "role": "assistant",
            "content": [
                {"type": "thinking", "thinking": "plan…", "signature": ""},
                {"type": "text", "text": "你好"}
            ],
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 3, "output_tokens": 5}
        }"#;
        let parsed: AnthropicResponse = serde_json::from_str(json).expect("deserialize");
        let out = convert_response(parsed);
        match out.message.content {
            MessageContent::Text(t) => assert_eq!(t, "你好"),
            other => panic!("unexpected {other:?}"),
        }
        assert!(out.tool_calls.is_empty());
    }

    #[test]
    fn parses_flat_tool_use_block() {
        let json = r#"{
            "id": "msg_2",
            "role": "assistant",
            "content": [
                {"type": "tool_use", "id": "tu1", "name": "Bash", "input": {"command": "ls"}}
            ],
            "stop_reason": "tool_use",
            "usage": {"input_tokens": 1, "output_tokens": 2}
        }"#;
        let parsed: AnthropicResponse = serde_json::from_str(json).expect("deserialize");
        let out = convert_response(parsed);
        assert_eq!(out.tool_calls.len(), 1);
        assert_eq!(out.tool_calls[0].name, "Bash");
    }

    #[test]
    fn cache_mode_extracts_system_and_stamps_breakpoints() {
        let msgs = vec![
            Message {
                id: Uuid::new_v4(),
                role: MessageRole::System,
                content: MessageContent::Text("sys".into()),
                timestamp: chrono::Utc::now(),
                metadata: Default::default(),
            },
            Message {
                id: Uuid::new_v4(),
                role: MessageRole::User,
                content: MessageContent::Text("hi".into()),
                timestamp: chrono::Utc::now(),
                metadata: Default::default(),
            },
        ];
        let (msgs, system) = convert_messages(msgs, true);
        let sys = system.expect("system extracted");
        assert_eq!(sys.len(), 1);
        assert_eq!(
            serde_json::to_value(&sys[0]).unwrap()["cache_control"]["type"],
            "ephemeral"
        );
        // system 不再留在 messages
        assert!(msgs.iter().all(|m| m.role != "system"));
        // 最后一条 user 文本块打点
        let last_text = serde_json::to_value(&msgs[0]).unwrap();
        assert_eq!(
            last_text["content"][0]["cache_control"]["type"],
            "ephemeral"
        );
    }

    #[test]
    fn cache_off_keeps_system_in_messages_without_breakpoints() {
        let msgs = vec![
            Message {
                id: Uuid::new_v4(),
                role: MessageRole::System,
                content: MessageContent::Text("sys".into()),
                timestamp: chrono::Utc::now(),
                metadata: Default::default(),
            },
            Message {
                id: Uuid::new_v4(),
                role: MessageRole::User,
                content: MessageContent::Text("hi".into()),
                timestamp: chrono::Utc::now(),
                metadata: Default::default(),
            },
        ];
        let (msgs, system) = convert_messages(msgs, false);
        assert!(system.is_none());
        let v = serde_json::to_value(msgs).unwrap();
        assert_eq!(v[0]["role"], "system");
        assert!(v[1]["content"][0].get("cache_control").is_none());
    }

    #[test]
    fn request_content_blocks_carry_type_tags() {
        // Strict Anthropic-compatible gateways (Kimi /coding) 400 without `type`.
        let msgs = vec![
            Message {
                id: Uuid::new_v4(),
                role: MessageRole::User,
                content: MessageContent::Text("hi".into()),
                timestamp: chrono::Utc::now(),
                metadata: Default::default(),
            },
            Message {
                id: Uuid::new_v4(),
                role: MessageRole::Tool,
                content: MessageContent::ToolResult {
                    tool_use_id: "tu1".into(),
                    content: "done".into(),
                    is_error: false,
                },
                timestamp: chrono::Utc::now(),
                metadata: Default::default(),
            },
        ];
        let (msgs, system) = convert_messages(msgs, false);
        assert!(system.is_none());
        let v = serde_json::to_value(msgs).unwrap();
        assert_eq!(v[0]["content"][0]["type"], "text");
        assert_eq!(v[1]["content"][0]["type"], "tool_result");
        assert_eq!(v[1]["content"][0]["tool_use_id"], "tu1");
    }
}
