//! Runtime model router inspired by OpenClaw / Claude Code mode routing.

use crate::model_catalog::is_known_model_alias;
use anycode_core::{AgentType, ModelConfig, ModelRouteProfile, RuntimeMode, RuntimeProfile};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ModelRouter {
    default_model: ModelConfig,
    agent_overrides: HashMap<String, ModelConfig>,
    route_profile: ModelRouteProfile,
}

impl ModelRouter {
    pub fn new(
        default_model: ModelConfig,
        agent_overrides: HashMap<AgentType, ModelConfig>,
        route_profile: ModelRouteProfile,
    ) -> Self {
        Self {
            default_model,
            agent_overrides: agent_overrides
                .into_iter()
                .map(|(k, v)| (k.as_str().to_string(), v))
                .collect(),
            route_profile,
        }
    }

    pub fn default_model(&self) -> &ModelConfig {
        &self.default_model
    }

    pub fn resolve_for_agent(&self, agent_type: &AgentType) -> ModelConfig {
        self.agent_overrides
            .get(agent_type.as_str())
            .cloned()
            .unwrap_or_else(|| self.default_model.clone())
    }

    pub fn resolve_for_mode(&self, mode: &RuntimeMode) -> ModelConfig {
        match mode {
            RuntimeMode::Plan => self.resolve_named("plan", "plan"),
            RuntimeMode::Code | RuntimeMode::General => {
                self.resolve_named("general-purpose", "code")
            }
            RuntimeMode::Explore => self.resolve_named("explore", "fast"),
            RuntimeMode::Channel => self.resolve_named("workspace-assistant", "channel"),
            RuntimeMode::Goal => self.resolve_named("goal", "best"),
        }
    }

    pub fn resolve_for_profile(&self, profile: &RuntimeProfile) -> ModelConfig {
        if let Some(alias) = profile.model_alias.as_deref() {
            return self.resolve_alias_or_agent(alias, profile.agent_type.as_str());
        }
        self.resolve_for_mode(&profile.mode)
    }

    pub fn resolve_summary_model(&self) -> ModelConfig {
        self.resolve_named("summary", "summary")
    }

    pub fn route_profile(&self) -> &ModelRouteProfile {
        &self.route_profile
    }

    fn resolve_named(&self, agent_name: &str, fallback_alias: &str) -> ModelConfig {
        if let Some(alias) = self.route_profile.alias_for_agent(agent_name) {
            return self.resolve_alias_or_agent(alias, agent_name);
        }
        if let Some(cfg) = self.agent_overrides.get(agent_name) {
            return cfg.clone();
        }
        self.resolve_alias_or_agent(fallback_alias, agent_name)
    }

    fn resolve_alias_or_agent(&self, alias_or_agent: &str, fallback_agent: &str) -> ModelConfig {
        let normalized = alias_or_agent.trim().to_ascii_lowercase();
        if !is_known_model_alias(&normalized) {
            if let Some(cfg) = self.agent_overrides.get(alias_or_agent) {
                return cfg.clone();
            }
            return self
                .agent_overrides
                .get(fallback_agent)
                .cloned()
                .unwrap_or_else(|| self.default_model.clone());
        }
        match normalized.as_str() {
            "best" | "code" => self
                .agent_overrides
                .get("general-purpose")
                .cloned()
                .unwrap_or_else(|| self.default_model.clone()),
            "fast" => self
                .agent_overrides
                .get("explore")
                .cloned()
                .unwrap_or_else(|| self.default_model.clone()),
            "plan" => self
                .agent_overrides
                .get("plan")
                .cloned()
                .unwrap_or_else(|| self.default_model.clone()),
            "channel" => self
                .agent_overrides
                .get("workspace-assistant")
                .cloned()
                .or_else(|| self.agent_overrides.get("channel").cloned())
                .unwrap_or_else(|| self.default_model.clone()),
            "summary" => self
                .agent_overrides
                .get("summary")
                .cloned()
                .or_else(|| self.agent_overrides.get("plan").cloned())
                .unwrap_or_else(|| self.default_model.clone()),
            _ => self.default_model.clone(),
        }
    }
}
