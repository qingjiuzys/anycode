//! Wire LLM client, tool registry, and `AgentRuntime` (shared by TUI, `run`, and daemon).

mod llm_session;
mod mcp_env;

use crate::app_config::{default_base_url_for, Config};
use crate::i18n::tr_args;
use anycode_agent::{AgentClaudeToolGating, AgentRuntime};
use anycode_core::prelude::*;
use anycode_llm::{build_multi_llm_stack, known_model_aliases, normalize_provider_id, ModelRouter};
use anycode_memory::{FileMemoryStore, HybridMemoryStore};
use anycode_security::{
    ApprovalCallback, InteractiveApprovalCallback, PromptFormat, SecurityLayer, SecurityPolicy,
};
use anycode_tools::{
    build_registry_with_services, catalog, default_skill_roots, validate_default_registry,
    CompiledClaudePermissionRules, SkillCatalog, ToolServices,
};
use async_trait::async_trait;
use fluent_bundle::FluentArgs;
use llm_session::{
    effective_provider, resolve_agent_base_url, resolve_anthropic_primary_config,
    resolve_bedrock_primary_config, resolve_github_copilot_primary_config,
    resolve_openai_shell_config, resolve_profile_api_key, scan_session_llm_needs,
};
use std::collections::HashMap;
use std::collections::HashSet;
use std::io::{stdout, IsTerminal};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::info;

fn compile_tool_name_deny_regexes(patterns: &[String]) -> Vec<regex::Regex> {
    patterns
        .iter()
        .filter_map(|p| {
            let t = p.trim();
            if t.is_empty() {
                return None;
            }
            match regex::Regex::new(t) {
                Ok(re) => Some(re),
                Err(e) => {
                    let mut a = FluentArgs::new();
                    a.set("pat", t.to_string());
                    a.set("err", e.to_string());
                    tracing::warn!(
                        target: "anycode_cli",
                        "{}",
                        tr_args("log-ignore-deny-pattern", &a)
                    );
                    None
                }
            }
        })
        .collect()
}

struct NoopMemoryStore;

#[async_trait]
impl MemoryStore for NoopMemoryStore {
    async fn save(&self, _memory: Memory) -> Result<(), CoreError> {
        Ok(())
    }

    async fn recall(&self, _query: &str, _mem_type: MemoryType) -> Result<Vec<Memory>, CoreError> {
        Ok(vec![])
    }

    async fn update(&self, _id: &str, _memory: Memory) -> Result<(), CoreError> {
        Ok(())
    }

    async fn delete(&self, _id: &str) -> Result<(), CoreError> {
        Ok(())
    }
}

fn sibling_sled_path(file_memory_root: &Path) -> std::path::PathBuf {
    let name = file_memory_root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("memory");
    let parent = file_memory_root.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!("{}.sled", name))
}

fn build_memory_store(
    config: &crate::app_config::MemoryConfig,
) -> anyhow::Result<Arc<dyn MemoryStore>> {
    match config.backend.as_str() {
        "noop" => Ok(Arc::new(NoopMemoryStore)),
        "file" => {
            let store = FileMemoryStore::new(config.path.clone()).map_err(|e| {
                let mut a = FluentArgs::new();
                a.set("err", e.to_string());
                anyhow::anyhow!("{}", tr_args("err-memory-file-store", &a))
            })?;
            Ok(Arc::new(store))
        }
        "hybrid" => {
            let sled_path = sibling_sled_path(&config.path);
            let store = HybridMemoryStore::new(sled_path, config.path.clone()).map_err(|e| {
                let mut a = FluentArgs::new();
                a.set("err", e.to_string());
                anyhow::anyhow!("{}", tr_args("err-memory-hybrid-store", &a))
            })?;
            Ok(Arc::new(store))
        }
        other => {
            let mut a = FluentArgs::new();
            a.set("b", other.to_string());
            anyhow::bail!("{}", tr_args("log-memory-backend-internal", &a))
        }
    }
}

