//! Assembles LLM stack, tools, security, and [`AgentRuntime`] (`initialize_runtime`).

use crate::app_config::Config;
use crate::i18n::tr_args;
use anycode_agent::{
    AgentClaudeToolGating, AgentRuntime, RuntimeCoreDeps, RuntimeMemoryOptions, RuntimeToolPolicy,
};
use anycode_core::prelude::*;
use anycode_core::DiskTaskOutput;
use anycode_llm::{build_multi_llm_stack, ModelRouter};
use anycode_security::{
    ApprovalCallback, InteractiveApprovalCallback, PromptFormat, SecurityLayer, SecurityPolicy,
};
use anycode_tools::{
    build_registry_with_services, catalog, default_skill_roots, validate_default_registry,
    AskUserQuestionHost, CompiledClaudePermissionRules, LspConnectionConfig, SkillCatalog,
    ToolServices,
};
use fluent_bundle::FluentArgs;
use std::collections::HashSet;
use std::io::{stdin, stdout, IsTerminal};
use std::sync::{Arc, Mutex};
use tracing::info;

use super::llm_session::{
    resolve_anthropic_primary_config, resolve_bedrock_primary_config,
    resolve_github_copilot_primary_config, resolve_openai_shell_config, scan_session_llm_needs,
};
use super::skills_registry;
use super::{build_memory_layer, build_model_routing_parts, compile_tool_name_deny_regexes};

/// Shared by the WeChat bridge, TUI, `run`, and other CLI entrypoints that need a full runtime.
///
/// `ask_user_question_host_override`: when `Some`, used for `AskUserQuestion` (stream REPL / fullscreen TUI).
/// When `None` and stdin+stdout are TTY, falls back to dialoguer on stderr.
pub(crate) async fn initialize_runtime(
    config: &Config,
    approval_override: Option<Box<dyn ApprovalCallback>>,
    ask_user_question_host_override: Option<std::sync::Arc<dyn AskUserQuestionHost>>,
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

    let (memory_store, memory_pipeline) = build_memory_layer(config)?;
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

    let mut merged_extra = config.skills.extra_dirs.clone();
    if let Some(ref ru) = config.skills.registry_url {
        let more = skills_registry::fetch_extra_skill_roots(ru).await;
        merged_extra.extend(more);
    }

    let skill_catalog: Arc<SkillCatalog> = Arc::new(if config.skills.enabled {
        let roots = default_skill_roots(&merged_extra, dirs::home_dir().as_deref());
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
        use super::mcp_env::{self, McpHttpTransport, McpServerEntry};
        use anycode_tools::{mcp_connected::McpConnected, mcp_rmcp_session::McpRmcpSession};
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
                    transport,
                    bearer_token,
                    oauth_credentials_path,
                    headers,
                } => {
                    let connect = async {
                        match transport {
                            McpHttpTransport::LegacySse => {
                                if oauth_credentials_path.is_some() {
                                    Err(CoreError::LLMError(
                                        "MCP legacy SSE 暂不支持 oauth_credentials_path；请用 bearer_token 或改用 streamable-http"
                                            .into(),
                                    ))
                                } else {
                                    anycode_tools::mcp_legacy_sse_session::McpLegacySseSession::connect(
                                        &url,
                                        &slug,
                                        bearer_token.as_deref(),
                                        &headers,
                                    )
                                    .await
                                    .map(|s| Arc::new(s) as Arc<dyn McpConnected>)
                                }
                            }
                            McpHttpTransport::StreamableHttp => {
                                if let Some(ref cred_path) = oauth_credentials_path {
                                    McpRmcpSession::connect_streamable_http_oauth(
                                        &url, &slug, cred_path, &headers,
                                    )
                                    .await
                                    .map(|s| Arc::new(s) as Arc<dyn McpConnected>)
                                } else {
                                    McpRmcpSession::connect_streamable_http(
                                        &url,
                                        &slug,
                                        bearer_token.as_deref(),
                                        &headers,
                                    )
                                    .await
                                    .map(|s| Arc::new(s) as Arc<dyn McpConnected>)
                                }
                            }
                        }
                    };
                    match connect.await {
                        Ok(sess) => {
                            tool_services.attach_mcp_session(sess);
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

    let lsp_cmd = if config.lsp.enabled {
        config.lsp.command.clone()
    } else {
        None
    };
    tool_services.set_lsp_connection_config(LspConnectionConfig {
        command: lsp_cmd,
        workspace_root: config.lsp.workspace_root.clone(),
        read_timeout: std::time::Duration::from_millis(config.lsp.read_timeout_ms),
    });

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
    super::prompt_runtime::augment_prompt_runtime(
        config,
        skill_catalog.as_ref(),
        &mut prompt_runtime,
    );

    let expose_skill_on_explore_plan =
        config.skills.enabled && config.skills.expose_on_explore_plan;

    let memory_pipeline_settings = if config.memory.backend == "pipeline" {
        Some(config.memory.pipeline.clone())
    } else {
        None
    };

    let runtime = Arc::new(AgentRuntime::new(
        RuntimeCoreDeps {
            llm_client,
            tools,
            memory_store,
            default_model_config,
            model_overrides,
            disk_output: Some(DiskTaskOutput::new_default()?),
            security,
            sandbox_mode: config.security.sandbox_mode,
            prompt_config: prompt_runtime,
        },
        RuntimeMemoryOptions {
            memory_pipeline,
            memory_pipeline_settings,
            memory_project_autosave_enabled,
        },
        RuntimeToolPolicy {
            tool_name_deny,
            claude_gating: AgentClaudeToolGating {
                rules: Some(claude_rules),
                defer_mcp_tools: config.security.defer_mcp_tools,
                mcp_defer_allowlist: mcp_defer_gate,
            },
            expose_skill_on_explore_plan,
        },
    ));

    tool_services.attach_sub_agent_executor(runtime.clone());

    let ask_host: Option<std::sync::Arc<dyn AskUserQuestionHost>> = ask_user_question_host_override
        .or_else(|| {
            if stdin().is_terminal() && stdout().is_terminal() {
                Some(
                    std::sync::Arc::new(crate::ask_user_host::DialoguerAskUserQuestionHost)
                        as std::sync::Arc<dyn AskUserQuestionHost>,
                )
            } else {
                None
            }
        });
    if let Some(h) = ask_host {
        tool_services.attach_ask_user_question_host(h);
    }

    Ok(runtime)
}
