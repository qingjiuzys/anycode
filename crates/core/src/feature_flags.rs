//! Unified runtime feature toggles.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FeatureFlag {
    Skills,
    Workflows,
    GoalMode,
    ChannelMode,
    ApprovalV2,
    ContextCompression,
    WorkspaceProfiles,
    /// Opt-in Kernel FileRead pilot. Default chat and scheduler stay on the legacy loop.
    HarnessV1ReadonlyPilot,
    /// Opt-in: execute_task / execute_turn become thin Kernel adapters. Default off.
    /// Never nest this loop inside the legacy loop on the same task.
    HarnessV1UnifiedKernel,
    /// Opt-in product GraphRunner routes. Default off. Not aliased to harness-v1.
    HarnessV1Graph,
}

impl FeatureFlag {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Skills => "skills",
            Self::Workflows => "workflows",
            Self::GoalMode => "goal-mode",
            Self::ChannelMode => "channel-mode",
            Self::ApprovalV2 => "approval-v2",
            Self::ContextCompression => "context-compression",
            Self::WorkspaceProfiles => "workspace-profiles",
            Self::HarnessV1ReadonlyPilot => "harness-v1-readonly-pilot",
            Self::HarnessV1UnifiedKernel => "harness-v1-unified-kernel",
            Self::HarnessV1Graph => "harness-v1-graph",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "skills" => Some(Self::Skills),
            "workflows" | "workflow" => Some(Self::Workflows),
            "goal-mode" | "goal" => Some(Self::GoalMode),
            "channel-mode" | "channel" => Some(Self::ChannelMode),
            "approval-v2" | "approval" => Some(Self::ApprovalV2),
            "context-compression" | "compact" => Some(Self::ContextCompression),
            "workspace-profiles" | "workspace" => Some(Self::WorkspaceProfiles),
            "harness-v1-readonly-pilot" | "harness-v1" => Some(Self::HarnessV1ReadonlyPilot),
            "harness-v1-unified-kernel" => Some(Self::HarnessV1UnifiedKernel),
            "harness-v1-graph" => Some(Self::HarnessV1Graph),
            _ => None,
        }
    }

    pub fn all() -> &'static [FeatureFlag] {
        &[
            Self::Skills,
            Self::Workflows,
            Self::GoalMode,
            Self::ChannelMode,
            Self::ApprovalV2,
            Self::ContextCompression,
            Self::WorkspaceProfiles,
            Self::HarnessV1ReadonlyPilot,
            Self::HarnessV1UnifiedKernel,
            Self::HarnessV1Graph,
        ]
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FeatureRegistry {
    #[serde(default)]
    enabled: BTreeSet<String>,
}

impl FeatureRegistry {
    pub fn from_enabled<I, S>(items: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut out = Self::default();
        for item in items {
            out.enabled.insert(item.into());
        }
        out
    }

    pub fn enable(&mut self, feature: impl AsRef<str>) -> bool {
        self.enabled
            .insert(feature.as_ref().trim().to_ascii_lowercase())
    }

    pub fn disable(&mut self, feature: impl AsRef<str>) -> bool {
        self.enabled
            .remove(feature.as_ref().trim().to_ascii_lowercase().as_str())
    }

    pub fn is_enabled(&self, feature: impl AsRef<str>) -> bool {
        self.enabled
            .contains(feature.as_ref().trim().to_ascii_lowercase().as_str())
    }

    pub fn enabled(&self) -> Vec<String> {
        self.enabled.iter().cloned().collect()
    }
}

/// Explicit experiment switch. Default is off; env wins over config.
pub fn harness_readonly_pilot_enabled(features: &FeatureRegistry) -> bool {
    env_flag_enabled("ANYCODE_HARNESS_V1_READONLY_PILOT")
        || features.is_enabled(FeatureFlag::HarnessV1ReadonlyPilot.as_str())
}

