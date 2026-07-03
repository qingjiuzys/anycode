//! Session-scoped LLM client stack (`build_multi_llm_stack` wiring).

use anycode_config::Config;
use anycode_llm::build_multi_llm_stack;
use std::sync::Arc;
use tracing::info;

use super::llm_session::{
    resolve_anthropic_primary_config, resolve_bedrock_primary_config,
    resolve_github_copilot_primary_config, resolve_openai_shell_config, scan_session_llm_needs,
};

pub async fn build_llm_stack(config: &Config) -> anyhow::Result<Arc<dyn anycode_core::LLMClient>> {
    let (need_openai, need_anthropic, need_bedrock, need_github_copilot) =
        scan_session_llm_needs(config);

    let openai_cfg = if need_openai {
        Some(resolve_openai_shell_config(config))
    } else {
        None
    };

    let anthropic_cfg = if need_anthropic {
        Some(resolve_anthropic_primary_config(config)?)
    } else {
        None
    };

    let bedrock_cfg = if need_bedrock {
        Some(resolve_bedrock_primary_config(config))
    } else {
        None
    };

    let copilot_cfg = if need_github_copilot {
        Some(resolve_github_copilot_primary_config(config)?)
    } else {
        None
    };

    let llm_client = build_multi_llm_stack(openai_cfg, anthropic_cfg, bedrock_cfg, copilot_cfg)
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    info!(
        target: "anycode_bootstrap",
        openai = need_openai,
        anthropic = need_anthropic,
        bedrock = need_bedrock,
        copilot = need_github_copilot,
        "LLM stack built"
    );

    Ok(llm_client)
}
