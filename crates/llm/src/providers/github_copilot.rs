//! GitHub Copilot：换 token 后以 Anthropic Messages 兼容路径调用（Claude 系模型）。

use super::anthropic::{
    convert_messages, convert_response, convert_tools, AnthropicRequest, AnthropicResponse,
};
use super::anthropic_stream::AnthropicSseStreamState;
use crate::copilot_token::resolve_copilot_api_token;
use crate::sse_data_lines::{SseDataLine, SseLineBuffer};
use anycode_core::prelude::*;
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use tokio::sync::mpsc;
use tracing::error;

const COPILOT_EDITOR_VERSION: &str = "vscode/1.96.2";
const COPILOT_USER_AGENT: &str = "GitHubCopilotChat/0.26.7";

pub struct GithubCopilotClient {
    client: Client,
    github_token: String,
}

impl GithubCopilotClient {
    pub fn new(github_token: String) -> Result<Self, super::super::LLMError> {
        if github_token.trim().is_empty() {
            return Err(super::super::LLMError::MissingApiKey);
        }
        Ok(Self {
            client: Client::new(),
            github_token,
        })
    }

    fn anthropic_url(api_base: &str) -> String {
        format!("{}/anthropic/v1/messages", api_base.trim_end_matches('/'))
    }

    fn is_claude_model(model: &str) -> bool {
        model.to_lowercase().contains("claude")
    }
}

#[async_trait]
impl LLMClient for GithubCopilotClient {
    async fn chat(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSchema>,
        config: &ModelConfig,
    ) -> Result<LLMResponse, CoreError> {
        if !Self::is_claude_model(&config.model) {
            return Err(CoreError::LLMError(
                "GitHub Copilot：当前仅支持模型 id 含 `claude` 的 Anthropic Messages 路径（与 OpenClaw 一致）"
                    .to_string(),
            ));
        }

        let key = config
            .api_key
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(self.github_token.as_str());

        let (copilot_token, api_base) = resolve_copilot_api_token(key).await?;
        let url = config
            .base_url
            .clone()
            .unwrap_or_else(|| Self::anthropic_url(&api_base));

        let request = AnthropicRequest {
            model: config.model.clone(),
            messages: convert_messages(messages),
            tools: if tools.is_empty() {
                None
            } else {
                Some(convert_tools(tools))
            },
            max_tokens: config.max_tokens.unwrap_or(8192),
            temperature: config.temperature,
            stream: false,
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", copilot_token))
            .header("anthropic-version", "2023-06-01")
            .header("Editor-Version", COPILOT_EDITOR_VERSION)
            .header("User-Agent", COPILOT_USER_AGENT)
            .header("X-Initiator", "user")
            .json(&request)
            .send()
            .await
            .map_err(|e| CoreError::LLMError(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(CoreError::LLMError(format!(
                "Copilot API error: {} - {}",
                status, error_text
            )));
        }

        let anthropic_response: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| CoreError::LLMError(e.to_string()))?;

        Ok(convert_response(anthropic_response))
    }

    async fn chat_stream(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSchema>,
        config: &ModelConfig,
    ) -> Result<mpsc::Receiver<StreamEvent>, CoreError> {
        if !Self::is_claude_model(&config.model) {
            return Err(CoreError::LLMError(
                "GitHub Copilot：流式仅支持含 `claude` 的模型 id".to_string(),
            ));
        }

        let key = config
            .api_key
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(self.github_token.as_str());

        let (copilot_token, api_base) = resolve_copilot_api_token(key).await?;
        let url = config
            .base_url
            .clone()
            .unwrap_or_else(|| Self::anthropic_url(&api_base));

        let request = AnthropicRequest {
            model: config.model.clone(),
            messages: convert_messages(messages),
            tools: if tools.is_empty() {
                None
            } else {
                Some(convert_tools(tools))
            },
            max_tokens: config.max_tokens.unwrap_or(8192),
            temperature: config.temperature,
            stream: true,
        };

        let (tx, rx) = mpsc::channel(100);
        let client = self.client.clone();

        tokio::spawn(async move {
            let response = match client
                .post(&url)
                .header("Authorization", format!("Bearer {}", copilot_token))
                .header("anthropic-version", "2023-06-01")
                .header("Editor-Version", COPILOT_EDITOR_VERSION)
                .header("User-Agent", COPILOT_USER_AGENT)
                .header("X-Initiator", "user")
                .json(&request)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    error!("Copilot stream request failed: {}", e);
                    let _ = tx.send(StreamEvent::Done).await;
                    return;
                }
            };

            if !response.status().is_success() {
                error!("Copilot stream HTTP {}", response.status());
                let _ = tx.send(StreamEvent::Done).await;
                return;
            }

            let mut stream = response.bytes_stream();
            let mut sse_buf = SseLineBuffer::new();
            let mut anth = AnthropicSseStreamState::new();

            'read: while let Some(chunk_res) = stream.next().await {
                let chunk = match chunk_res {
                    Ok(c) => c,
                    Err(e) => {
                        error!("Copilot stream chunk: {}", e);
                        break;
                    }
                };
                let Ok(text) = std::str::from_utf8(&chunk) else {
                    continue;
                };
                for ev in sse_buf.push_str(text) {
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
                            Err(e) => error!("Copilot stream JSON: {}", e),
                        },
                    }
                }
            }
            for ev in sse_buf.finish() {
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