/// Explicit experiment switch for the unified Kernel adapters. Default is off; env wins.
pub fn harness_unified_kernel_enabled(features: &FeatureRegistry) -> bool {
    env_flag_enabled("ANYCODE_HARNESS_V1_UNIFIED_KERNEL")
        || features.is_enabled(FeatureFlag::HarnessV1UnifiedKernel.as_str())
}

/// Opt-in GraphRunner product routes. Default off; not implied by `harness-v1`.
pub fn harness_graph_enabled(features: &FeatureRegistry) -> bool {
    env_flag_enabled("ANYCODE_HARNESS_V1_GRAPH")
        || features.is_enabled(FeatureFlag::HarnessV1Graph.as_str())
}

/// Per-project gray allowlist captured at process start. Tests pass the list
/// explicitly and must not mutate process env.
pub fn harness_gray_project_enabled(project_id: &str, allowlist: &[String]) -> bool {
    let id = project_id.trim();
    !id.is_empty() && allowlist.iter().any(|item| item.trim() == id)
}

/// Production helper: read the gray project list once at host construction.
#[must_use]
pub fn harness_gray_projects_from_env() -> Vec<String> {
    std::env::var("ANYCODE_HARNESS_V1_GRAY_PROJECTS")
        .ok()
        .map(|raw| {
            raw.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn env_flag_enabled(name: &str) -> bool {
    std::env::var(name)
        .map(|v| matches!(v.trim(), "1" | "true" | "TRUE" | "yes" | "on"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readonly_pilot_is_off_by_default() {
        let registry = FeatureRegistry::default();
        assert!(!registry.is_enabled(FeatureFlag::HarnessV1ReadonlyPilot.as_str()));
        if std::env::var_os("ANYCODE_HARNESS_V1_READONLY_PILOT").is_none() {
            assert!(!harness_readonly_pilot_enabled(&registry));
        }
    }

    #[test]
    fn readonly_pilot_can_be_enabled_from_config() {
        let registry = FeatureRegistry::from_enabled(["harness-v1-readonly-pilot"]);
        assert!(registry.is_enabled(FeatureFlag::HarnessV1ReadonlyPilot.as_str()));
        assert_eq!(
            FeatureFlag::parse("harness-v1-readonly-pilot"),
            Some(FeatureFlag::HarnessV1ReadonlyPilot)
        );
    }

    #[test]
    fn unified_kernel_is_off_by_default_and_not_aliased_to_harness_v1() {
        let registry = FeatureRegistry::default();
        assert!(!registry.is_enabled(FeatureFlag::HarnessV1UnifiedKernel.as_str()));
        assert_eq!(
            FeatureFlag::parse("harness-v1"),
            Some(FeatureFlag::HarnessV1ReadonlyPilot)
        );
        assert_eq!(
            FeatureFlag::parse("harness-v1-unified-kernel"),
            Some(FeatureFlag::HarnessV1UnifiedKernel)
        );
        if std::env::var_os("ANYCODE_HARNESS_V1_UNIFIED_KERNEL").is_none() {
            assert!(!harness_unified_kernel_enabled(&registry));
        }
    }

    #[test]
    fn graph_flag_is_off_by_default_and_not_aliased() {
        let registry = FeatureRegistry::default();
        assert!(!registry.is_enabled(FeatureFlag::HarnessV1Graph.as_str()));
        assert_eq!(
            FeatureFlag::parse("harness-v1"),
            Some(FeatureFlag::HarnessV1ReadonlyPilot)
        );
        assert_eq!(
            FeatureFlag::parse("harness-v1-graph"),
            Some(FeatureFlag::HarnessV1Graph)
        );
        if std::env::var_os("ANYCODE_HARNESS_V1_GRAPH").is_none() {
            assert!(!harness_graph_enabled(&registry));
        }
        assert!(harness_gray_project_enabled(
            "proj-a",
            &["proj-a".into(), "proj-b".into()]
        ));
        assert!(!harness_gray_project_enabled("proj-c", &["proj-a".into()]));
        assert!(!harness_gray_project_enabled("", &["proj-a".into()]));
    }
}
