//! Resolve per-task tool deny lists from config, env, and execution surface.

use crate::app_config::Config;
use crate::tasks::RunTaskOptions;
use anycode_tools::{
    detect_ci_environment, resolve_runtime_tool_filters, RuntimeToolPolicyInput,
    ToolExecutionSurface,
};

pub(crate) fn resolve_task_tool_filters(
    config: &Config,
    surface: ToolExecutionSurface,
    options: &RunTaskOptions,
) -> (Vec<String>, Vec<String>) {
    resolve_runtime_tool_filters(RuntimeToolPolicyInput {
        surface,
        profiles: &config.runtime.tool_policy_profiles,
        explicit_profile: options.tool_profile.as_deref(),
        explicit_allowlist: options.tool_allowlist.as_deref(),
        extra_deny_names: &config.runtime.tool_deny_names,
        extra_deny_prefixes: &config.runtime.tool_deny_prefixes,
    })
}

pub(crate) fn headless_task_surface() -> ToolExecutionSurface {
    if detect_ci_environment() {
        ToolExecutionSurface::Ci
    } else {
        ToolExecutionSurface::Headless
    }
}

pub(crate) fn resolve_headless_task_tool_filters(
    options: &RunTaskOptions,
) -> (Vec<String>, Vec<String>) {
    resolve_runtime_tool_filters(RuntimeToolPolicyInput {
        surface: headless_task_surface(),
        profiles: &anycode_tools::ToolPolicyProfiles::default(),
        explicit_profile: options.tool_profile.as_deref(),
        explicit_allowlist: options.tool_allowlist.as_deref(),
        extra_deny_names: &[],
        extra_deny_prefixes: &[],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use anycode_config::{LLMConfig, MemoryConfig, RuntimeSettings, SecurityConfig};
    use anycode_core::{FeatureRegistry, ModelRouteProfile, RuntimeMode};
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn minimal_config() -> Config {
        Config {
            llm: LLMConfig {
                provider: "mock".into(),
                plan: "coding".into(),
                model: "mock".into(),
                api_key: "k".into(),
                base_url: None,
                temperature: 0.7,
                max_tokens: 4096,
                provider_credentials: HashMap::new(),
                zai_tool_choice_first_turn: false,
            },
            memory: MemoryConfig {
                path: PathBuf::from("/tmp"),
                auto_save: false,
                backend: "noop".into(),
                pipeline: anycode_core::MemoryPipelineSettings::default(),
                embedding_model: None,
                embedding_base_url: None,
                embedding_provider: "http".into(),
                embedding_local_cache_dir: None,
                embedding_local_model: None,
                embedding_hf_endpoint: None,
            },
            security: SecurityConfig {
                permission_mode: "default".into(),
                require_approval: true,
                sandbox_mode: false,
                mcp_tool_deny_patterns: vec![],
                mcp_tool_deny_rules: vec![],
                always_allow_rules: vec![],
                always_ask_rules: vec![],
                defer_mcp_tools: false,
                session_skip_interactive_approval: false,
            },
            routing: Default::default(),
            runtime: RuntimeSettings {
                default_mode: RuntimeMode::Code,
                features: FeatureRegistry::default(),
                model_routes: ModelRouteProfile::default(),
                tool_policy_profiles: Default::default(),
                tool_deny_names: vec![],
                tool_deny_prefixes: vec![],
                model_fallback: None,
                max_agent_turns: None,
                max_tool_calls: None,
                workspace_project_label: None,
                workspace_channel_profile: None,
            },
            prompt: Default::default(),
            skills: Default::default(),
            agents: Default::default(),
            session: Default::default(),
            status_line: Default::default(),
            terminal: Default::default(),
            lsp: Default::default(),
            mcp: Default::default(),
            notifications: Default::default(),
        }
    }

    #[test]
    fn headless_filters_apply_explicit_profile() {
        let config = minimal_config();
        let options = RunTaskOptions {
            tool_profile: Some("read_only".into()),
            ..RunTaskOptions::default()
        };
        let (names, _) =
            resolve_task_tool_filters(&config, ToolExecutionSurface::Headless, &options);
        assert!(names.iter().any(|n| n == "Bash"));
    }
}
