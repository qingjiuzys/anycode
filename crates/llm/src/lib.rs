//! anyCode LLM Clients
//!
//! 支持多个 LLM 提供商（目录见 [`provider_catalog`]）。

use anycode_core::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

mod chat_model_ref;
pub mod copilot_token;
mod google_catalog;
mod model_catalog;
mod model_context;
mod model_router;
mod multi_client;
mod provider_catalog;
mod providers;
mod retry_strategy;
mod sse_data_lines;

pub use chat_model_ref::{
    build_qualified_chat_model_value, resolve_chat_model_ref, zai_model_catalog_entries,
    ChatModelResolution, ChatModelResolutionReason, ChatModelResolutionSource, ModelCatalogEntry,
};
pub use copilot_token::{
    anycode_credentials_dir, copilot_token_cache_path, github_oauth_token_path,
    read_github_oauth_access_token, resolve_copilot_api_token,
};
pub use google_catalog::{is_known_google_model_id, GoogleModelCatalogEntry, GOOGLE_MODEL_CATALOG};
pub use model_catalog::{
    clone_with_model, is_known_model_alias, known_model_aliases, ModelAliasDescriptor,
    MODEL_ALIASES,
};
pub use model_context::{resolve_context_window_tokens, DEFAULT_CONTEXT_WINDOW_TOKENS};
pub use model_router::ModelRouter;
pub use multi_client::MultiProviderLlmClient;
pub use provider_catalog::{
    catalog_lookup, is_known_provider_id, normalize_provider_id, transport_for_provider_id,
    LlmTransport, ProviderCatalogEntry, PROVIDER_CATALOG, ROUTING_AGENT_PRESETS, ZAI_AUTH_METHODS,
};
pub use retry_strategy::{
    is_retryable_status as retry_is_retryable_status, retry_delay_ms as retry_strategy_delay_ms,
    ErrorCategory, JitterRetryStrategy, ProviderRetryConfig, RetryConfig, RetryStrategy,
};

// ============================================================================
// anyCode 多模型门面
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// provider 标识（与 `PROVIDER_CATALOG` 中 `id` 对齐，如 z.ai、openrouter）
    pub provider: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub model: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    /// OpenAI 兼容栈：首轮 agent 请求在带 tools 时倾向 `tool_choice: required`（环境变量优先）。
    #[serde(default)]
    pub zai_tool_choice_first_turn: bool,
}

/// OpenAI Chat Completions 兼容客户端（内部为 [`ZaiClient`]）。
pub fn build_zai_openai_stack_client(
    cfg: &ProviderConfig,
) -> Result<providers::zai::ZaiClient, CoreError> {
    let norm = normalize_provider_id(&cfg.provider);
    match transport_for_provider_id(&norm) {
        LlmTransport::OpenAiChatCompletions => {
            let mut client =
                providers::zai::ZaiClient::new(cfg.api_key.clone(), Some(cfg.model.clone()));
            if let Some(ref u) = cfg.base_url {
                client = client.with_base_url(u.clone());
            } else if norm != "z.ai" {
                return Err(CoreError::LLMError(format!(
                    "provider `{}` 须配置 base_url（OpenAI 兼容 Chat Completions 完整 URL）",
                    cfg.provider
                )));
            }
            if cfg.zai_tool_choice_first_turn {
                client = client.with_tool_choice_first_turn(true);
            }
            Ok(client)
        }
        _ => Err(CoreError::LLMError(format!(
            "provider `{}` 不是 OpenAI Chat Completions 兼容栈",
            cfg.provider
        ))),
    }
}

/// 单后端：与全局 `provider` 一致时使用（含 Bedrock / Copilot 等异步初始化路径）。
pub async fn build_llm_client(cfg: &ProviderConfig) -> Result<Arc<dyn LLMClient>, CoreError> {
    let norm = normalize_provider_id(&cfg.provider);
    match transport_for_provider_id(&norm) {
        LlmTransport::AnthropicMessages => {
            let client = providers::anthropic::AnthropicClient::new(cfg.api_key.clone())
                .map_err(|e| CoreError::LLMError(format!("anthropic client: {}", e)))?;
            let client = if let Some(ref u) = cfg.base_url {
                client.with_base_url(u.clone())
            } else {
                client
            };
            Ok(Arc::new(client))
        }
        LlmTransport::OpenAiChatCompletions => Ok(Arc::new(build_zai_openai_stack_client(cfg)?)),
        LlmTransport::BedrockConverse => {
            let client = providers::bedrock::BedrockClient::from_provider_config(cfg).await?;
            Ok(Arc::new(client))
        }
        LlmTransport::GithubCopilot => {
            let token = if cfg.api_key.trim().is_empty() {
                copilot_token::read_github_oauth_access_token().ok_or_else(|| {
                    CoreError::LLMError(
                        "GitHub Copilot：请在 config 填写 api_key，或运行 `anycode model auth copilot`"
                            .to_string(),
                    )
                })?
            } else {
                cfg.api_key.clone()
            };
            let client = providers::github_copilot::GithubCopilotClient::new(token)
                .map_err(|e| CoreError::LLMError(format!("github copilot client: {}", e)))?;
            Ok(Arc::new(client))
        }
    }
}

