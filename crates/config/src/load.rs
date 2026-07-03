//! Load `AnyCodeConfig` into runtime `Config`.

use crate::schema::*;
use crate::user_config::*;
use crate::workspace::apply_project_overlays;
use anycode_agent::RuntimePromptConfig;
use anycode_core::FeatureRegistry;
use std::path::{Path, PathBuf};
use tracing::info;

fn resolve_model_instructions_file_from_env() -> Option<PathBuf> {
    std::env::var("ANYCODE_MODEL_INSTRUCTIONS_FILE")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
}

pub async fn load_config(config_file: Option<PathBuf>) -> anyhow::Result<Config> {
    let default_path = resolve_config_path(None)?;
    let mut cfg = match load_anycode_config_resolved(config_file.clone())? {
        Some(c) => c,
        None => {
            eprintln!(
                "warning: no config at {}; using defaults (run setup in the app)",
                default_path.display()
            );
            default_anycode_config()
        }
    };

    validate_permission_mode(cfg.security.permission_mode.trim())?;
    let runtime_mode = validate_runtime_mode(cfg.runtime.default_mode.trim())?;
    validate_llm_provider(&cfg.provider)?;
    validate_notifications(&cfg.notifications)?;

    if let Ok(v) = serde_json::to_value(&cfg) {
        let reg = anycode_llm::ResolvedModelRegistry::from_config(&v);
        if let Some(item) = reg.active_item(anycode_llm::ModelCapability::Chat) {
            cfg.provider = item.provider.clone();
            cfg.model = item.model.clone();
            if let Some(p) = item.plan.as_ref() {
                cfg.plan = p.clone();
            }
            if let Some(u) = item.base_url.as_ref() {
                cfg.base_url = Some(u.clone());
            }
            if let Some(k) = reg.resolve_api_key(item) {
                cfg.api_key = k;
            }
        }
    }

    let config_path = resolve_config_path(config_file.clone())?;
    let base_dir = config_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    let system_prompt_override = match cfg.system_prompt_override.as_deref() {
        None | Some("") => None,
        Some(s) => {
            let v = resolve_system_prompt_field(s.trim(), base_dir)?;
            let t = v.trim();
            if t.is_empty() {
                None
            } else {
                Some(v)
            }
        }
    };
    let system_prompt_append = match cfg.system_prompt_append.as_deref() {
        None | Some("") => None,
        Some(s) => {
            let v = resolve_system_prompt_field(s.trim(), base_dir)?;
            let t = v.trim();
            if t.is_empty() {
                None
            } else {
                Some(v)
            }
        }
    };

    let memory_path = resolve_memory_directory(cfg.memory.path.clone())?;
    let memory_backend = normalize_memory_backend(&cfg.memory.backend)?;

    let embedding_provider =
        normalize_embedding_provider(cfg.memory.pipeline.embedding_provider.as_deref())?;
    let mut pipeline = merge_memory_pipeline_settings(&cfg.memory.pipeline);
    if embedding_provider == "local" {
        pipeline.embedding_enabled = true;
    }

    let model_instructions_file = resolve_model_instructions_file_from_env();

    let mut lsp_runtime: LspRuntime = cfg.lsp.clone().into();
    if let Some(ref p) = lsp_runtime.workspace_root {
        if p.as_os_str().is_empty() {
            lsp_runtime.workspace_root = None;
        } else {
            let full = if p.is_absolute() {
                p.clone()
            } else {
                base_dir.join(p)
            };
            lsp_runtime.workspace_root = std::fs::canonicalize(&full).ok().or(Some(full));
        }
    }

    let mut wechat_history: WechatHistoryRuntime = cfg.wechat_history.clone().into();
    if let Some(ref p) = wechat_history.config.data_dir {
        if !p.as_os_str().is_empty() {
            let full = if p.is_absolute() {
                p.clone()
            } else {
                base_dir.join(p)
            };
            wechat_history.config.data_dir = std::fs::canonicalize(&full).ok().or(Some(full));
        }
    }
    if let Some(ref p) = wechat_history.config.key_map_path {
        if !p.as_os_str().is_empty() {
            let full = if p.is_absolute() {
                p.clone()
            } else {
                base_dir.join(p)
            };
            wechat_history.config.key_map_path = std::fs::canonicalize(&full).ok().or(Some(full));
        }
    }

    Ok(Config {
        llm: LLMConfig {
            provider: cfg.provider,
            plan: cfg.plan,
            model: cfg.model,
            api_key: cfg.api_key,
            base_url: cfg.base_url,
            temperature: cfg.temperature,
            max_tokens: cfg.max_tokens,
            provider_credentials: cfg.provider_credentials,
            zai_tool_choice_first_turn: cfg.zai_tool_choice_first_turn,
        },
        memory: MemoryConfig {
            path: memory_path,
            auto_save: cfg.memory.auto_save,
            backend: memory_backend,
            pipeline,
            embedding_model: cfg
                .memory
                .pipeline
                .embedding_model
                .clone()
                .filter(|s| !s.trim().is_empty()),
            embedding_base_url: cfg
                .memory
                .pipeline
                .embedding_base_url
                .clone()
                .filter(|s| !s.trim().is_empty()),
            embedding_provider,
            embedding_local_cache_dir: resolve_embedding_local_cache_dir(
                cfg.memory.pipeline.embedding_local_cache_dir.clone(),
            )?,
            embedding_local_model: cfg
                .memory
                .pipeline
                .embedding_local_model
                .clone()
                .filter(|s| !s.trim().is_empty()),
            embedding_hf_endpoint: cfg
                .memory
                .pipeline
                .embedding_hf_endpoint
                .clone()
                .filter(|s| !s.trim().is_empty()),
        },
        security: SecurityConfig {
            permission_mode: cfg.security.permission_mode.clone(),
            require_approval: cfg.security.require_approval,
            sandbox_mode: cfg.security.sandbox_mode,
            mcp_tool_deny_patterns: cfg.security.mcp_tool_deny_patterns.clone(),
            mcp_tool_deny_rules: cfg.security.mcp_tool_deny_rules.clone(),
            always_allow_rules: cfg.security.always_allow_rules.clone(),
            always_ask_rules: cfg.security.always_ask_rules.clone(),
            defer_mcp_tools: cfg.security.defer_mcp_tools,
            session_skip_interactive_approval: false,
        },
        routing: cfg.routing,
        runtime: RuntimeSettings {
            default_mode: runtime_mode,
            features: FeatureRegistry::from_enabled(cfg.runtime.enabled_features),
            model_routes: cfg.runtime.model_routes,
            tool_policy_profiles: cfg.runtime.tool_policy_profiles.into(),
            tool_deny_names: cfg.runtime.tool_deny_names.clone(),
            tool_deny_prefixes: cfg.runtime.tool_deny_prefixes.clone(),
            model_fallback: cfg.runtime.model_fallback.clone(),
            max_agent_turns: cfg.runtime.max_agent_turns,
            max_tool_calls: cfg.runtime.max_tool_calls,
            workspace_project_label: None,
            workspace_channel_profile: None,
        },
        prompt: RuntimePromptConfig {
            system_prompt_override,
            system_prompt_append,
            skills_section: None,
            skills_section_by_agent: std::collections::HashMap::new(),
            workspace_section: None,
            channel_section: None,
            workflow_section: None,
            goal_section: None,
            prompt_fragments: vec![],
            model_instructions: cfg.model_instructions.into(),
            model_instructions_file,
            model_instructions_content: None,
        },
        skills: cfg.skills.into(),
        agents: cfg.agents.into(),
        session: cfg.session.into(),
        status_line: cfg.status_line.into(),
        terminal: cfg.terminal.into(),
        channels: cfg.channels.into(),
        lsp: lsp_runtime,
        mcp: cfg.mcp.clone().into(),
        notifications: cfg.notifications,
        wechat_history,
    })
}