/// Default LLM config + per-agent overrides (before summary/workspace-assistant/goal fill-ins).
pub(crate) fn build_model_routing_parts(
    config: &Config,
) -> (ModelConfig, HashMap<AgentType, ModelConfig>) {
    let g_norm = normalize_provider_id(&config.llm.provider);
    let default_base_url = if g_norm == "z.ai" {
        config
            .llm
            .base_url
            .clone()
            .or_else(|| Some(default_base_url_for(config.llm.plan.as_str()).to_string()))
    } else {
        config.llm.base_url.clone()
    };

    let default_model_config = ModelConfig {
        provider: LLMProvider::Custom(config.llm.provider.clone()),
        model: config.llm.model.clone(),
        base_url: default_base_url.clone(),
        temperature: Some(config.llm.temperature),
        max_tokens: Some(config.llm.max_tokens),
        api_key: None,
    };

    let mut model_overrides: HashMap<AgentType, ModelConfig> = HashMap::new();
    for (agent_type, profile) in config.routing.agents.iter() {
        let eff_p = effective_provider(&config.llm.provider, Some(profile));
        let resolved_model = profile
            .model
            .clone()
            .unwrap_or_else(|| config.llm.model.clone());
        let resolved_temperature = profile.temperature.or(Some(config.llm.temperature));
        let resolved_max_tokens = profile.max_tokens.or(Some(config.llm.max_tokens));
        let resolved_base_url = resolve_agent_base_url(config, profile, &default_base_url);
        let api_key = resolve_profile_api_key(config, profile, &eff_p);
        model_overrides.insert(
            AgentType::new(agent_type.clone()),
            ModelConfig {
                provider: LLMProvider::Custom(eff_p),
                model: resolved_model,
                base_url: resolved_base_url,
                temperature: resolved_temperature,
                max_tokens: resolved_max_tokens,
                api_key,
            },
        );
    }

    (default_model_config, model_overrides)
}

/// Same routing snapshot as runtime (before optional agent fill-ins). For `status` / diagnostics.
pub(crate) fn build_preview_model_router(config: &Config) -> ModelRouter {
    let (default_model_config, model_overrides) = build_model_routing_parts(config);
    ModelRouter::new(
        default_model_config,
        model_overrides,
        config.runtime.model_routes.clone(),
    )
}

