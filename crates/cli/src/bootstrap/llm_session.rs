//! 从 `Config` 推导会话级 LLM 需求与 `ProviderConfig`（可单测，不依赖完整 runtime）。

use crate::app_config::{default_base_url_for, Config, ModelProfile};
use anycode_llm::{
    normalize_provider_id, read_github_oauth_access_token, transport_for_provider_id, LlmTransport,
    ProviderConfig,
};

pub(crate) fn effective_provider(global: &str, profile: Option<&ModelProfile>) -> String {
    normalize_provider_id(
        profile
            .and_then(|p| p.provider.as_deref())
            .unwrap_or(global),
    )
}

pub(crate) fn scan_session_llm_needs(config: &Config) -> (bool, bool, bool, bool) {
    let mut need_openai = false;
    let mut need_anthropic = false;
    let mut need_bedrock = false;
    let mut need_github_copilot = false;
    let mut note = |pid: &str| match transport_for_provider_id(pid) {
        LlmTransport::OpenAiChatCompletions => need_openai = true,
        LlmTransport::AnthropicMessages => need_anthropic = true,
        LlmTransport::BedrockConverse => need_bedrock = true,
        LlmTransport::GithubCopilot => need_github_copilot = true,
    };
    note(&normalize_provider_id(&config.llm.provider));
    if let Some(ref d) = config.routing.default {
        note(&effective_provider(&config.llm.provider, Some(d)));
    }
    for (_, p) in &config.routing.agents {
        note(&effective_provider(&config.llm.provider, Some(p)));
    }
    (
        need_openai,
        need_anthropic,
        need_bedrock,
        need_github_copilot,
    )
}

pub(crate) fn resolve_openai_shell_config(config: &Config) -> ProviderConfig {
    let g = normalize_provider_id(&config.llm.provider);
    if transport_for_provider_id(&g) == LlmTransport::OpenAiChatCompletions {
        let base_url = if g == "z.ai" {
            config
                .llm
                .base_url
                .clone()
                .or_else(|| Some(default_base_url_for(config.llm.plan.as_str()).to_string()))
        } else {
            config.llm.base_url.clone()
        };
        return ProviderConfig {
            provider: config.llm.provider.clone(),
            api_key: config.llm.api_key.clone(),
            base_url,
            model: config.llm.model.clone(),
            temperature: Some(config.llm.temperature),
            max_tokens: Some(config.llm.max_tokens),
            zai_tool_choice_first_turn: config.llm.zai_tool_choice_first_turn,
        };
    }

    let api_key = config
        .llm
        .provider_credentials
        .get("z.ai")
        .or(config.llm.provider_credentials.get("openai"))
        .or(config.llm.provider_credentials.get("openrouter"))
        .cloned()
        .unwrap_or_default();
    let base_url = Some(default_base_url_for(config.llm.plan.as_str()).to_string());
    ProviderConfig {
        provider: "z.ai".to_string(),
        api_key,
        base_url,
        model: config.llm.model.clone(),
        temperature: Some(config.llm.temperature),
        max_tokens: Some(config.llm.max_tokens),
        zai_tool_choice_first_turn: config.llm.zai_tool_choice_first_turn,
    }
}

pub(crate) fn resolve_anthropic_primary_config(config: &Config) -> anyhow::Result<ProviderConfig> {
    use crate::i18n::tr;

    let g = normalize_provider_id(&config.llm.provider);
    let api_key = if transport_for_provider_id(&g) == LlmTransport::AnthropicMessages {
        if config.llm.api_key.trim().is_empty() {
            anyhow::bail!("{}", tr("err-anthropic-api-key"));
        }
        config.llm.api_key.clone()
    } else {
        config
            .llm
            .provider_credentials
            .get("anthropic")
            .cloned()
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| anyhow::anyhow!("{}", tr("err-anthropic-routing-key")))?
    };
    Ok(ProviderConfig {
        provider: "anthropic".to_string(),
        api_key,
        base_url: config.llm.base_url.clone(),
        model: config.llm.model.clone(),
        temperature: Some(config.llm.temperature),
        max_tokens: Some(config.llm.max_tokens),
        zai_tool_choice_first_turn: false,
    })
}

pub(crate) fn resolve_bedrock_primary_config(config: &Config) -> ProviderConfig {
    ProviderConfig {
        provider: "amazon_bedrock".to_string(),
        api_key: String::new(),
        base_url: config.llm.base_url.clone(),
        model: config.llm.model.clone(),
        temperature: Some(config.llm.temperature),
        max_tokens: Some(config.llm.max_tokens),
        zai_tool_choice_first_turn: false,
    }
}

pub(crate) fn resolve_github_copilot_primary_config(
    config: &Config,
) -> anyhow::Result<ProviderConfig> {
    use crate::i18n::tr;

    let g = normalize_provider_id(&config.llm.provider);
    let api_key = if transport_for_provider_id(&g) == LlmTransport::GithubCopilot {
        if !config.llm.api_key.trim().is_empty() {
            config.llm.api_key.clone()
        } else {
            read_github_oauth_access_token()
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| anyhow::anyhow!("{}", tr("err-github-copilot-token")))?
        }
    } else {
        config
            .llm
            .provider_credentials
            .get("github_copilot")
            .cloned()
            .filter(|s| !s.trim().is_empty())
            .or_else(read_github_oauth_access_token)
            .ok_or_else(|| anyhow::anyhow!("{}", tr("err-github-copilot-routing-key")))?
    };
    Ok(ProviderConfig {
        provider: "github_copilot".to_string(),
        api_key,
        base_url: config.llm.base_url.clone(),
        model: config.llm.model.clone(),
        temperature: Some(config.llm.temperature),
        max_tokens: Some(config.llm.max_tokens),
        zai_tool_choice_first_turn: false,
    })
}

