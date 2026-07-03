//! TUI approval message types shared with workbench callback.

use tokio::sync::oneshot;

#[derive(Debug, Clone, Copy)]
pub enum ApprovalDecision {
    AllowOnce,
    AllowToolForProject,
    Deny,
}

pub struct PendingApproval {
    pub tool: String,
    pub input_preview: String,
    pub reply: oneshot::Sender<ApprovalDecision>,
}
