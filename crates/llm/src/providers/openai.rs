//! OpenAI Chat Completions API（官方 `api.openai.com` 或兼容网关）。
//!
//! 请求/响应 JSON 与 [`super::zai::ZaiClient`] 所用 OpenAI 兼容形态一致；非流式解析复用 `ZaiResponse` + [`super::zai::convert_response`]。

use super::zai::{
    llm_response_from_openai_compatible_str, messages_to_openai_json_for_config,
    openai_tools_from_schemas,
};
use crate::http_retry::{
    evaluate_http_retry, evaluate_network_retry, retry_after_header_ms, retry_exhausted_error,
    sleep_retry_delay,
};
use crate::retry_strategy::ProviderRetryConfig;
use crate::sse_data_lines::{SseDataLine, SseLineBuffer};
use crate::LLMError;
use anycode_core::prelude::*;
use anycode_core::{LlmRetryObserver, QuerySource};
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error};

const DEFAULT_OPENAI_CHAT_URL: &str = "https://api.openai.com/v1/chat/completions";
/// 单次 HTTP 请求总超时（含流式读 body）；更长对话可设环境变量 `API_TIMEOUT_MS`。
const DEFAULT_API_TIMEOUT_MS: u64 = 180_000;
/// 建连阶段（TCP/TLS）超时，与总超时独立。
const DEFAULT_API_CONNECT_TIMEOUT_MS: u64 = 30_000;
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

