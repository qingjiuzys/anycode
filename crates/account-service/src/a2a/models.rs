//! A2A / anyCode handoff types (see docs/a2a/task-mapping.md).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffKind {
    Project,
    Session,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffState {
    PendingApproval,
    Approved,
    Uploading,
    Importing,
    Completed,
    Rejected,
    Failed,
    Expired,
}

impl HandoffState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PendingApproval => "pending_approval",
            Self::Approved => "approved",
            Self::Uploading => "uploading",
            Self::Importing => "importing",
            Self::Completed => "completed",
            Self::Rejected => "rejected",
            Self::Failed => "failed",
            Self::Expired => "expired",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "pending_approval" => Self::PendingApproval,
            "approved" => Self::Approved,
            "uploading" => Self::Uploading,
            "importing" => Self::Importing,
            "completed" => Self::Completed,
            "rejected" => Self::Rejected,
            "failed" => Self::Failed,
            "expired" => Self::Expired,
            _ => return None,
        })
    }
}

/// Agent Card (P1 subset). See docs/a2a/agent-card.schema.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCard {
    pub schema_version: String,
    pub instance_id: String,
    pub device_id: String,
    pub organization_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    pub name: String,
    pub transport: String,
    pub version: String,
    pub capabilities: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl AgentCard {
    pub fn anycode_desktop(
        instance_id: &str,
        device_id: &str,
        organization_id: &str,
        user_id: &str,
        name: &str,
        version: &str,
    ) -> Self {
        Self {
            schema_version: "anycode_agent_card_v1".into(),
            instance_id: instance_id.into(),
            device_id: device_id.into(),
            organization_id: organization_id.into(),
            user_id: Some(user_id.into()),
            name: name.into(),
            transport: "cloud".into(),
            version: version.into(),
            capabilities: vec![
                "handoff.project".into(),
                "handoff.session".into(),
                "streaming.relay".into(),
            ],
            skills: vec![],
            url: None,
            last_seen: Some(Utc::now()),
            metadata: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamPeerView {
    pub user_id: String,
    pub display_name: String,
    pub email: String,
    pub device_id: String,
    pub instance_id: String,
    pub device_name: String,
    pub version: String,
    pub transport: String,
    pub online: bool,
    pub last_seen: DateTime<Utc>,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffTaskView {
    pub id: String,
    pub kind: HandoffKind,
    pub state: HandoffState,
    pub sender_user_id: String,
    pub sender_device_id: String,
    pub sender_instance_id: String,
    pub sender_name: String,
    pub recipient_user_id: String,
    pub recipient_device_id: String,
    pub recipient_instance_id: String,
    pub recipient_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_token: Option<String>,
    pub progress_pct: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// P2 JSON-RPC envelope stub (not wired in P1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2aJsonRpcRequest {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2aVersionNegotiation {
    pub a2a_version: String,
    pub supported_versions: Vec<String>,
}

pub const DEFAULT_A2A_VERSION: &str = "0.1";
pub const PRESENCE_TTL_SECS: i64 = 90;
pub const HANDOFF_TTL_SECS: i64 = 300;
pub const STREAM_TOKEN_TTL_SECS: i64 = 300;
/// In-memory replay buffer cap (must match Desktop cloud export `max_bytes`).
pub const MAX_RELAY_BUFFER_BYTES: usize = 64 * 1024 * 1024;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handoff_state_roundtrip() {
        for s in [
            HandoffState::PendingApproval,
            HandoffState::Approved,
            HandoffState::Uploading,
            HandoffState::Importing,
            HandoffState::Completed,
            HandoffState::Rejected,
            HandoffState::Failed,
            HandoffState::Expired,
        ] {
            assert_eq!(HandoffState::parse(s.as_str()), Some(s));
        }
        assert_eq!(HandoffState::parse("bogus"), None);
    }

    #[test]
    fn agent_card_defaults() {
        let card = AgentCard::anycode_desktop("inst1", "dev1", "org1", "user1", "Desk", "0.2.4");
        assert_eq!(card.schema_version, "anycode_agent_card_v1");
        assert_eq!(card.transport, "cloud");
        assert!(card.capabilities.contains(&"streaming.relay".into()));
        assert!(card.capabilities.contains(&"handoff.project".into()));
    }
}