#[derive(Debug, Clone, Default)]
pub struct LoadOpts {
    pub config_file: Option<PathBuf>,
    pub ignore_approval: bool,
    pub workspace_overlay: bool,
    pub workspace_overlay_dir: Option<PathBuf>,
    pub wechat_bridge: bool,
}

pub async fn load_runtime_config(opts: LoadOpts) -> anyhow::Result<Config> {
    let mut config = load_config_for_session(opts.config_file, opts.ignore_approval).await?;
    if opts.wechat_bridge {
        apply_channel_self_hosted_security(&mut config);
    }
    if let Some(dir) = opts.workspace_overlay_dir {
        let wd = std::fs::canonicalize(&dir).unwrap_or(dir);
        apply_project_overlays(&mut config, &wd);
    } else if opts.workspace_overlay {
        if let Ok(cwd) = std::env::current_dir() {
            let wd = std::fs::canonicalize(&cwd).unwrap_or(cwd);
            apply_project_overlays(&mut config, &wd);
        }
    }
    Ok(config)
}

pub async fn load_config_for_session(
    config_file: Option<PathBuf>,
    ignore_approval: bool,
) -> anyhow::Result<Config> {
    let mut config = load_config(config_file).await?;
    apply_ignore_approval_cli(&mut config, ignore_approval);
    Ok(config)
}

fn apply_ignore_approval_cli(config: &mut Config, ignore_approval: bool) {
    config.security.session_skip_interactive_approval = ignore_approval;
    if ignore_approval {
        if config.security.require_approval {
            info!(target: "anycode_config", "session: ignoring tool approval for this process");
        }
        config.security.require_approval = false;
    }
}

pub fn security_wants_interactive_approval_callback(config: &Config) -> bool {
    !config.security.session_skip_interactive_approval
        && (config.security.require_approval || !config.security.always_ask_rules.is_empty())
}

pub fn apply_channel_self_hosted_security(config: &mut Config) {
    if config.security.require_approval {
        info!(
            target: "anycode_config",
            "channel self-hosted: disabling interactive tool approval"
        );
        config.security.require_approval = false;
    }
    config.security.session_skip_interactive_approval = true;
    if !config.security.always_ask_rules.is_empty() {
        tracing::debug!(
            target: "anycode_config",
            "channel self-hosted: clearing {} always_ask_rules for this process",
            config.security.always_ask_rules.len()
        );
        config.security.always_ask_rules.clear();
    }
}
