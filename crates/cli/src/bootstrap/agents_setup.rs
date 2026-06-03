//! Declarative and shipped agent profile registration.

use crate::app_config::Config;
use anycode_agent::{AgentRuntime, ProfileAgent};
use anycode_core::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

pub(crate) async fn build_agents_setup(
    runtime: &Arc<AgentRuntime>,
    config: &Config,
    default_model: &ModelConfig,
    model_overrides_snapshot: &HashMap<AgentType, ModelConfig>,
    expose_skill_on_explore_plan: bool,
) {
    super::agents::register_declarative_agents(
        runtime,
        config,
        default_model,
        model_overrides_snapshot,
    )
    .await;
    let shipped = super::agents::shipped_role_profiles();
    for (id, spec) in shipped.profiles {
        if config.agents.profiles.contains_key(&id) {
            continue;
        }
        if let Some(resolved) =
            super::agents::resolve_profile(&id, &spec, expose_skill_on_explore_plan)
        {
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
