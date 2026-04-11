//! OpenAI Chat Completions API（官方 `api.openai.com` 或兼容网关）。
//!
//! 请求/响应 JSON 与 [`super::zai::ZaiClient`] 所用 OpenAI 兼容形态一致；非流式解析复用 `ZaiResponse` + [`super::zai::convert_response`]。

use super::zai::{
    is_retryable_status, llm_response_from_openai_compatible_str, messages_to_openai_json,
    openai_tools_from_schemas, retry_delay_ms,
};
use crate::sse_data_lines::{SseDataLine, SseLineBuffer};
use crate::LLMError;
use anycode_core::prelude::*;
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, warn};

const DEFAULT_OPENAI_CHAT_URL: &str = "https://api.openai.com/v1/chat/completions";
const DEFAULT_API_TIMEOUT_MS: u64 = 600_000;
const DEFAULT_MAX_RETRIES: u32 = 10;

#[derive(Debug, Serialize)]
struct OpenAiChatRequestBody {
    model: String,
    messages: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream_options: Option<OpenAiStreamOptions>,
}

#[derive(Debug, Serialize)]
struct OpenAiStreamOptions {
    include_usage: bool,
}

fn openai_tool_choice(tools_empty: bool) -> Option<String> {
    if tools_empty {
        return None;
    }
    if let Ok(v) = std::env::var("ANYCODE_OPENAI_TOOL_CHOICE") {
        let v = v.trim().to_lowercase();
        if matches!(v.as_str(), "required" | "auto" | "none") {
            return Some(v);
        }
    }
    Some("auto".to_string())
}

