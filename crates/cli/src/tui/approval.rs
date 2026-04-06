//! TUI 内嵌工具审批（channel → 主循环 y/n）。

use crate::i18n::tr;
use anycode_security::{ApprovalCallback, SecurityPolicy};
use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot};

pub(crate) struct PendingApproval {
    pub(crate) tool: String,
    pub(crate) input_preview: String,
    pub(crate) reply: oneshot::Sender<bool>,
}

pub(crate) struct TuiApprovalCallback {
    tx: mpsc::Sender<PendingApproval>,
}

impl TuiApprovalCallback {
    pub(crate) fn new(tx: mpsc::Sender<PendingApproval>) -> Self {
        Self { tx }
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
        reply_rx
            .await
            .map_err(|_| anyhow::anyhow!("{}", tr("tui-approval-cancelled")))
    }
}
