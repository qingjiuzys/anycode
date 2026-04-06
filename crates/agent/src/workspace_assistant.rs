//! Workspace-first assistant profile for channel mode.

use anycode_core::prelude::*;
use async_trait::async_trait;
use anycode_tools::workspace_assistant_tool_names;

pub struct WorkspaceAssistantAgent {
    model_config: ModelConfig,
    include_skill: bool,
}

impl WorkspaceAssistantAgent {
    pub fn new(model_config: ModelConfig, include_skill: bool) -> Self {
        Self {
            model_config,
            include_skill,
        }
    }
}

#[async_trait]
impl Agent for WorkspaceAssistantAgent {
    fn agent_type(&self) -> &AgentType {
        static AGENT_TYPE: std::sync::OnceLock<AgentType> = std::sync::OnceLock::new();
        AGENT_TYPE.get_or_init(|| AgentType::new("workspace-assistant"))
    }

    fn description(&self) -> &str {
        "Workspace assistant for channel mode. Prefer reading, searching, status updates, workflow control, and guided coding handoff over direct file mutation."
    }

    fn tools(&self) -> Vec<ToolName> {
        workspace_assistant_tool_names(self.include_skill)
    }

    async fn execute(&mut self, _task: Task) -> Result<TaskResult, CoreError> {
        let _ = &self.model_config;
        Ok(TaskResult::Success {
            output: "Workspace assistant completed".to_string(),
            artifacts: vec![],
        })
    }
}

pub struct GoalAgent {
    model_config: ModelConfig,
}

impl GoalAgent {
    pub fn new(model_config: ModelConfig) -> Self {
        Self { model_config }
    }
}

#[async_trait]
impl Agent for GoalAgent {
    fn agent_type(&self) -> &AgentType {
        static AGENT_TYPE: std::sync::OnceLock<AgentType> = std::sync::OnceLock::new();
        AGENT_TYPE.get_or_init(|| AgentType::new("goal"))
    }

    fn description(&self) -> &str {
        "Goal execution agent for long-running loops with retries and progress tracking."
    }

    fn tools(&self) -> Vec<ToolName> {
        anycode_tools::general_purpose_tool_names()
    }

    async fn execute(&mut self, _task: Task) -> Result<TaskResult, CoreError> {
        let _ = &self.model_config;
        Ok(TaskResult::Success {
            output: "Goal execution completed".to_string(),
            artifacts: vec![],
        })
    }
}
