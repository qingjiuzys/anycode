//! Register declarative agent profiles and merge routing / skill allowlists.

use crate::app_config::{AgentProfileFile, AgentProfileToolsFile, AgentsConfig};
use crate::bootstrap::model_resolve::resolve_model_profile;
use anycode_agent::{
    is_builtin_extends, resolve_profile as resolve_agent_profile, AgentProfileSpec, AgentRuntime,
    ProfileAgent, ResolvedAgentProfile,
};
use anycode_core::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

pub(crate) fn merge_profile_routing(
    config: &crate::app_config::Config,
    model_overrides: &mut HashMap<AgentType, ModelConfig>,
) {
    for (id, profile) in &config.agents.profiles {
        let Some(routing) = profile.routing.as_ref() else {
            continue;
        };
        if model_overrides.contains_key(&AgentType::new(id)) {
            continue;
        }
        model_overrides.insert(
            AgentType::new(id.clone()),
            resolve_model_profile(config, routing),
        );
    }
}

fn profile_spec_from_file(spec: &AgentProfileFile) -> AgentProfileSpec {
    AgentProfileSpec {
        extends: spec.extends.clone(),
        description: spec.description.clone(),
        tools_allow: spec.tools.as_ref().and_then(|t| t.allow.clone()),
        tools_deny: spec.tools.as_ref().and_then(|t| t.deny.clone()),
        skills_allowlist: spec.skills.as_ref().and_then(|s| s.allowlist.clone()),
        prompt_overlay: spec.prompt_overlay.clone(),
    }
}

pub(crate) fn resolve_profile_from_file(
    id: &str,
    spec: &AgentProfileFile,
    include_skill_on_explore_plan: bool,
) -> ResolvedAgentProfile {
    resolve_agent_profile(
        id,
        &profile_spec_from_file(spec),
        include_skill_on_explore_plan,
    )
}

pub(crate) fn resolve_profile(
    id: &str,
    spec: &AgentProfileFile,
    include_skill_on_explore_plan: bool,
) -> Option<ResolvedAgentProfile> {
    Some(resolve_profile_from_file(
        id,
        spec,
        include_skill_on_explore_plan,
    ))
}

pub(crate) async fn register_declarative_agents(
    runtime: &Arc<AgentRuntime>,
    config: &crate::app_config::Config,
    default_model: &ModelConfig,
    model_overrides: &HashMap<AgentType, ModelConfig>,
) {
    let include_skill = config.skills.enabled && config.skills.expose_on_explore_plan;
    for (id, spec) in &config.agents.profiles {
        if is_builtin_extends(id) {
            tracing::warn!(
                target: "anycode_cli",
                "skipping agent profile `{id}`: id conflicts with builtin agent"
            );
            continue;
        }
        let Some(resolved) = resolve_profile(id, spec, include_skill) else {
            continue;
        };
        let model = model_overrides
            .get(&AgentType::new(id))
            .cloned()
            .unwrap_or_else(|| default_model.clone());
        let agent = Box::new(ProfileAgent::new(resolved, model)) as Box<dyn Agent>;
        runtime.register_agent(agent).await;
    }
}

pub(crate) fn merge_profile_skill_allowlists(
    agents: &AgentsConfig,
    agent_allowlists: &mut HashMap<String, Vec<String>>,
) {
    for (id, spec) in &agents.profiles {
        if let Some(list) = spec
            .skills
            .as_ref()
            .and_then(|s| s.allowlist.as_ref())
            .filter(|v| !v.is_empty())
        {
            agent_allowlists.insert(id.clone(), list.clone());
        }
    }
}

