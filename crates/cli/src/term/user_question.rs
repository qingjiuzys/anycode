//! Pending `AskUserQuestion` UI (mpsc + oneshot), mirroring [`super::approval::PendingApproval`].

use tokio::sync::oneshot;

/// User must pick option label(s) or cancel (reply `Err(())`).
pub(crate) struct PendingUserQuestion {
    pub header: String,
    pub question: String,
    pub option_labels: Vec<String>,
    pub option_descriptions: Vec<String>,
    /// Reserved for future multi-select UI (channel host currently rejects `multi_select`).
    #[allow(dead_code)]
    pub multi_select: bool,
    pub reply: oneshot::Sender<Result<Vec<String>, ()>>,
}
