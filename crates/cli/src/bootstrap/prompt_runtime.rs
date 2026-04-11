//! Fills [`RuntimePromptConfig`] for `initialize_runtime` (workspace/channel/workflow/skills sections).

use crate::app_config::Config;
use crate::i18n::tr_args;
use anycode_agent::RuntimePromptConfig;
use anycode_llm::known_model_aliases;
use anycode_tools::SkillCatalog;
use fluent_bundle::FluentArgs;

/// Mutates `prompt_runtime` cloned from `config.prompt` with skills, workspace labels, and routing hints.
pub(crate) fn augment_prompt_runtime(
    config: &Config,
    skill_catalog: &SkillCatalog,
    prompt_runtime: &mut RuntimePromptConfig,
) {
    if prompt_runtime.model_instructions_file.is_some() {
        let working_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        if let Err(e) = prompt_runtime.resolve_model_instructions_file(&working_dir) {
            let mut a = FluentArgs::new();
            a.set(
                "path",
                prompt_runtime
                    .model_instructions_file
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
            );
            a.set("err", e.to_string());
            tracing::warn!(
                target: "anycode_cli",
                "{}",
                tr_args("log-model-instructions-fail", &a)
            );
        }
    }

    if config.skills.enabled {
        if let Some(section) = skill_catalog.render_prompt_subsection() {
            prompt_runtime.skills_section = Some(section);
        }
        for (agent, ids) in &config.skills.agent_allowlists {
            if ids.is_empty() {
                continue;
            }
            if let Some(section) =
                skill_catalog.render_prompt_subsection_allowlist(Some(ids.as_slice()))
            {
                prompt_runtime
                    .skills_section_by_agent
                    .insert(agent.clone(), section);
            }
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
}
