//! Assembles LLM stack, tools, security, and [`AgentRuntime`] (`initialize_runtime`).

use crate::agents::build_agents_setup;
use crate::llm_stack::build_llm_stack;
use crate::security_setup::build_security_setup;
use crate::tools_setup::build_tools_setup;
use crate::{
    build_failover_policy, build_memory_layer, build_model_routing_parts,
    compile_tool_name_deny_regexes, effective_memory_backend, MemoryAttachMode,
};
use anycode_agent::{
    AgentClaudeToolGating, AgentRuntime, RuntimeCoreDeps, RuntimeMemoryOptions, RuntimeToolPolicy,
};
use anycode_config::Config;
use anycode_core::prelude::*;
use anycode_core::DiskTaskOutput;
use anycode_llm::ModelRouter;
use anycode_security::ApprovalCallback;
use anycode_tools::AskUserQuestionHost;
use std::collections::HashSet;
use std::io::{stdin, stdout, IsTerminal};
use std::sync::Arc;
use tracing::info;

/// Injectable hosts for approval, ask-user, and optional TTY dialoguer fallback.
#[derive(Default)]
pub struct RuntimeHosts {
    pub approval_override: Option<Box<dyn ApprovalCallback>>,
    pub ask_user_question_host: Option<Arc<dyn AskUserQuestionHost>>,
    /// When true and no ask host is set, use dialoguer on TTY (`dialoguer-host` feature).
    pub dialoguer_on_tty: bool,
}