fn openai_tool_choice(
    messages: &[Message],
    tools_empty: bool,
    config: &ModelConfig,
) -> Option<String> {
    if tools_empty {
        return None;
    }
    if let Ok(v) = std::env::var("ANYCODE_OPENAI_TOOL_CHOICE") {
        let v = v.trim().to_lowercase();
        if matches!(v.as_str(), "required" | "auto" | "none") {
            return Some(v);
        }
    }
    let capabilities = crate::capabilities_for_model_config(config);
    if capabilities.weak_local_model
        && (crate::has_tool_recovery_nudge(messages)
            || (crate::is_first_agent_turn(messages)
                && crate::explicitly_requests_tool_execution(messages)))
    {
        return Some("required".to_string());
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
        .connect_timeout(Duration::from_millis(DEFAULT_API_CONNECT_TIMEOUT_MS))
        .timeout(Duration::from_millis(configured_api_timeout_ms()))
        .build()
        .unwrap_or_else(|_| Client::new())
}

/// `true` 表示应停止（`tx` 已关闭）。
async fn emit_openai_sse_with_state(
    val: &Value,
    tx: &mpsc::Sender<StreamEvent>,
    state: &mut crate::tool_call_normalizer::OpenAiCompatStreamState,
) -> bool {
    crate::openai_compat_stream::emit_openai_sse_with_state(val, tx, state).await
}

fn openai_stream_usage_from_value(v: &Value) -> Option<Usage> {
    crate::openai_compat_stream::openai_stream_usage_from_value(v)
}

async fn send_chat_with_retries(
    client: &Client,
    url: &str,
    auth_key: &str,
    body: &OpenAiChatRequestBody,
    source: QuerySource,
    model: &str,
    observer: Option<&dyn LlmRetryObserver>,
) -> Result<reqwest::Response, CoreError> {
    let provider_cfg = ProviderRetryConfig::openai();
    let max_retries = provider_cfg.base_config.max_retries;
    let mut last_err: Option<String> = None;
    let mut response: Option<reqwest::Response> = None;
    let mut consecutive_overload = 0u32;
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

                let retry_after_ms = retry_after_header_ms(resp.headers());
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

                if crate::providers::zai::is_quota_exhausted(&error_text) {
                    error!("OpenAI-compatible quota exhausted — failing fast without retries");
                    break;
                }

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
                if out.should_retry && attempt <= max_retries {
                    sleep_retry_delay(out.delay, attempt, model, source, observer).await;
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
                let out = evaluate_network_retry(&provider_cfg, source, attempt);
                if out.should_retry && attempt <= max_retries {
                    sleep_retry_delay(out.delay, attempt, model, source, observer).await;
                    continue;
                }
                break;
            }
        }
    }

    response.ok_or_else(|| {
        retry_exhausted_error(
            "OpenAI",
            &last_err.unwrap_or_else(|| "unknown error".to_string()),
        )
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
        let tool_choice = openai_tool_choice(&messages, tools.is_empty(), config);
        let openai_messages = messages_to_openai_json_for_config(messages, config)?;

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

        let model_for_retry = model.clone();
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

        let response = send_chat_with_retries(
            &self.client,
            &base_url,
            auth_key,
            &body,
            config.query_source,
            &model_for_retry,
            config.retry_observer.as_deref(),
        )
        .await?;

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
        llm_response_from_openai_compatible_str(&text, &tools)
    }

    async fn chat_stream(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSchema>,
        config: &ModelConfig,
    ) -> Result<mpsc::Receiver<StreamEvent>, CoreError> {
        let tool_choice = openai_tool_choice(&messages, tools.is_empty(), config);
        let openai_messages = messages_to_openai_json_for_config(messages, config)?;

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
        let stream_tools = tools.clone();

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
            let mut stream_state =
                crate::tool_call_normalizer::OpenAiCompatStreamState::new(stream_tools);

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
                    if emit_openai_sse_with_state(&val, &tx, &mut stream_state).await {
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
                if emit_openai_sse_with_state(&val, &tx, &mut stream_state).await {
                    return;
                }
            }

            if crate::openai_compat_stream::flush_openai_sse_state(&tx, &mut stream_state).await {
                return;
            }

            let _ = tx.send(StreamEvent::Done).await;
        });

        Ok(rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;
    use uuid::Uuid;

    fn text(role: MessageRole, value: &str) -> Message {
        Message {
            id: Uuid::new_v4(),
            role,
            content: MessageContent::Text(value.into()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    fn local_config() -> ModelConfig {
        ModelConfig {
            provider: LLMProvider::OpenAI,
            model: "minicpm5-1b".into(),
            base_url: Some("http://127.0.0.1:47100/v1/chat/completions".into()),
            ..Default::default()
        }
    }

    #[test]
    fn local_weak_model_requires_tool_only_for_explicit_agent_work() {
        let config = local_config();
        let tool_task = vec![
            text(MessageRole::System, "system"),
            text(MessageRole::User, "请读取 Cargo.toml"),
        ];
        assert_eq!(
            openai_tool_choice(&tool_task, false, &config).as_deref(),
            Some("required")
        );

        let question = vec![
            text(MessageRole::System, "system"),
            text(MessageRole::User, "解释 Rust 所有权"),
        ];
        assert_eq!(
            openai_tool_choice(&question, false, &config).as_deref(),
            Some("auto")
        );
    }

    #[test]
    fn local_weak_model_requires_tool_for_complex_delivery_brief() {
        let config = local_config();
        let brief = vec![
            text(MessageRole::System, "system"),
            text(
                MessageRole::User,
                "六月数据、代码仓库，帮我完整交付一轮。fixtures/e2e-complex-repo/ artifacts/ DELIVERY_MANIFEST.json cargo test",
            ),
        ];
        assert_eq!(
            openai_tool_choice(&brief, false, &config).as_deref(),
            Some("required")
        );
    }

    #[test]
    fn recovery_nudge_requires_tool_once_after_refusal() {
        let config = local_config();
        let messages = vec![
            text(MessageRole::User, "请执行测试"),
            text(MessageRole::Assistant, "I cannot do that"),
            text(MessageRole::User, crate::TOOL_RECOVERY_NUDGE),
        ];
        assert_eq!(
            openai_tool_choice(&messages, false, &config).as_deref(),
            Some("required")
        );
    }
}
