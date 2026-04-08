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

    /// Documented defaults matching built-in `ModelRouter` fallbacks (`plan`/`code`/`fast`/`channel`/`best`).
    /// Use in docs, setup templates, or tests; merge into user config only when fields are empty.
    pub fn documented_mode_alias_defaults() -> Self {
        let mut mode_aliases = HashMap::new();
        mode_aliases.insert("general".to_string(), "code".to_string());
        mode_aliases.insert("plan".to_string(), "plan".to_string());
        mode_aliases.insert("code".to_string(), "code".to_string());
        mode_aliases.insert("explore".to_string(), "fast".to_string());
        mode_aliases.insert("channel".to_string(), "channel".to_string());
        mode_aliases.insert("goal".to_string(), "best".to_string());
        Self {
            default_alias: None,
            mode_aliases,
            agent_aliases: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn documented_defaults_cover_all_runtime_modes() {
        let d = ModelRouteProfile::documented_mode_alias_defaults();
        for m in [
            RuntimeMode::General,
            RuntimeMode::Explore,
            RuntimeMode::Plan,
            RuntimeMode::Code,
            RuntimeMode::Channel,
            RuntimeMode::Goal,
        ] {
            assert!(
                d.alias_for_mode(&m).is_some(),
                "missing mode alias for {}",
                m.as_str()
            );
        }
    }
}
