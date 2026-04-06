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
