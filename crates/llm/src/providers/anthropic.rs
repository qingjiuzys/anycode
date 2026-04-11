use super::anthropic_stream::AnthropicSseStreamState;
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
        let request = AnthropicRequest {
            model: config.model.clone(),
            messages: convert_messages(messages),
            tools: if tools.is_empty() {
                None
            } else {
                Some(convert_tools(tools))
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
        let mut attempt: u32 = 0;
        loop {
            attempt += 1;
            let response = self
                .client
                .post(&base_url)
                .header("x-api-key", auth_key)
                .header("anthropic-version", "2023-06-01")
                .json(&request)
                .send()
                .await
                .map_err(|e| CoreError::LLMError(e.to_string()))?;

            let status = response.status();
            if status.is_success() {
                let anthropic_response: AnthropicResponse = response
                    .json()
                    .await
                    .map_err(|e| CoreError::LLMError(e.to_string()))?;
                return Ok(convert_response(anthropic_response));
            }

            let retry_after_ms = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .map(|s| s.saturating_mul(1000));

            let error_text = response.text().await.unwrap_or_default();
            if attempt <= MAX_RETRIES && super::zai::is_retryable_status(status) {
                let delay = retry_after_ms.unwrap_or_else(|| super::zai::retry_delay_ms(attempt));
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                continue;
            }
            return Err(CoreError::LLMError(format!(
                "API error: {} - {}",
                status, error_text
            )));
        }
    }

    async fn chat_stream(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSchema>,
        config: &ModelConfig,
    ) -> Result<mpsc::Receiver<StreamEvent>, CoreError> {
        let request = AnthropicRequest {
            model: config.model.clone(),
            messages: convert_messages(messages),
            tools: if tools.is_empty() {
                None
            } else {
                Some(convert_tools(tools))
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
                error!("Stream API error: {}", response.status());
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
#[serde(untagged)]
pub(crate) enum AnthropicContent {
    Text { text: String },
    ToolUse { tool_use: AnthropicToolUse },
    ToolResult { tool_result: AnthropicToolResult },
}

#[derive(Debug, Serialize)]
pub(crate) struct AnthropicToolUse {
    id: String,
    name: String,
    input: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub(crate) struct AnthropicToolResult {
    tool_use_id: String,
    content: String,
    is_error: Option<bool>,
}

#[derive(Debug, Serialize)]
pub(crate) struct AnthropicTool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
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
#[serde(untagged)]
pub(crate) enum AnthropicResponseContent {
    Text { text: String },
    ToolUse { tool_use: AnthropicToolUseResponse },
}

#[derive(Debug, Deserialize)]
pub(crate) struct AnthropicToolUseResponse {
    id: String,
    name: String,
    input: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

// ============================================================================
// Conversion Functions
// ============================================================================

pub(crate) fn convert_messages(messages: Vec<Message>) -> Vec<AnthropicMessage> {
    messages
        .into_iter()
        .map(|msg| {
            if msg.role == MessageRole::Assistant {
                let mut parts: Vec<AnthropicContent> = vec![];
                if let MessageContent::Text(t) = &msg.content {
                    if !t.is_empty() {
                        parts.push(AnthropicContent::Text { text: t.clone() });
                    }
                }
                if let Some(v) = msg.metadata.get(ANYCODE_TOOL_CALLS_METADATA_KEY) {
                    if let Ok(calls) = serde_json::from_value::<Vec<ToolCall>>(v.clone()) {
                        for c in calls {
                            parts.push(AnthropicContent::ToolUse {
                                tool_use: AnthropicToolUse {
                                    id: c.id,
                                    name: c.name,
                                    input: c.input,
                                },
                            });
                        }
                    }
                }
                if parts.is_empty() {
                    parts.push(AnthropicContent::Text {
                        text: String::new(),
                    });
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
                    MessageContent::Text(text) => vec![AnthropicContent::Text { text }],
                    MessageContent::ToolUse { name, input } => vec![AnthropicContent::ToolUse {
                        tool_use: AnthropicToolUse {
                            id: Uuid::new_v4().to_string(),
                            name,
                            input,
                        },
                    }],
                    MessageContent::ToolResult {
                        tool_use_id,
                        content,
                        is_error,
                    } => vec![AnthropicContent::ToolResult {
                        tool_result: AnthropicToolResult {
                            tool_use_id,
                            content,
                            is_error: Some(is_error),
                        },
                    }],
                },
            }
        })
        .collect()
}

pub(crate) fn convert_tools(tools: Vec<ToolSchema>) -> Vec<AnthropicTool> {
    tools
        .into_iter()
        .map(|tool| AnthropicTool {
            name: tool.name,
            description: tool.description,
            input_schema: tool.input_schema,
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
            AnthropicResponseContent::ToolUse { tool_use } => {
                tool_calls.push(ToolCall {
                    id: tool_use.id,
                    name: tool_use.name,
                    input: tool_use.input,
                });
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
            cache_creation_tokens: None,
            cache_read_tokens: None,
        },
    }
}
