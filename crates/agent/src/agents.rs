//! 内置 Agent 实现（general-purpose / explore / plan）。

use anycode_core::prelude::*;
use anycode_tools::{explore_plan_tool_names_with_skill, general_purpose_tool_names};
use async_trait::async_trait;

/// General-purpose agent（默认全工具集）
pub struct GeneralPurposeAgent {
    tools: Vec<ToolName>,
    model_config: ModelConfig,
}

impl GeneralPurposeAgent {
    pub fn new(model_config: ModelConfig) -> Self {
        Self {
            tools: general_purpose_tool_names(),
            model_config,
        }
    }
}

#[async_trait]
impl Agent for GeneralPurposeAgent {
    fn agent_type(&self) -> &AgentType {
        static AGENT_TYPE: std::sync::OnceLock<AgentType> = std::sync::OnceLock::new();
        AGENT_TYPE.get_or_init(|| AgentType::new("general-purpose"))
    }

    fn description(&self) -> &str {
        "General-purpose agent for researching complex questions, searching for code, and executing multi-step tasks. When you are searching for a keyword or file and are not confident that you will find the right match in the first few tries use this agent to perform the search for you."
    }

    fn tools(&self) -> Vec<ToolName> {
        self.tools.clone()
    }

    async fn execute(&mut self, _task: Task) -> Result<TaskResult, CoreError> {
        let _ = &self.model_config;
        Ok(TaskResult::Success {
            output: "Task completed".to_string(),
            artifacts: vec![],
        })
    }
}

/// Explore agent（读/搜为主）
pub struct ExploreAgent {
    model_config: ModelConfig,
    include_skill: bool,
}

impl ExploreAgent {
    pub fn new(model_config: ModelConfig, include_skill: bool) -> Self {
        Self {
            model_config,
            include_skill,
        }
    }
}

#[async_trait]
impl Agent for ExploreAgent {
    fn agent_type(&self) -> &AgentType {
        static AGENT_TYPE: std::sync::OnceLock<AgentType> = std::sync::OnceLock::new();
        AGENT_TYPE.get_or_init(|| AgentType::new("explore"))
    }

    fn description(&self) -> &str {
        "Fast agent specialized for exploring codebases. Use this when you need to quickly find files by patterns (eg. \"src/components/**/*.tsx\"), search code for keywords (eg. \"API endpoints\"), or answer questions about the codebase (eg. \"how do API endpoints work?\"). When calling this agent, specify the desired thoroughness level: \"quick\" for basic searches, \"medium\" for moderate exploration, or \"very thorough\" for comprehensive analysis across multiple locations and naming conventions."
    }

    fn tools(&self) -> Vec<ToolName> {
        explore_plan_tool_names_with_skill(self.include_skill)
    }

    async fn execute(&mut self, _task: Task) -> Result<TaskResult, CoreError> {
        let _ = &self.model_config;
        Ok(TaskResult::Success {
            output: "Exploration completed".to_string(),
            artifacts: vec![],
        })
    }
}

/// Plan agent（规划与拆解）
pub struct PlanAgent {
    model_config: ModelConfig,
    include_skill: bool,
}

impl PlanAgent {
    pub fn new(model_config: ModelConfig, include_skill: bool) -> Self {
        Self {
            model_config,
            include_skill,
        }
    }
}

#[async_trait]
impl Agent for PlanAgent {
    fn agent_type(&self) -> &AgentType {
        static AGENT_TYPE: std::sync::OnceLock<AgentType> = std::sync::OnceLock::new();
        AGENT_TYPE.get_or_init(|| AgentType::new("plan"))
    }

    fn description(&self) -> &str {
        "Software architect agent for designing implementation plans. Use this when you need to plan the implementation strategy for a task. Returns step-by-step plans, identifies critical files, and considers architectural trade-offs."
    }

    fn tools(&self) -> Vec<ToolName> {
        explore_plan_tool_names_with_skill(self.include_skill)
    }

    async fn execute(&mut self, _task: Task) -> Result<TaskResult, CoreError> {
        let _ = &self.model_config;
        Ok(TaskResult::Success {
            output: "Plan created".to_string(),
            artifacts: vec![],
        })
    }
}