/// 多后端：全局与 `routing.agents` 可混用多种传输。
pub async fn build_multi_llm_stack(
    chat_completions_provider: Option<ProviderConfig>,
    anthropic: Option<ProviderConfig>,
    bedrock: Option<ProviderConfig>,
    github_copilot: Option<ProviderConfig>,
) -> Result<Arc<dyn LLMClient>, CoreError> {
    let chat_completions: Option<Arc<dyn LLMClient>> =
        if let Some(ref c) = chat_completions_provider {
            #[cfg(feature = "openai")]
            {
                let norm = normalize_provider_id(&c.provider);
                if norm == "openai" {
                    let cli = providers::openai::OpenAIClient::new(c.api_key.clone())
                        .map_err(|e| CoreError::LLMError(format!("openai client: {}", e)))?;
                    let cli = if let Some(ref u) = c.base_url {
                        cli.with_base_url(u.clone())
                    } else {
                        cli
                    };
                    Some(Arc::new(cli) as Arc<dyn LLMClient>)
                } else {
                    Some(Arc::new(build_zai_openai_stack_client(c)?) as Arc<dyn LLMClient>)
                }
            }
            #[cfg(not(feature = "openai"))]
            {
                Some(Arc::new(build_zai_openai_stack_client(c)?) as Arc<dyn LLMClient>)
            }
        } else {
            None
        };

    let anthropic_client: Option<Arc<dyn LLMClient>> = if let Some(ref c) = anthropic {
        let client = providers::anthropic::AnthropicClient::new(c.api_key.clone())
            .map_err(|e| CoreError::LLMError(format!("anthropic client: {}", e)))?;
        let client = if let Some(ref u) = c.base_url {
            client.with_base_url(u.clone())
        } else {
            client
        };
        Some(Arc::new(client) as Arc<dyn LLMClient>)
    } else {
        None
    };

    let bedrock_client: Option<Arc<dyn LLMClient>> = if let Some(ref c) = bedrock {
        let client = providers::bedrock::BedrockClient::from_provider_config(c).await?;
        Some(Arc::new(client) as Arc<dyn LLMClient>)
    } else {
        None
    };

    let copilot_client: Option<Arc<dyn LLMClient>> = if let Some(ref c) = github_copilot {
        let token = if c.api_key.trim().is_empty() {
            copilot_token::read_github_oauth_access_token().ok_or_else(|| {
                CoreError::LLMError(
                    "routing 使用 GitHub Copilot 但未配置 api_key，且缺少 ~/.anycode/credentials/github-oauth.json"
                        .to_string(),
                )
            })?
        } else {
            c.api_key.clone()
        };
        let client = providers::github_copilot::GithubCopilotClient::new(token)
            .map_err(|e| CoreError::LLMError(format!("github copilot: {}", e)))?;
        Some(Arc::new(client) as Arc<dyn LLMClient>)
    } else {
        None
    };

    if chat_completions.is_none()
        && anthropic_client.is_none()
        && bedrock_client.is_none()
        && copilot_client.is_none()
    {
        return Err(CoreError::LLMError(
            "至少需要配置一种 LLM 后端（OpenAI 兼容、Anthropic、Bedrock 或 GitHub Copilot）"
                .to_string(),
        ));
    }

    Ok(Arc::new(MultiProviderLlmClient::new(
        chat_completions,
        anthropic_client,
        bedrock_client,
        copilot_client,
    )))
}

#[derive(Error, Debug)]
pub enum LLMError {
    #[error("API key is missing")]
    MissingApiKey,

    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Stream error: {0}")]
    StreamError(String),
}
pub use providers::anthropic::AnthropicClient;
pub use providers::zai::{
    zai_default_chat_url_for_plan, zai_model_display_name, ZaiClient, ZaiModel,
    ZaiModelCatalogEntry, ZAI_CN_CODING_URL, ZAI_CN_GENERAL_URL, ZAI_DEFAULT_CODING_ENDPOINT,
    ZAI_GLOBAL_CODING_URL, ZAI_GLOBAL_GENERAL_URL, ZAI_MODEL_CATALOG,
};

#[cfg(feature = "openai")]
pub use providers::openai::OpenAIClient;
