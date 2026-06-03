//! Agent implementation materialized from a declarative profile.

use crate::agent_profiles::ResolvedAgentProfile;
use crate::agents::agent_execute_delegated_to_runtime;
use anycode_core::prelude::*;
use async_trait::async_trait;

pub struct ProfileAgent {
    profile: ResolvedAgentProfile,
    agent_type: AgentType,
    model_config: ModelConfig,
}

impl ProfileAgent {
    pub fn new(profile: ResolvedAgentProfile, model_config: ModelConfig) -> Self {
        let agent_type = AgentType::new(&profile.id);
        Self {
            profile,
            agent_type,
            model_config,
        }
    }

    pub fn profile(&self) -> &ResolvedAgentProfile {
        &self.profile
    }
}

#[async_trait]
impl Agent for ProfileAgent {
    fn agent_type(&self) -> &AgentType {
        &self.agent_type
    }

    fn description(&self) -> &str {
        &self.profile.description
    }

    fn tools(&self) -> Vec<ToolName> {
        self.profile.tools.clone()
    }

    fn system_prompt_overlay(&self) -> Option<&str> {
        self.profile.prompt_overlay.as_deref()
    }

    fn runtime_mode(&self) -> RuntimeMode {
        self.profile.runtime_mode
    }

    async fn execute(&mut self, _task: Task) -> Result<TaskResult, CoreError> {
        let _ = &self.model_config;
        agent_execute_delegated_to_runtime()
    }
}