/// Shared by the WeChat bridge, TUI, and daemon.
pub(crate) async fn initialize_runtime(
    config: &Config,
    approval_override: Option<Box<dyn ApprovalCallback>>,
) -> anyhow::Result<Arc<AgentRuntime>> {
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

    let llm_client: Arc<dyn anycode_core::LLMClient> =
        build_multi_llm_stack(openai_cfg, anthropic_cfg, bedrock_cfg, copilot_cfg)
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    let mut ls = FluentArgs::new();
    ls.set("openai", format!("{need_openai}"));
    ls.set("anthropic", format!("{need_anthropic}"));
    ls.set("bedrock", format!("{need_bedrock}"));
    ls.set("copilot", format!("{need_github_copilot}"));
    info!(target: "anycode_cli", "{}", tr_args("log-llm-session", &ls));

    let memory_store: Arc<dyn MemoryStore> = build_memory_store(&config.memory)?;
    let mut mi = FluentArgs::new();
    mi.set("backend", config.memory.backend.clone());
    mi.set("path", config.memory.path.display().to_string());
    mi.set("auto", format!("{}", config.memory.auto_save));
    info!(target: "anycode_cli", "{}", tr_args("log-memory-info", &mi));

    let permission_mode = match config.security.permission_mode.as_str() {
        "auto" => PermissionMode::Auto,
        "plan" => PermissionMode::Plan,
        "accept_edits" | "acceptEdits" => PermissionMode::AcceptEdits,
        "bypass" => PermissionMode::BypassPermissions,
        _ => PermissionMode::Default,
    };
    let approval_callback: Option<Box<dyn ApprovalCallback>> = if let Some(cb) = approval_override {
        Some(cb)
    } else if !crate::app_config::security_wants_interactive_approval_callback(config) {
        None
    } else if stdout().is_terminal() {
        Some(Box::new(InteractiveApprovalCallback::new(
            PromptFormat::CLI,
        )))
    } else {
        Some(Box::new(InteractiveApprovalCallback::new(
            PromptFormat::Silent,
        )))
    };
    let security = Arc::new(SecurityLayer::new_with_optional_callback(
        permission_mode,
        approval_callback,
    ));
    let mut bash_policy = SecurityPolicy::interactive_shell();
    bash_policy.sandbox_mode = config.security.sandbox_mode;
    let mut fw_policy = SecurityPolicy::sensitive_mutation();
    fw_policy.sandbox_mode = config.security.sandbox_mode;
    if !config.security.require_approval {
        bash_policy.require_approval = false;
        fw_policy.require_approval = false;
    }
    security
        .set_tool_policy(catalog::TOOL_BASH, bash_policy)
        .await;
    security
        .set_tool_policy(catalog::TOOL_FILE_WRITE, fw_policy.clone())
        .await;

    for t in catalog::SECURITY_SENSITIVE_TOOL_IDS {
        security.set_tool_policy(*t, fw_policy.clone()).await;
    }

    let mcp_defer_gate = if config.security.defer_mcp_tools {
        Some(Arc::new(Mutex::new(HashSet::new())))
    } else {
        None
    };

    let skill_catalog: Arc<SkillCatalog> = Arc::new(if config.skills.enabled {
        let roots = default_skill_roots(&config.skills.extra_dirs, dirs::home_dir().as_deref());
        SkillCatalog::scan(
            &roots,
            config.skills.allowlist.as_deref(),
            config.skills.run_timeout_ms,
            config.skills.minimal_env,
        )
    } else {
        SkillCatalog::scan(
            &[],
            None,
            config.skills.run_timeout_ms,
            config.skills.minimal_env,
        )
    });

    let tool_services: Arc<ToolServices> = {
        let ts = if let Some(h) = dirs::home_dir() {
            let path = h.join(".anycode/tasks/orchestration.json");
            ToolServices::load_or_new_with_mcp_defer(
                path,
                mcp_defer_gate.clone(),
                skill_catalog.clone(),
            )
            .map_err(|e| {
                let mut a = FluentArgs::new();
                a.set("err", e.to_string());
                anyhow::anyhow!("{}", tr_args("err-bootstrap-orch", &a))
            })?
        } else {
            ToolServices::new_ephemeral_with_skills(mcp_defer_gate.clone(), skill_catalog.clone())
        };
        Arc::new(ts)
    };

    let claude_rules = CompiledClaudePermissionRules::compile(
        &config.security.mcp_tool_deny_rules,
        &config.security.always_allow_rules,
        &config.security.always_ask_rules,
    );

    #[cfg(feature = "tools-mcp")]
    {
        use anycode_tools::{mcp_connected::McpConnected, mcp_rmcp_session::McpRmcpSession};
        use mcp_env::McpServerEntry;
        for entry in mcp_env::mcp_server_entries_from_env() {
            match entry {
                McpServerEntry::Stdio { slug, command } => {
                    match anycode_tools::mcp_session::McpStdioSession::connect(&command, &slug)
                        .await
                    {
                        Ok(sess) => {
                            tool_services.attach_mcp_stdio(Arc::new(sess));
                            let mut a = FluentArgs::new();
                            a.set("slug", slug.clone());
                            tracing::info!(target: "anycode_cli", "{}", tr_args("log-mcp-stdio-ok", &a));
                        }
                        Err(e) => {
                            let mut a = FluentArgs::new();
                            a.set("slug", slug.clone());
                            a.set("err", e.to_string());
                            tracing::warn!(
                                target: "anycode_cli",
                                "{}",
                                tr_args("log-mcp-stdio-fail", &a)
                            );
                        }
                    }
                }
                McpServerEntry::Http {
                    slug,
                    url,
                    bearer_token,
                    oauth_credentials_path,
                    headers,
                } => {
                    let connect = async {
                        if let Some(ref cred_path) = oauth_credentials_path {
                            McpRmcpSession::connect_streamable_http_oauth(
                                &url, &slug, cred_path, &headers,
                            )
                            .await
                        } else {
                            McpRmcpSession::connect_streamable_http(
                                &url,
                                &slug,
                                bearer_token.as_deref(),
                                &headers,
                            )
                            .await
                        }
                    };
                    match connect.await {
                        Ok(sess) => {
                            let s: Arc<dyn McpConnected> = Arc::new(sess);
                            tool_services.attach_mcp_session(s);
                            let mut a = FluentArgs::new();
                            a.set("slug", slug.clone());
                            a.set("url", url.clone());
                            tracing::info!(target: "anycode_cli", "{}", tr_args("log-mcp-http-ok", &a));
                        }
                        Err(e) => {
                            let mut a = FluentArgs::new();
                            a.set("slug", slug.clone());
                            a.set("url", url.clone());
                            a.set("err", e.to_string());
                            tracing::warn!(
                                target: "anycode_cli",
                                "{}",
                                tr_args("log-mcp-http-fail", &a)
                            );
                        }
                    }
                }
            }
        }
    }

    let tools = build_registry_with_services(config.security.sandbox_mode, tool_services.clone());
    validate_default_registry(&tools)?;

    for name in tools.keys() {
        if name.starts_with("mcp__") {
            security.set_tool_policy(name, fw_policy.clone()).await;
        }
    }

    let (default_model_config, mut model_overrides) = build_model_routing_parts(config);
    let router = ModelRouter::new(
        default_model_config.clone(),
        model_overrides.clone(),
        config.runtime.model_routes.clone(),
    );
    model_overrides
        .entry(AgentType::new("summary"))
        .or_insert_with(|| router.resolve_summary_model());
    model_overrides
        .entry(AgentType::new("workspace-assistant"))
        .or_insert_with(|| router.resolve_for_mode(&RuntimeMode::Channel));
    model_overrides
        .entry(AgentType::new("goal"))
        .or_insert_with(|| router.resolve_for_mode(&RuntimeMode::Goal));

    let memory_project_autosave_enabled =
        config.memory.auto_save && config.memory.backend != "noop";

    let tool_name_deny = compile_tool_name_deny_regexes(&config.security.mcp_tool_deny_patterns);

    let mut prompt_runtime = config.prompt.clone();
    if config.skills.enabled {
        if let Some(section) = skill_catalog.render_prompt_subsection() {
            prompt_runtime.skills_section = Some(section);
        }
    }
    let ws_extra = match (
        &config.runtime.workspace_project_label,
        &config.runtime.workspace_channel_profile,
    ) {
        (None, None) => String::new(),
        (Some(l), None) => format!("\nProject label: {l}"),
        (None, Some(c)) => format!("\nChannel profile (project): {c}"),
        (Some(l), Some(c)) => format!("\nProject label: {l}\nChannel profile (project): {c}"),
    };
    prompt_runtime.workspace_section = Some(format!(
        "## Workspace Management\nWorkspace registry root: {}\nDefault runtime mode: {}\nEnabled features: {}{}",
        crate::workspace::canonical_root_string(),
        config.runtime.default_mode.as_str(),
        config.runtime.features.enabled().join(", "),
        ws_extra
    ));
    prompt_runtime.channel_section = Some(
        "## Channel Mode\nChannel mode defaults to the workspace assistant. It should prefer read/search/status/workflow behavior and only hand off to coding when explicitly asked."
            .to_string(),
    );
    prompt_runtime.workflow_section = Some(
        "## Workflow\nIf a workspace workflow.yml exists, prefer using it as structured execution guidance before improvising a long multi-step plan."
            .to_string(),
    );
    prompt_runtime.goal_section = Some(
        "## Goal Mode\nFor goal-oriented tasks, keep iterating until completion criteria are met, but stop and surface hard blockers such as missing approvals, missing credentials, or impossible environment requirements.\nWhen `done_when` is set on the goal spec, treat assistant output as complete only if it contains that substring (case-sensitive). Use `GoalSpec.max_attempts_cap` in API/CLI integrations to bound attempts even when infinite retries are enabled."
            .to_string(),
    );
    prompt_runtime.prompt_fragments.push(format!(
        "## Model Routing\nKnown aliases: {}\nMode aliases default to: general=code, explore=fast, plan=plan, channel=channel, goal=best.",
        known_model_aliases().join(", ")
    ));

    let expose_skill_on_explore_plan =
        config.skills.enabled && config.skills.expose_on_explore_plan;

    let runtime = Arc::new(AgentRuntime::new(
        llm_client,
        tools,
        memory_store,
        default_model_config,
        model_overrides,
        Some(DiskTaskOutput::new_default()?),
        security,
        config.security.sandbox_mode,
        prompt_runtime,
        memory_project_autosave_enabled,
        tool_name_deny,
        AgentClaudeToolGating {
            rules: Some(claude_rules),
            defer_mcp_tools: config.security.defer_mcp_tools,
            mcp_defer_allowlist: mcp_defer_gate,
        },
        expose_skill_on_explore_plan,
    ));

    tool_services.attach_sub_agent_executor(runtime.clone());

    Ok(runtime)
}
