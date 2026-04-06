//! 多后端 LLM：OpenAI 兼容、Anthropic、Bedrock、GitHub Copilot 按 `ModelConfig.provider` 分发。

use crate::provider_catalog::transport_for_provider_id;
use crate::provider_catalog::LlmTransport;
use anycode_core::prelude::*;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc;

fn provider_str(mc: &ModelConfig) -> String {
    match &mc.provider {
        LLMProvider::Custom(s) => s.clone(),
        LLMProvider::Anthropic => "anthropic".to_string(),
        LLMProvider::OpenAI => "openai".to_string(),
        LLMProvider::Local => "local".to_string(),
    }
}

/// 同时持有多种可选客户端，按目录 [`LlmTransport`] 分发。
pub struct MultiProviderLlmClient {
    chat_completions: Option<Arc<dyn LLMClient>>,
    anthropic: Option<Arc<dyn LLMClient>>,
    bedrock: Option<Arc<dyn LLMClient>>,
    github_copilot: Option<Arc<dyn LLMClient>>,
}

impl MultiProviderLlmClient {
    pub fn new(
        chat_completions: Option<Arc<dyn LLMClient>>,
        anthropic: Option<Arc<dyn LLMClient>>,
        bedrock: Option<Arc<dyn LLMClient>>,
        github_copilot: Option<Arc<dyn LLMClient>>,
    ) -> Self {
        Self {
            chat_completions,
            anthropic,
            bedrock,
            github_copilot,
        }
    }

    fn transport(&self, config: &ModelConfig) -> LlmTransport {
        let id = provider_str(config);
        transport_for_provider_id(&id)
    }
}

#[async_trait]
impl LLMClient for MultiProviderLlmClient {
    async fn chat(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSchema>,
        config: &ModelConfig,
    ) -> Result<LLMResponse, CoreError> {
        match self.transport(config) {
            LlmTransport::AnthropicMessages => {
                let Some(ref a) = self.anthropic else {
                    return Err(CoreError::LLMError(
                        "当前会话未初始化 Anthropic 客户端（检查 provider 与 api_key）".to_string(),
                    ));
                };
                a.chat(messages, tools, config).await
            }
            LlmTransport::OpenAiChatCompletions => {
                let Some(ref c) = self.chat_completions else {
                    return Err(CoreError::LLMError(
                        "未初始化 OpenAI 兼容客户端（检查 provider / api_key / base_url）"
                            .to_string(),
                    ));
                };
                c.chat(messages, tools, config).await
            }
            LlmTransport::BedrockConverse => {
                let Some(ref b) = self.bedrock else {
                    return Err(CoreError::LLMError(
                        "未初始化 Amazon Bedrock 客户端（检查 AWS 凭证与 model）".to_string(),
                    ));
                };
                b.chat(messages, tools, config).await
            }
            LlmTransport::GithubCopilot => {
                let Some(ref g) = self.github_copilot else {
                    return Err(CoreError::LLMError(
                        "未初始化 GitHub Copilot 客户端（GitHub token 或 `anycode model auth copilot`）"
                            .to_string(),
                    ));
                };
                g.chat(messages, tools, config).await
            }
        }
    }

    async fn chat_stream(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSchema>,
        config: &ModelConfig,
    ) -> Result<mpsc::Receiver<StreamEvent>, CoreError> {
        match self.transport(config) {
            LlmTransport::AnthropicMessages => {
                let Some(ref a) = self.anthropic else {
                    return Err(CoreError::LLMError(
                        "当前会话未初始化 Anthropic 客户端".to_string(),
                    ));
                };
                a.chat_stream(messages, tools, config).await
            }
            LlmTransport::OpenAiChatCompletions => {
                let Some(ref c) = self.chat_completions else {
                    return Err(CoreError::LLMError(
                        "未初始化 OpenAI 兼容客户端".to_string(),
                    ));
                };
                c.chat_stream(messages, tools, config).await
            }
            LlmTransport::BedrockConverse => {
                let Some(ref b) = self.bedrock else {
                    return Err(CoreError::LLMError(
                        "未初始化 Amazon Bedrock 客户端".to_string(),
                    ));
                };
                b.chat_stream(messages, tools, config).await
            }
            LlmTransport::GithubCopilot => {
                let Some(ref g) = self.github_copilot else {
                    return Err(CoreError::LLMError(
                        "未初始化 GitHub Copilot 客户端".to_string(),
                    ));
                };
                g.chat_stream(messages, tools, config).await
            }
        }
    }
}