/// Shared composition root for dashboard, daemon, and legacy CLI paths.
pub async fn initialize_runtime(
    config: &Config,
    hosts: RuntimeHosts,
    memory_attach: MemoryAttachMode,
    project_enabled: Option<HashSet<String>>,
) -> anyhow::Result<Arc<AgentRuntime>> {
    if std::env::var_os("ANYCODE_REPLY_LANG").is_none() {
        std::env::set_var(
            "ANYCODE_REPLY_LANG",
            anycode_locale::resolve_locale().as_str(),
        );
    }
    let llm_client = build_llm_stack(config).await?;

    let (memory_store, memory_pipeline) = build_memory_layer(config, memory_attach)?;
    info!(
        target: "anycode_bootstrap",
        backend = %config.memory.backend,
        attach = %memory_attach.as_str(),
        effective = %effective_memory_backend(config, memory_attach),
        path = %config.memory.path.display(),
        auto_save = config.memory.auto_save,
        "memory layer ready"
    );

    let security_setup = build_security_setup(config, hosts.approval_override).await;
    let tools_setup = build_tools_setup(
        config,
        security_setup.mcp_defer_gate.clone(),
        security_setup.security.as_ref(),
        &security_setup.fw_policy,
    )
    .await?;

    let (default_model_config, mut model_overrides) = build_model_routing_parts(config);
    crate::agents::merge_profile_routing(config, &mut model_overrides);
    let model_overrides_snapshot = model_overrides.clone();
    let failover_policy = build_failover_policy(config);
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
    let mut skill_agent_allowlists = config.skills.agent_allowlists.clone();
    crate::agents::merge_profile_skill_allowlists(&config.agents, &mut skill_agent_allowlists);
    let mut config_for_prompt = config.clone();
    config_for_prompt.skills.agent_allowlists = skill_agent_allowlists;

    crate::prompt_runtime::augment_prompt_runtime(
        &config_for_prompt,
        tools_setup.skill_catalog.as_ref(),
        project_enabled.as_ref(),
        &mut prompt_runtime,
    );

    tools_setup
        .tool_services
        .set_skills_governance(anycode_tools::SkillsGovernance {
            global_allowlist: config.skills.allowlist.clone(),
            agent_allowlists: config_for_prompt.skills.agent_allowlists.clone(),
            project_enabled: project_enabled.clone(),
        });
    if let Ok((_, cfg_value)) = anycode_llm::read_config_value(None) {
        let media_reg = anycode_llm::media::MediaClientRegistry::from_config(&cfg_value);
        tools_setup
            .tool_services
            .set_media_registry(Arc::new(media_reg));
    }

    let memory_pipeline_settings = if config.memory.backend == "pipeline" {
        Some(config.memory.pipeline.clone())
    } else {
        None
    };
    let session_notifications = if config.notifications.is_configured() {
        Some(config.notifications.clone())
    } else {
        None
    };

    let default_model_for_profiles = default_model_config.clone();
    let runtime = Arc::new(AgentRuntime::new(
        RuntimeCoreDeps {
            llm_client,
            tools: tools_setup.tools,
            memory_store,
            default_model_config,
            model_overrides,
            failover_policy,
            disk_output: Some(DiskTaskOutput::new_default()?),
            security: security_setup.security.clone(),
            sandbox_mode: config.security.sandbox_mode,
            prompt_config: prompt_runtime,
        },
        RuntimeMemoryOptions {
            memory_pipeline,
            memory_pipeline_settings,
            memory_project_autosave_enabled,
            session_notifications,
        },
        RuntimeToolPolicy {
            tool_name_deny,
            claude_gating: AgentClaudeToolGating {
                rules: Some(tools_setup.claude_rules),
                defer_mcp_tools: config.security.defer_mcp_tools,
                mcp_defer_allowlist: security_setup.mcp_defer_gate,
            },
            expose_skill_on_explore_plan: tools_setup.expose_skill_on_explore_plan,
        },
    ));

    tools_setup
        .tool_services
        .attach_sub_agent_executor(runtime.clone());
    runtime.attach_tool_services(tools_setup.tool_services.clone());

    let ask_host: Option<Arc<dyn AskUserQuestionHost>> =
        hosts.ask_user_question_host.or_else(|| {
            let dashboard_session = std::env::var(anycode_dashboard_ipc::approval_ipc::SESSION_ENV)
                .ok()
                .filter(|s| !s.is_empty());
            if dashboard_session.is_some()
                && anycode_dashboard_ipc::question_ipc::web_questions_enabled()
            {
                Some(
                    Arc::new(crate::workbench::workbench_ask::WorkbenchAskUserQuestionHost::new())
                        as Arc<dyn AskUserQuestionHost>,
                )
            } else if hosts.dialoguer_on_tty && stdin().is_terminal() && stdout().is_terminal() {
                #[cfg(feature = "dialoguer-host")]
                {
                    Some(
                        Arc::new(crate::hosts::dialoguer::DialoguerAskUserQuestionHost)
                            as Arc<dyn AskUserQuestionHost>,
                    )
                }
                #[cfg(not(feature = "dialoguer-host"))]
                {
                    None
                }
            } else {
                None
            }
        });
    if let Some(h) = ask_host {
        tools_setup.tool_services.attach_ask_user_question_host(h);
    }
    tools_setup
        .tool_services
        .attach_wechat_outbound_host(Arc::new(
            crate::workbench::wechat_outbound_host::CliWeChatOutboundHost,
        ));

    build_agents_setup(
        &runtime,
        config,
        &default_model_for_profiles,
        &model_overrides_snapshot,
        tools_setup.expose_skill_on_explore_plan,
    )
    .await;

    Ok(runtime)
}

/// Back-compat wrapper matching the legacy CLI signature.
pub async fn initialize_runtime_legacy(
    config: &Config,
    approval_override: Option<Box<dyn ApprovalCallback>>,
    ask_user_question_host_override: Option<Arc<dyn AskUserQuestionHost>>,
    memory_attach: MemoryAttachMode,
    project_enabled: Option<HashSet<String>>,
) -> anyhow::Result<Arc<AgentRuntime>> {
    initialize_runtime(
        config,
        RuntimeHosts {
            approval_override,
            ask_user_question_host: ask_user_question_host_override,
            dialoguer_on_tty: true,
        },
        memory_attach,
        project_enabled,
    )
    .await
}
