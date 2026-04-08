//! TUI 内嵌工具审批（channel → 主循环 y/n）。

use crate::i18n::tr;
use anycode_security::{find_project_root, ApprovalCallback, ProjectApprovalStore, SecurityPolicy};
use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug, Clone, Copy)]
pub(crate) enum ApprovalDecision {
    AllowOnce,
    AllowToolForProject,
    Deny,
}

pub(crate) struct PendingApproval {
    pub(crate) tool: String,
    pub(crate) input_preview: String,
    pub(crate) reply: oneshot::Sender<ApprovalDecision>,
}

pub(crate) struct TuiApprovalCallback {
    tx: mpsc::Sender<PendingApproval>,
    project_allow: std::sync::Arc<std::sync::Mutex<ProjectApprovalStore>>,
}

impl TuiApprovalCallback {
    pub(crate) fn new(tx: mpsc::Sender<PendingApproval>) -> Self {
        Self {
            tx,
            project_allow: std::sync::Arc::new(std::sync::Mutex::new(
                ProjectApprovalStore::load_or_new(),
            )),
        }
    }
}

#[async_trait]
impl ApprovalCallback for TuiApprovalCallback {
    async fn request_approval(
        &self,
        tool: &str,
        input: &serde_json::Value,
        _policy: &SecurityPolicy,
    ) -> anyhow::Result<bool> {
        if let Ok(store) = self.project_allow.lock() {
            let root =
                find_project_root().unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
            if store.tool_allowed(&root, tool) {
                return Ok(true);
            }
        }
        let (reply_tx, reply_rx) = oneshot::channel();
        let pending = PendingApproval {
            tool: tool.to_string(),
            input_preview: serde_json::to_string_pretty(input).unwrap_or_else(|_| "{}".to_string()),
            reply: reply_tx,
        };
        self.tx
            .send(pending)
            .await
            .map_err(|_| anyhow::anyhow!("{}", tr("tui-approval-tui-exited")))?;
        let decision = reply_rx
            .await
            .map_err(|_| anyhow::anyhow!("{}", tr("tui-approval-cancelled")))?;
        match decision {
            ApprovalDecision::AllowOnce => Ok(true),
            ApprovalDecision::AllowToolForProject => {
                let root = find_project_root()
                    .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
                if let Ok(mut store) = self.project_allow.lock() {
                    store.allow_tool(&root, tool);
                }
                Ok(true)
            }
            ApprovalDecision::Deny => Ok(false),
        }
    }
}