fn configured_api_timeout_ms() -> u64 {
    std::env::var("API_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|v| *v >= 1_000)
        .unwrap_or(DEFAULT_API_TIMEOUT_MS)
}

fn build_http_client() -> Client {
    Client::builder()
        .timeout(Duration::from_millis(configured_api_timeout_ms()))
        .build()
        .unwrap_or_else(|_| Client::new())
}

/// `true` 表示应停止（`tx` 已关闭）。
async fn emit_openai_sse_json_chunk(
    val: &Value,
    tx: &mpsc::Sender<StreamEvent>,
    tool_builders: &mut HashMap<u64, (Option<String>, Option<String>, String)>,
) -> bool {
    if let Some(usage) = val.get("usage").and_then(openai_stream_usage_from_value) {
        if tx.send(StreamEvent::Usage(usage)).await.is_err() {
            return true;
        }
    }
    let choice = val
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first());

    if let Some(delta) = choice.and_then(|c| c.get("delta")) {
        if let Some(c) = delta.get("content").and_then(|x| x.as_str()) {
            if !c.is_empty() && tx.send(StreamEvent::Delta(c.to_string())).await.is_err() {
                return true;
            }
        }
        if let Some(arr) = delta.get("tool_calls").and_then(|x| x.as_array()) {
            for part in arr {
                let index = part.get("index").and_then(|i| i.as_u64()).unwrap_or(0);
                let entry = tool_builders
                    .entry(index)
                    .or_insert((None, None, String::new()));
                if let Some(id) = part.get("id").and_then(|i| i.as_str()) {
                    entry.0 = Some(id.to_string());
                }
                if let Some(f) = part.get("function") {
                    if let Some(n) = f.get("name").and_then(|x| x.as_str()) {
                        entry.1.get_or_insert_with(|| n.to_string());
                    }
                    if let Some(a) = f.get("arguments").and_then(|x| x.as_str()) {
                        entry.2.push_str(a);
                    }
                }
            }
        }
    }

    if let Some(reason) = choice
        .and_then(|c| c.get("finish_reason"))
        .and_then(|r| r.as_str())
    {
        if reason == "tool_calls" {
            let mut indices: Vec<u64> = tool_builders.keys().copied().collect();
            indices.sort_unstable();
            for i in indices {
                if let Some((id_o, name_o, args)) = tool_builders.remove(&i) {
                    let id = id_o.unwrap_or_else(|| format!("call_{i}"));
                    let name = name_o.unwrap_or_default();
                    let input: Value = if args.trim().is_empty() {
                        json!({})
                    } else {
                        serde_json::from_str(&args).unwrap_or_else(|_| json!({ "raw": args }))
                    };
                    let tc = ToolCall { id, name, input };
                    if tx.send(StreamEvent::ToolCall(tc)).await.is_err() {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn openai_stream_usage_from_value(v: &Value) -> Option<Usage> {
    Some(Usage {
        input_tokens: v.get("prompt_tokens")?.as_u64()? as u32,
        output_tokens: v.get("completion_tokens")?.as_u64()? as u32,
        cache_creation_tokens: None,
        cache_read_tokens: None,
    })
}

async fn send_chat_with_retries(
    client: &Client,
    url: &str,
    auth_key: &str,
    body: &OpenAiChatRequestBody,
) -> Result<reqwest::Response, CoreError> {
    let max_retries: u32 = DEFAULT_MAX_RETRIES;
    let mut last_err: Option<String> = None;
    let mut response: Option<reqwest::Response> = None;
    for attempt in 1..=max_retries + 1 {
        let send_res = client
            .post(url)
            .header("Authorization", format!("Bearer {}", auth_key))
            .json(body)
            .send()
            .await;

        match send_res {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    response = Some(resp);
                    break;
                }

                let retry_after_ms = resp
                    .headers()
                    .get("retry-after")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .map(|secs| secs.saturating_mul(1000));

                let error_text = resp.text().await.unwrap_or_default();
                let mut snippet = error_text.clone();
                const MAX_ERR: usize = 2000;
                if snippet.len() > MAX_ERR {
                    snippet.truncate(MAX_ERR);
                    snippet.push_str("...<truncated>");
                }
                last_err = Some(format!(
                    "OpenAI API error: status={} url={} body={}",
                    status.as_u16(),
                    url,
                    if snippet.is_empty() {
                        "<empty>"
                    } else {
                        &snippet
                    }
                ));

                if attempt <= max_retries && is_retryable_status(status) {
                    let delay = retry_after_ms.unwrap_or_else(|| retry_delay_ms(attempt));
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                    continue;
                }
                break;
            }
            Err(e) => {
                let mut msg = e.to_string();
                if e.is_timeout() {
                    msg = format!(
                        "{msg} · API_TIMEOUT_MS={}ms, try increasing it",
                        configured_api_timeout_ms()
                    );
                }
                last_err = Some(msg);
                if attempt <= max_retries {
                    let delay = retry_delay_ms(attempt);
                    warn!(
                        "Retrying in {} seconds… (attempt {}/{}){}",
                        delay / 1000,
                        attempt,
                        max_retries,
                        if std::env::var("API_TIMEOUT_MS").is_ok() {
                            format!(
                                " · API_TIMEOUT_MS={}ms, try increasing it",
                                configured_api_timeout_ms()
                            )
                        } else {
                            String::new()
                        }
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                    continue;
                }
                break;
            }
        }
    }

    response.ok_or_else(|| {
        CoreError::LLMError(format!(
            "OpenAI request failed after retries: {}",
            last_err.unwrap_or_else(|| "unknown error".to_string())
        ))
    })
}

/// OpenAI 官方 Chat Completions 客户端（`feature = "openai"`）。
pub struct OpenAIClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl OpenAIClient {
    pub fn new(api_key: String) -> Result<Self, LLMError> {
        if api_key.is_empty() {
            return Err(LLMError::MissingApiKey);
        }

        Ok(Self {
            client: build_http_client(),
            api_key,
            base_url: DEFAULT_OPENAI_CHAT_URL.to_string(),
        })
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }
}

#[async_trait]
impl LLMClient for OpenAIClient {
    async fn chat(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSchema>,
        config: &ModelConfig,
    ) -> Result<LLMResponse, CoreError> {
        let tool_choice = openai_tool_choice(tools.is_empty());
        let openai_messages = messages_to_openai_json(messages)?;

        let model = config
            .model
            .trim()
            .is_empty()
            .then(|| "gpt-4o-mini".to_string())
            .unwrap_or_else(|| config.model.clone());

        let tools_json = if tools.is_empty() {
            None
        } else {
            Some(openai_tools_from_schemas(&tools))
        };

        if tools_json.is_some() {
            debug!(
                "OpenAI request includes {} tools, tool_choice={:?}",
                tools.len(),
                tool_choice
            );
        }

        let body = OpenAiChatRequestBody {
            model,
            messages: openai_messages,
            temperature: config.temperature,
            max_tokens: config.max_tokens,
            stream: Some(false),
            tools: tools_json,
            tool_choice,
            stream_options: None,
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

        let response = send_chat_with_retries(&self.client, &base_url, auth_key, &body).await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(CoreError::LLMError(format!(
                "OpenAI API error (no retry): status={} url={} body={}",
                status.as_u16(),
                base_url,
                if error_text.is_empty() {
                    "<empty>"
                } else {
                    &error_text
                }
            )));
        }

        let text = response
            .text()
            .await
            .map_err(|e| CoreError::LLMError(e.to_string()))?;
        llm_response_from_openai_compatible_str(&text)
    }

    async fn chat_stream(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSchema>,
        config: &ModelConfig,
    ) -> Result<mpsc::Receiver<StreamEvent>, CoreError> {
        let tool_choice = openai_tool_choice(tools.is_empty());
        let openai_messages = messages_to_openai_json(messages)?;

        let model = config
            .model
            .trim()
            .is_empty()
            .then(|| "gpt-4o-mini".to_string())
            .unwrap_or_else(|| config.model.clone());

        let tools_json = if tools.is_empty() {
            None
        } else {
            Some(openai_tools_from_schemas(&tools))
        };

        let body = OpenAiChatRequestBody {
            model,
            messages: openai_messages,
            temperature: config.temperature,
            max_tokens: config.max_tokens,
            stream: Some(true),
            tools: tools_json,
            tool_choice,
            stream_options: Some(OpenAiStreamOptions {
                include_usage: true,
            }),
        };

        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| self.base_url.clone());

        let auth_key: String = config
            .api_key
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.api_key.clone());

        let client = self.client.clone();
        let (tx, rx) = mpsc::channel(128);

        tokio::spawn(async move {
            let response = match client
                .post(&base_url)
                .header("Authorization", format!("Bearer {}", auth_key))
                .json(&body)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    error!("OpenAI stream request failed: {}", e);
                    let _ = tx.send(StreamEvent::Done).await;
                    return;
                }
            };

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                error!(
                    "OpenAI stream HTTP error: {} {}",
                    status,
                    &body[..body.len().min(500)]
                );
                let _ = tx.send(StreamEvent::Done).await;
                return;
            }

            let mut stream = response.bytes_stream();
            let mut sse_buf = SseLineBuffer::new();
            let mut tool_builders: HashMap<u64, (Option<String>, Option<String>, String)> =
                HashMap::new();

            'read: while let Some(chunk_res) = stream.next().await {
                let chunk = match chunk_res {
                    Ok(c) => c,
                    Err(e) => {
                        error!("OpenAI stream read: {}", e);
                        break;
                    }
                };
                let Ok(text) = std::str::from_utf8(&chunk) else {
                    continue;
                };
                for line_ev in sse_buf.push_str(text) {
                    let data = match line_ev {
                        SseDataLine::Done => break 'read,
                        SseDataLine::Payload(s) => s,
                    };
                    let Ok(val) = serde_json::from_str::<Value>(&data) else {
                        continue;
                    };
                    if emit_openai_sse_json_chunk(&val, &tx, &mut tool_builders).await {
                        return;
                    }
                }
            }

            for line_ev in sse_buf.finish() {
                let SseDataLine::Payload(data) = line_ev else {
                    break;
                };
                let Ok(val) = serde_json::from_str::<Value>(&data) else {
                    continue;
                };
                if emit_openai_sse_json_chunk(&val, &tx, &mut tool_builders).await {
                    return;
                }
            }

            let _ = tx.send(StreamEvent::Done).await;
        });

        Ok(rx)
    }
}
