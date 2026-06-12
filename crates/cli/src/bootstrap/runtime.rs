//! Assembles LLM stack, tools, security, and [`AgentRuntime`] (`initialize_runtime`).

use crate::app_config::Config;
use crate::i18n::tr_args;
use anycode_agent::{
    AgentClaudeToolGating, AgentRuntime, RuntimeCoreDeps, RuntimeMemoryOptions, RuntimeToolPolicy,
};
use anycode_core::prelude::*;
use anycode_core::DiskTaskOutput;
use anycode_llm::ModelRouter;
use anycode_security::ApprovalCallback;
use anycode_tools::AskUserQuestionHost;
use fluent_bundle::FluentArgs;
use std::collections::HashSet;
use std::io::{stdin, stdout, IsTerminal};
use std::sync::Arc;
use tracing::info;

use super::agents::build_agents_setup;
use super::llm_stack::build_llm_stack;
use super::security_setup::build_security_setup;
use super::tools_setup::build_tools_setup;
use super::{
    build_failover_policy, build_memory_layer, build_model_routing_parts,
    compile_tool_name_deny_regexes, effective_memory_backend, MemoryAttachMode,
};

/// Shared by the WeChat bridge, TUI, `run`, and other CLI entrypoints that need a full runtime.
///
/// `ask_user_question_host_override`: when `Some`, used for `AskUserQuestion` (stream REPL / fullscreen TUI).
/// When `None` and stdin+stdout are TTY, falls back to dialoguer on stderr.
///
/// `project_enabled`: project-scoped skill allowlist from the dashboard DB; callers with a known cwd
/// should resolve this via [`crate::workbench::project_skills::load_project_enabled_skills`].
pub(crate) async fn initialize_runtime(
    config: &Config,
    approval_override: Option<Box<dyn ApprovalCallback>>,
    ask_user_question_host_override: Option<std::sync::Arc<dyn AskUserQuestionHost>>,
    memory_attach: MemoryAttachMode,
    project_enabled: Option<HashSet<String>>,
) -> anyhow::Result<Arc<AgentRuntime>> {
    // Reply-language fallback: the dashboard sets ANYCODE_REPLY_LANG explicitly
    // (UI language); plain CLI usage follows the resolved locale.
    if std::env::var_os("ANYCODE_REPLY_LANG").is_none() {
        std::env::set_var(
            "ANYCODE_REPLY_LANG",
            anycode_locale::resolve_locale().as_str(),
        );
    }
    let llm_client = build_llm_stack(config).await?;

    let (memory_store, memory_pipeline) = build_memory_layer(config, memory_attach)?;
    let mut mi = FluentArgs::new();
    mi.set("backend", config.memory.backend.clone());
    mi.set("attach", memory_attach.as_str().to_string());
    mi.set(
        "effective",
        effective_memory_backend(config, memory_attach).to_string(),
    );
    mi.set("path", config.memory.path.display().to_string());
    mi.set("auto", format!("{}", config.memory.auto_save));
    info!(target: "anycode_cli", "{}", tr_args("log-memory-info", &mi));

    let security_setup = build_security_setup(config, approval_override).await;
    let tools_setup = build_tools_setup(
        config,
        security_setup.mcp_defer_gate.clone(),
        security_setup.security.as_ref(),
        &security_setup.fw_policy,
    )
    .await?;

    let (default_model_config, mut model_overrides) = build_model_routing_parts(config);
    super::agents::merge_profile_routing(config, &mut model_overrides);
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
    super::agents::merge_profile_skill_allowlists(&config.agents, &mut skill_agent_allowlists);
    let mut config_for_prompt = config.clone();
    config_for_prompt.skills.agent_allowlists = skill_agent_allowlists;

    super::prompt_runtime::augment_prompt_runtime(
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

    let ask_host: Option<std::sync::Arc<dyn AskUserQuestionHost>> = ask_user_question_host_override
        .or_else(|| {
            let dashboard_session = std::env::var(anycode_dashboard::approval_ipc::SESSION_ENV)
                .ok()
                .filter(|s| !s.is_empty());
            if dashboard_session.is_some()
                && anycode_dashboard::question_ipc::web_questions_enabled()
            {
                Some(std::sync::Arc::new(
                    crate::workbench::workbench_ask::WorkbenchAskUserQuestionHost::new(),
                ) as std::sync::Arc<dyn AskUserQuestionHost>)
            } else if stdin().is_terminal() && stdout().is_terminal() {
                Some(
                    std::sync::Arc::new(crate::ask_user_host::DialoguerAskUserQuestionHost)
                        as std::sync::Arc<dyn AskUserQuestionHost>,
                )
            } else {
                None
            }
        });
    if let Some(h) = ask_host {
        tools_setup.tool_services.attach_ask_user_question_host(h);
    }
    tools_setup
        .tool_services
        .attach_wechat_outbound_host(std::sync::Arc::new(
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
