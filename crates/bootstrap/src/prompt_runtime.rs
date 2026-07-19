//! Fills [`RuntimePromptConfig`] for `initialize_runtime` (workspace/channel/workflow/skills sections).

use anycode_config::Config;

use anycode_agent::RuntimePromptConfig;
use anycode_llm::known_model_aliases;
use anycode_tools::{SkillCatalog, SkillsGovernance};

use std::collections::HashSet;

fn skills_governance_for_config(
    config: &Config,
    project_enabled: Option<&HashSet<String>>,
) -> SkillsGovernance {
    SkillsGovernance {
        global_allowlist: config.skills.allowlist.clone(),
        agent_allowlists: config.skills.agent_allowlists.clone(),
        project_enabled: project_enabled.cloned(),
    }
}

fn allowlist_vec(set: &HashSet<String>) -> Vec<String> {
    let mut v: Vec<String> = set.iter().cloned().collect();
    v.sort();
    v
}

/// Mutates `prompt_runtime` cloned from `config.prompt` with skills, workspace labels, and routing hints.
pub fn augment_prompt_runtime(
    config: &Config,
    skill_catalog: &SkillCatalog,
    project_enabled: Option<&HashSet<String>>,
    prompt_runtime: &mut RuntimePromptConfig,
) {
    if prompt_runtime.model_instructions_file.is_some() {
        let working_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        if let Err(e) = prompt_runtime.resolve_model_instructions_file(&working_dir) {
            tracing::warn!(
                target: "anycode_bootstrap",
                path = ?prompt_runtime.model_instructions_file,
                error = %e,
                "failed to resolve model instructions file"
            );
        }
    }

    if config.skills.enabled {
        let gov = skills_governance_for_config(config, project_enabled);
        let default_agent = "general-purpose";
        if let Some(eff) = gov.effective_ids(default_agent) {
            let ids = allowlist_vec(&eff);
            if let Some(section) = skill_catalog.render_prompt_subsection_allowlist(Some(&ids)) {
                prompt_runtime.skills_section = Some(section);
            }
        } else {
            prompt_runtime.skills_section = Some(SkillCatalog::render_prompt_skills_contract());
        }
        let mut agents: Vec<String> = config.skills.agent_allowlists.keys().cloned().collect();
        agents.sort();
        agents.dedup();
        for agent in agents {
            if let Some(eff) = gov.effective_ids(&agent) {
                let ids = allowlist_vec(&eff);
                if ids.is_empty() {
                    continue;
                }
                if let Some(section) = skill_catalog.render_prompt_subsection_allowlist(Some(&ids))
                {
                    prompt_runtime
                        .skills_section_by_agent
                        .insert(agent, section);
                }
            } else if let Some(ids) = config.skills.agent_allowlists.get(&agent) {
                if ids.is_empty() {
                    continue;
                }
                if let Some(section) =
                    skill_catalog.render_prompt_subsection_allowlist(Some(ids.as_slice()))
                {
                    prompt_runtime
                        .skills_section_by_agent
                        .insert(agent, section);
                }
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
        anycode_config::canonical_root_string(),
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
        "## Goal Mode\nFor goal-oriented tasks, keep iterating until completion criteria are met (retries are unlimited by default; use `--max-goal-attempts` only when a cap is needed). Retry after tool/LLM failures unless the user cancels. Stop and surface hard blockers such as missing approvals, missing credentials, or impossible environment requirements.\nWhen `done_when` is set for a `test/...` Flutter directory, completion requires the marker in assistant output, on that directory's README.md, and passing `flutter analyze` + `flutter test` in that directory (the engine re-runs these checks). Use `GoalSpec.max_attempts_cap` to bound attempts when required."
            .to_string(),
    );
    prompt_runtime.prompt_fragments.push(format!(
        "## Model Routing\nKnown aliases: {}\nMode aliases default to: general=code, explore=fast, plan=plan, channel=channel, goal=best.",
        known_model_aliases().join(", ")
    ));
}

/// Build a fully augmented [`RuntimePromptConfig`] for preview/runtime parity.
pub fn build_runtime_prompt_config(
    config: &Config,
    skill_catalog: &SkillCatalog,
    project_enabled: Option<&HashSet<String>>,
) -> RuntimePromptConfig {
    let mut config_for_prompt = config.clone();
    let mut skill_agent_allowlists = config.skills.agent_allowlists.clone();
    crate::agents::merge_profile_skill_allowlists(&config.agents, &mut skill_agent_allowlists);
    config_for_prompt.skills.agent_allowlists = skill_agent_allowlists;
    let mut prompt_runtime = config.prompt.clone();
    augment_prompt_runtime(
        &config_for_prompt,
        skill_catalog,
        project_enabled,
        &mut prompt_runtime,
    );
    prompt_runtime
}
