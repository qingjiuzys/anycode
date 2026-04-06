//! Runtime mode/profile unifies routing, prompt, and approval defaults.

use crate::{AgentType, FeatureRegistry, PermissionMode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuntimeMode {
    General,
    Explore,
    Plan,
    Code,
    Channel,
    Goal,
}

impl RuntimeMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Explore => "explore",
            Self::Plan => "plan",
            Self::Code => "code",
            Self::Channel => "channel",
            Self::Goal => "goal",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "general" | "default" => Some(Self::General),
            "explore" => Some(Self::Explore),
            "plan" => Some(Self::Plan),
            "code" | "coding" => Some(Self::Code),
            "channel" => Some(Self::Channel),
            "goal" => Some(Self::Goal),
            _ => None,
        }
    }

    pub fn default_agent(&self) -> AgentType {
        match self {
            Self::General | Self::Code => AgentType::new("general-purpose"),
            Self::Explore => AgentType::new("explore"),
            Self::Plan => AgentType::new("plan"),
            Self::Channel => AgentType::new("workspace-assistant"),
            Self::Goal => AgentType::new("goal"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeProfile {
    pub mode: RuntimeMode,
    pub agent_type: AgentType,
    pub permission_mode: PermissionMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_alias: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal: Option<String>,
    #[serde(default)]
    pub features: FeatureRegistry,
}

impl RuntimeProfile {
    pub fn for_mode(mode: RuntimeMode, features: FeatureRegistry) -> Self {
        let permission_mode = match mode {
            RuntimeMode::Plan => PermissionMode::Plan,
            RuntimeMode::Channel => PermissionMode::Auto,
            _ => PermissionMode::Default,
        };
        Self {
            agent_type: mode.default_agent(),
            model_alias: Some(mode.as_str().to_string()),
            channel_profile: None,
            goal: None,
            mode,
            permission_mode,
            features,
        }
    }

    pub fn with_agent(mut self, agent_type: AgentType) -> Self {
        self.agent_type = agent_type;
        self
    }
}