pub(crate) fn resolve_profile_api_key(
    config: &Config,
    profile: &ModelProfile,
    eff_provider: &str,
) -> Option<String> {
    if let Some(ref k) = profile.api_key {
        if !k.trim().is_empty() {
            return Some(k.clone());
        }
    }
    let en = normalize_provider_id(eff_provider);
    if en == normalize_provider_id(&config.llm.provider) {
        if !config.llm.api_key.trim().is_empty() {
            return Some(config.llm.api_key.clone());
        }
    }
    config
        .llm
        .provider_credentials
        .get(en.as_str())
        .cloned()
        .filter(|s| !s.trim().is_empty())
}

pub(crate) fn resolve_agent_base_url(
    config: &Config,
    profile: &ModelProfile,
    global_fallback: &Option<String>,
) -> Option<String> {
    if let Some(ref u) = profile.base_url {
        return Some(u.clone());
    }
    let eff = effective_provider(&config.llm.provider, Some(profile));
    let en = normalize_provider_id(&eff);
    if en == "z.ai" {
        let plan = profile.plan.as_deref().unwrap_or(config.llm.plan.as_str());
        return Some(default_base_url_for(plan).to_string());
    }
    if matches!(
        transport_for_provider_id(&en),
        LlmTransport::AnthropicMessages | LlmTransport::GithubCopilot
    ) {
        return config.llm.base_url.clone();
    }
    if transport_for_provider_id(&en) == LlmTransport::BedrockConverse {
        return config.llm.base_url.clone();
    }
    config
        .llm
        .base_url
        .clone()
        .or_else(|| global_fallback.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_config::{
        LLMConfig, MemoryConfig, RoutingConfig, RuntimeSettings, SecurityConfig, SessionConfig,
        SkillsConfig, StatusLineRuntime,
    };
    use anycode_agent::RuntimePromptConfig;
    use anycode_core::{FeatureRegistry, ModelRouteProfile, RuntimeMode};
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn base_config() -> Config {
        Config {
            llm: LLMConfig {
                provider: "z.ai".to_string(),
                plan: "coding".to_string(),
                model: "glm-5".to_string(),
                api_key: "k".to_string(),
                base_url: None,
                temperature: 0.7,
                max_tokens: 4096,
                provider_credentials: HashMap::new(),
                zai_tool_choice_first_turn: false,
            },
            memory: MemoryConfig {
                path: PathBuf::from("/tmp"),
                auto_save: false,
                backend: "noop".to_string(),
            },
            security: SecurityConfig {
                permission_mode: "default".to_string(),
                require_approval: true,
                sandbox_mode: false,
                mcp_tool_deny_patterns: vec![],
                mcp_tool_deny_rules: vec![],
                always_allow_rules: vec![],
                always_ask_rules: vec![],
                defer_mcp_tools: false,
                session_skip_interactive_approval: false,
            },
            routing: RoutingConfig::default(),
            runtime: RuntimeSettings {
                default_mode: RuntimeMode::Code,
                features: FeatureRegistry::default(),
                model_routes: ModelRouteProfile::default(),
                workspace_project_label: None,
                workspace_channel_profile: None,
            },
            prompt: RuntimePromptConfig::default(),
            skills: SkillsConfig {
                registry_url: None,
                agent_allowlists: std::collections::HashMap::new(),
                ..SkillsConfig::default()
            },
            session: SessionConfig::default(),
            status_line: StatusLineRuntime::default(),
        }
    }

    #[test]
    fn scan_zai_only_needs_openai_compat() {
        let c = base_config();
        let (o, a, b, cp) = scan_session_llm_needs(&c);
        assert!(o);
        assert!(!a);
        assert!(!b);
        assert!(!cp);
    }

    #[test]
    fn scan_mixed_routing_needs_both() {
        let mut c = base_config();
        c.llm.provider = "z.ai".to_string();
        c.routing.agents.insert(
            "plan".to_string(),
            ModelProfile {
                provider: Some("anthropic".to_string()),
                api_key: Some("ak".to_string()),
                plan: None,
                model: None,
                temperature: None,
                max_tokens: None,
                base_url: None,
            },
        );
        let (o, a, b, cp) = scan_session_llm_needs(&c);
        assert!(o, "global z.ai");
        assert!(a, "plan agent uses anthropic");
        assert!(!b);
        assert!(!cp);
    }

    #[test]
    fn resolve_profile_api_key_prefers_profile() {
        let mut c = base_config();
        c.llm.api_key = "global".to_string();
        let p = ModelProfile {
            provider: None,
            api_key: Some("per-agent".to_string()),
            plan: None,
            model: None,
            temperature: None,
            max_tokens: None,
            base_url: None,
        };
        assert_eq!(
            resolve_profile_api_key(&c, &p, "z.ai").as_deref(),
            Some("per-agent")
        );
    }

    #[test]
    fn resolve_agent_base_url_zai_profile_uses_plan_default() {
        let c = base_config();
        let p = ModelProfile {
            provider: Some("z.ai".to_string()),
            plan: Some("general".to_string()),
            model: None,
            temperature: None,
            max_tokens: None,
            base_url: None,
            api_key: None,
        };
        let u = resolve_agent_base_url(&c, &p, &None).expect("url");
        assert!(u.contains("z.ai"));
    }
}
