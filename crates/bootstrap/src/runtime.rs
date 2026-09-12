//! Assembles LLM stack, tools, security, and [`AgentRuntime`] (`initialize_runtime`).

use crate::agents::build_agents_setup;
use crate::llm_stack::build_llm_stack;
use crate::security_setup::build_security_setup;
use crate::tools_setup::build_tools_setup;
use crate::{
    build_failover_chain, build_memory_layer, build_model_routing_parts,
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
use std::path::Path;
use std::sync::Arc;
use tracing::info;

/// Injectable hosts for approval, ask-user, and Skill Apps.
#[derive(Default)]
pub struct RuntimeHosts {
    pub approval_override: Option<Box<dyn ApprovalCallback>>,
    pub ask_user_question_host: Option<Arc<dyn AskUserQuestionHost>>,
    pub skill_app_host: Option<Arc<dyn anycode_tools::SkillAppHost>>,
}

/// Shared composition root for dashboard, daemon, and legacy CLI paths.
pub async fn initialize_runtime(
    config: &Config,
    hosts: RuntimeHosts,
    memory_attach: MemoryAttachMode,
    project_enabled: Option<HashSet<String>>,
    skill_project_root: Option<&Path>,
) -> anyhow::Result<Arc<AgentRuntime>> {
    if std::env::var_os("ANYCODE_REPLY_LANG").is_none() {
        std::env::set_var(
            "ANYCODE_REPLY_LANG",
            anycode_locale::resolve_locale().as_str(),
        );
    }
    // 文件式 agent 定义（`.anycode/agents/*.md`）合并：必须在 `merge_profile_routing`
    // 之前，使文件 agent 的 routing / skills allowlist / 注册走既有 profiles 管线。
    // 优先级：用户目录 < 项目目录 < config.json < builtin（见 merge_file_agents_into_config）。
    let mut config_owned = config.clone();
    crate::agents::merge_file_agents_into_config(&mut config_owned.agents, skill_project_root);
    let config = &config_owned;
    // Managed python/node under ~/.anycode/runtimes win over system PATH for
    // every spawned tool/skill; provisioning (if missing) runs in background.
    crate::runtimes::prepend_runtime_paths();
    crate::runtimes::spawn_runtime_provision();
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
        skill_project_root,
    )
    .await?;

    let (default_model_config, mut model_overrides) = build_model_routing_parts(config)?;
    crate::agents::merge_profile_routing(config, &mut model_overrides);
    let model_overrides_snapshot = model_overrides.clone();
    let failover_chain = build_failover_chain(config);
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
    // auto-memory 引擎裁决：enabled + fork_agent + LLM 可用（api_key 非空）才接 LLM 驱动；
    // 否则保持本地规则管线（`resolve_automem_engine` 语义）。
    // `ANYCODE_DISABLE_AUTOMEM=1` 一键关闭（对齐 Claude `CLAUDE_CODE_DISABLE_AUTO_MEMORY`）。
    let automem_env_disabled = std::env::var("ANYCODE_DISABLE_AUTOMEM")
        .map(|v| matches!(v.trim(), "1" | "true" | "TRUE" | "yes" | "on"))
        .unwrap_or(false);
    let llm_available = !config.llm.api_key.trim().is_empty() && !automem_env_disabled;
    let automem = match anycode_memory::automem::resolve_automem_engine(
        &config.memory.automem,
        llm_available,
    ) {
        anycode_memory::automem::AutomemEngine::Llm if !automem_env_disabled => {
            Some(config.memory.automem.clone())
        }
        _ => None,
    };
    let session_notifications = if config.notifications.is_configured() {
        Some(config.notifications.clone())
    } else {
        None
    };

    let default_model_for_profiles = default_model_config.clone();
    let runtime = Arc::new(
        AgentRuntime::new(
            RuntimeCoreDeps {
                llm_client,
                tools: tools_setup.tools,
                memory_store,
                default_model_config,
                model_overrides,
                failover_chain,
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
                automem,
                automem_base_path: config.memory.automem.base_path.clone(),
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
        )
        .with_auto_compact(
            config.session.auto_compact,
            anycode_agent::CompactPolicy {
                trigger_ratio: config.session.auto_compact_ratio.clamp(0.01, 1.0),
                hard_token_threshold: config.session.auto_compact_min_input_tokens,
                suppress_follow_up_questions: true,
                ..Default::default()
            }
            .with_env(),
        )
        .with_session_context(
            config.session.context_window_auto,
            config.session.context_window_tokens,
        )
        .with_harness_unified_kernel(anycode_core::harness_unified_kernel_enabled(
            &config.runtime.features,
        )),
    );

    tools_setup
        .tool_services
        .attach_sub_agent_executor(runtime.clone());
    runtime.attach_tool_services(tools_setup.tool_services.clone());
    runtime.attach_self();
    runtime.attach_harness_tools().await;

    if anycode_core::harness_readonly_pilot_enabled(&config.runtime.features) {
        info!(
            target: "anycode_bootstrap",
            "harness-v1 read-only FileRead pilot is armed; execute_task and scheduler stay on the legacy loop unless unified kernel is also armed"
        );
    }
    if anycode_core::harness_unified_kernel_enabled(&config.runtime.features) {
        info!(
            target: "anycode_bootstrap",
            "harness-v1 unified Kernel adapters are armed; execute_task and execute_turn use Kernel.run and do not nest the legacy loop"
        );
    }

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
            } else {
                None
            }
        });
    if let Some(h) = ask_host {
        tools_setup.tool_services.attach_ask_user_question_host(h);
    }

    let skill_host: Option<Arc<dyn anycode_tools::SkillAppHost>> =
        hosts.skill_app_host.or_else(|| {
            let dashboard_session = std::env::var(anycode_dashboard_ipc::approval_ipc::SESSION_ENV)
                .ok()
                .filter(|s| !s.is_empty());
            if dashboard_session.is_some()
                && anycode_dashboard_ipc::skill_app_ipc::web_skill_apps_enabled()
            {
                Some(
                    Arc::new(crate::workbench::workbench_skill_app::WorkbenchSkillAppHost::new())
                        as Arc<dyn anycode_tools::SkillAppHost>,
                )
            } else {
                None
            }
        });
    if let Some(h) = skill_host {
        tools_setup.tool_services.attach_skill_app_host(h);
    }

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
            ..Default::default()
        },
        memory_attach,
        project_enabled,
        None,
    )
    .await
}
