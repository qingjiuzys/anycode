//! Handoff request state machine.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffParty {
    pub instance_id: String,
    pub device_name: String,
    pub host: String,
    pub lan_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffRecord {
    pub id: String,
    pub kind: HandoffKind,
    pub state: HandoffState,
    pub direction: HandoffDirection,
    pub sender: HandoffParty,
    pub recipient: HandoffParty,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    pub session_id: Option<String>,
    pub session_title: Option<String>,
    pub target_project_id: Option<String>,
    pub target_root_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upload_token: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default)]
    pub progress_pct: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundle_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffDirection {
    Outgoing,
    Incoming,
}

impl HandoffRecord {
    pub fn new_outgoing(
        kind: HandoffKind,
        sender: HandoffParty,
        recipient: HandoffParty,
        project_id: Option<String>,
        project_name: Option<String>,
        session_id: Option<String>,
        session_title: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: format!("ho_{}", Uuid::new_v4().simple()),
            kind,
            state: HandoffState::PendingApproval,
            direction: HandoffDirection::Outgoing,
            sender,
            recipient,
            project_id,
            project_name,
            session_id,
            session_title,
            target_project_id: None,
            target_root_path: None,
            upload_token: None,
            created_at: now,
            updated_at: now,
            error: None,
            progress_pct: 0,
            bundle_path: None,
        }
    }

    pub fn new_incoming(req: IncomingHandoffRequest) -> Self {
        let now = Utc::now();
        Self {
            id: req.id.clone(),
            kind: req.kind,
            state: HandoffState::PendingApproval,
            direction: HandoffDirection::Incoming,
            sender: req.sender,
            recipient: req.recipient,
            project_id: req.project_id,
            project_name: req.project_name,
            session_id: req.session_id,
            session_title: req.session_title,
            target_project_id: None,
            target_root_path: None,
            upload_token: None,
            created_at: now,
            updated_at: now,
            error: None,
            progress_pct: 0,
            bundle_path: None,
        }
    }

    pub fn approve(&mut self, target_root_path: Option<String>, target_project_id: Option<String>) {
        self.state = HandoffState::Approved;
        self.target_root_path = target_root_path;
        self.target_project_id = target_project_id;
        self.upload_token = Some(format!("ht_{}", Uuid::new_v4().simple()));
        self.updated_at = Utc::now();
    }

    pub fn reject(&mut self) {
        self.state = HandoffState::Rejected;
        self.updated_at = Utc::now();
    }

    pub fn is_token_valid(&self, token: &str) -> bool {
        self.upload_token.as_deref() == Some(token)
            && self.state == HandoffState::Approved
            && Utc::now() - self.updated_at < Duration::minutes(5)
    }

    pub fn expire_if_stale(&mut self) {
        if self.state == HandoffState::PendingApproval
            && Utc::now() - self.created_at > Duration::minutes(10)
        {
            self.state = HandoffState::Expired;
            self.updated_at = Utc::now();
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingHandoffRequest {
    pub id: String,
    pub kind: HandoffKind,
    pub sender: HandoffParty,
    pub recipient: HandoffParty,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    pub session_id: Option<String>,
    pub session_title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutgoingHandoffStatus {
    pub id: String,
    pub state: HandoffState,
    pub progress_pct: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upload_token: Option<String>,
}

impl From<&HandoffRecord> for OutgoingHandoffStatus {
    fn from(r: &HandoffRecord) -> Self {
        Self {
            id: r.id.clone(),
            state: r.state,
            progress_pct: r.progress_pct,
            error: r.error.clone(),
            upload_token: r.upload_token.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffApprovedNotice {
    pub id: String,
    pub upload_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_root_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_project_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(id: &str) -> HandoffParty {
        HandoffParty {
            instance_id: id.into(),
            device_name: "test".into(),
            host: "192.168.1.2".into(),
            lan_port: 43181,
        }
    }

    #[test]
    fn outgoing_starts_pending() {
        let r = HandoffRecord::new_outgoing(
            HandoffKind::Project,
            party("a"),
            party("b"),
            Some("proj_1".into()),
            Some("Demo".into()),
            None,
            None,
        );
        assert_eq!(r.state, HandoffState::PendingApproval);
        assert_eq!(r.direction, HandoffDirection::Outgoing);
    }

    #[test]
    fn approve_issues_token() {
        let mut r = HandoffRecord::new_incoming(IncomingHandoffRequest {
            id: "ho_test".into(),
            kind: HandoffKind::Session,
            sender: party("a"),
            recipient: party("b"),
            project_id: Some("p".into()),
            project_name: None,
            session_id: Some("s".into()),
            session_title: None,
        });
        r.approve(None, None);
        assert_eq!(r.state, HandoffState::Approved);
        assert!(r.upload_token.is_some());
        assert!(r.is_token_valid(r.upload_token.as_ref().unwrap()));
    }

    #[test]
    fn reject_marks_rejected() {
        let mut r = HandoffRecord::new_outgoing(
            HandoffKind::Project,
            party("a"),
            party("b"),
            None,
            None,
            None,
            None,
        );
        r.reject();
        assert_eq!(r.state, HandoffState::Rejected);
    }
}
