//! User-facing model alias preferences per mode and agent.

use crate::RuntimeMode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelRouteProfile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_alias: Option<String>,
    #[serde(default)]
    pub mode_aliases: HashMap<String, String>,
    #[serde(default)]
    pub agent_aliases: HashMap<String, String>,
}

impl ModelRouteProfile {
    pub fn alias_for_mode(&self, mode: &RuntimeMode) -> Option<&str> {
        self.mode_aliases
            .get(mode.as_str())
            .map(|s| s.as_str())
            .or(self.default_alias.as_deref())
    }

    pub fn alias_for_agent(&self, agent_type: &str) -> Option<&str> {
        self.agent_aliases
            .get(agent_type)
            .map(|s| s.as_str())
            .or(self.default_alias.as_deref())
    }
}