/// Shipped role presets (extends builtins) for quick start.
pub(crate) fn shipped_role_profiles() -> AgentsConfig {
    let mut profiles = HashMap::new();
    profiles.insert(
        "builder".into(),
        AgentProfileFile {
            extends: "general-purpose".into(),
            description: Some("Default implementation-focused coding agent".into()),
            tools: None,
            skills: None,
            routing: None,
            prompt_overlay: None,
        },
    );
    profiles.insert(
        "planner".into(),
        AgentProfileFile {
            extends: "plan".into(),
            description: Some("Architecture and task decomposition".into()),
            tools: None,
            skills: None,
            routing: None,
            prompt_overlay: None,
        },
    );
    profiles.insert(
        "explorer".into(),
        AgentProfileFile {
            extends: "explore".into(),
            description: Some("Fast codebase exploration".into()),
            tools: None,
            skills: None,
            routing: None,
            prompt_overlay: None,
        },
    );
    profiles.insert(
        "verifier".into(),
        AgentProfileFile {
            extends: "explore".into(),
            description: Some("Read-only verification and test inspection".into()),
            tools: Some(AgentProfileToolsFile {
                allow: None,
                deny: Some(vec!["Bash".into(), "Edit".into(), "FileWrite".into()]),
            }),
            skills: None,
            routing: None,
            prompt_overlay: None,
        },
    );
    profiles.insert(
        "reviewer".into(),
        AgentProfileFile {
            extends: "explore".into(),
            description: Some("PR-style review without shell mutation".into()),
            tools: Some(AgentProfileToolsFile {
                allow: Some(vec![
                    "FileRead".into(),
                    "Grep".into(),
                    "Glob".into(),
                    "StructuredOutput".into(),
                ]),
                deny: None,
            }),
            skills: None,
            routing: None,
            prompt_overlay: None,
        },
    );
    profiles.insert(
        "channel-ops".into(),
        AgentProfileFile {
            extends: "workspace-assistant".into(),
            description: Some("IM / cron channel operations".into()),
            tools: None,
            skills: None,
            routing: None,
            prompt_overlay: None,
        },
    );
    profiles.insert(
        "goal-runner".into(),
        AgentProfileFile {
            extends: "goal".into(),
            description: Some("Autonomous goal iteration".into()),
            tools: None,
            skills: None,
            routing: None,
            prompt_overlay: None,
        },
    );
    profiles.insert(
        "office-writer".into(),
        AgentProfileFile {
            extends: "general-purpose".into(),
            description: Some("Office writing: reports, briefs, content drafts".into()),
            tools: None,
            skills: Some(crate::app_config::AgentProfileSkillsFile {
                allowlist: Some(vec![
                    "content-repurpose".into(),
                    "doc-summary".into(),
                    "md-to-pdf".into(),
                    "weekly-report".into(),
                ]),
            }),
            routing: None,
            prompt_overlay: Some(
                "You are an office writing assistant. Produce clear Markdown drafts; do not publish externally. Use KnowledgeSearch for indexed project materials when paths are configured.".into(),
            ),
        },
    );
    profiles.insert(
        "data-analyst".into(),
        AgentProfileFile {
            extends: "general-purpose".into(),
            description: Some("Spreadsheets, summaries, and data-oriented reports".into()),
            tools: None,
            skills: Some(crate::app_config::AgentProfileSkillsFile {
                allowlist: Some(vec![
                    "doc-summary".into(),
                    "report-to-csv".into(),
                    "weekly-report".into(),
                ]),
            }),
            routing: None,
            prompt_overlay: Some(
                "Focus on accurate data summaries and tables; cite source files. Use KnowledgeSearch and report-to-csv when exporting tabular results.".into(),
            ),
        },
    );
    profiles.insert(
        "researcher".into(),
        AgentProfileFile {
            extends: "explore".into(),
            description: Some("Industry research and daily briefs".into()),
            tools: None,
            skills: Some(crate::app_config::AgentProfileSkillsFile {
                allowlist: Some(vec!["daily-brief".into()]),
            }),
            routing: None,
            prompt_overlay: Some(
                "Gather sources with WebSearch/WebFetch; synthesize with citations. Bind daily-brief skill for scheduled summaries.".into(),
            ),
        },
    );
    profiles.insert(
        "file-operator".into(),
        AgentProfileFile {
            extends: "workspace-assistant".into(),
            description: Some("Batch file organization and cleanup".into()),
            tools: None,
            skills: Some(crate::app_config::AgentProfileSkillsFile {
                allowlist: Some(vec!["file-organizer".into()]),
            }),
            routing: None,
            prompt_overlay: None,
        },
    );
    AgentsConfig {
        profiles,
        defaults: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reviewer_profile_denies_bash() {
        let spec = AgentProfileFile {
            extends: "explore".into(),
            description: Some("review".into()),
            tools: Some(AgentProfileToolsFile {
                allow: Some(vec!["FileRead".into(), "Grep".into()]),
                deny: None,
            }),
            skills: None,
            routing: None,
            prompt_overlay: None,
        };
        let resolved = resolve_profile("reviewer", &spec, false).unwrap();
        assert!(!resolved.tools.contains(&"Bash".to_string()));
        assert!(resolved.tools.contains(&"FileRead".to_string()));
    }
}
