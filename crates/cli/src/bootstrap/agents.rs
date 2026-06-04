//! Register declarative agent profiles and merge routing / skill allowlists.

use crate::app_config::{
    AgentProfileFile, AgentProfileSkillsFile, AgentProfileToolsFile, AgentsConfig,
};
use crate::bootstrap::model_resolve::resolve_model_profile;
use anycode_agent::{
    is_builtin_extends, profile_spec_for_builtin, resolve_profile as resolve_agent_profile,
    AgentProfileSpec, AgentRuntime, ProfileAgent, ResolvedAgentProfile,
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

fn agent_profile_file_from_spec(spec: &AgentProfileSpec) -> AgentProfileFile {
    AgentProfileFile {
        extends: spec.extends.clone(),
        description: spec.description.clone(),
        tools: if spec.tools_allow.is_some() || spec.tools_deny.is_some() {
            Some(AgentProfileToolsFile {
                allow: spec.tools_allow.clone(),
                deny: spec.tools_deny.clone(),
            })
        } else {
            None
        },
        skills: spec
            .skills_allowlist
            .as_ref()
            .map(|allowlist| AgentProfileSkillsFile {
                allowlist: Some(allowlist.clone()),
            }),
        routing: None,
        prompt_overlay: spec.prompt_overlay.clone(),
    }
}

/// Shipped role presets (extends builtins) for quick start.
pub(crate) fn shipped_role_profiles() -> AgentsConfig {
    use anycode_agent::SHIPPED_ROLE_IDS;
    let mut profiles = std::collections::HashMap::new();
    for id in SHIPPED_ROLE_IDS {
        if let Some(spec) = profile_spec_for_builtin(id) {
            profiles.insert(id.to_string(), agent_profile_file_from_spec(&spec));
        }
    }
    AgentsConfig {
        profiles,
        defaults: Default::default(),
    }
}

pub(crate) async fn build_agents_setup(
    runtime: &Arc<AgentRuntime>,
    config: &crate::app_config::Config,
    default_model: &ModelConfig,
    model_overrides_snapshot: &HashMap<AgentType, ModelConfig>,
    expose_skill_on_explore_plan: bool,
) {
    register_declarative_agents(runtime, config, default_model, model_overrides_snapshot).await;
    let shipped = shipped_role_profiles();
    for (id, spec) in shipped.profiles {
        if config.agents.profiles.contains_key(&id) {
            continue;
        }
        if let Some(resolved) = resolve_profile(&id, &spec, expose_skill_on_explore_plan) {
            let model = model_overrides_snapshot
                .get(&AgentType::new(&id))
                .cloned()
                .unwrap_or_else(|| default_model.clone());
            runtime
                .register_agent(Box::new(ProfileAgent::new(resolved, model)))
                .await;
        }
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
