//! Dashboard Web + optional TUI tool approval callback.

use crate::term::approval::{ApprovalDecision, PendingApproval};
use anycode_dashboard::approval_ipc::{self, SESSION_ENV};
use anycode_security::{
    find_project_root, ApprovalCallback, InteractiveApprovalCallback, ProjectApprovalStore,
    PromptFormat, SecurityPolicy,
};
use async_trait::async_trait;
use std::io::IsTerminal;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

const WEB_POLL_MS: u64 = 400;
const WEB_TIMEOUT: Duration = Duration::from_secs(30 * 60);

pub struct WorkbenchApprovalCallback {
    tui_tx: Option<mpsc::Sender<PendingApproval>>,
    fallback: Option<InteractiveApprovalCallback>,
    project_allow: Arc<std::sync::Mutex<ProjectApprovalStore>>,
}

impl WorkbenchApprovalCallback {
    pub fn with_tui_channel(tx: mpsc::Sender<PendingApproval>) -> Self {
        Self {
            tui_tx: Some(tx),
            fallback: None,
            project_allow: Arc::new(std::sync::Mutex::new(ProjectApprovalStore::load_or_new())),
        }
    }

    pub fn web_and_cli() -> Self {
        let fmt = if std::io::stdout().is_terminal() {
            PromptFormat::CLI
        } else {
            PromptFormat::Silent
        };
        Self {
            tui_tx: None,
            fallback: Some(InteractiveApprovalCallback::new(fmt)),
            project_allow: Arc::new(std::sync::Mutex::new(ProjectApprovalStore::load_or_new())),
        }
    }

    fn session_id(&self) -> Option<String> {
        std::env::var(SESSION_ENV).ok().filter(|s| !s.is_empty())
    }

    fn tool_allowed_for_project(&self, tool: &str) -> bool {
        let Ok(store) = self.project_allow.lock() else {
            return false;
        };
        let root =
            find_project_root().unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        store.tool_allowed(&root, tool)
    }

    fn allow_tool_for_project(&self, tool: &str) {
        let root =
            find_project_root().unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        if let Ok(mut store) = self.project_allow.lock() {
            store.allow_tool(&root, tool);
        }
    }

    async fn wait_web(&self, approval_id: &str) -> Option<String> {
        let deadline = tokio::time::Instant::now() + WEB_TIMEOUT;
        loop {
            if let Some(decision) = approval_ipc::poll_response(approval_id) {
                return Some(decision);
            }
            if tokio::time::Instant::now() >= deadline {
                approval_ipc::clear_pending(approval_id);
                return Some("deny".into());
            }
            tokio::time::sleep(Duration::from_millis(WEB_POLL_MS)).await;
        }
    }

    async fn wait_tui(
        &self,
        tool: &str,
        input: &serde_json::Value,
    ) -> anyhow::Result<Option<ApprovalDecision>> {
        let Some(tx) = &self.tui_tx else {
            return Ok(None);
        };
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        let pending = PendingApproval {
            tool: tool.to_string(),
            input_preview: serde_json::to_string_pretty(input).unwrap_or_else(|_| "{}".to_string()),
            reply: reply_tx,
        };
        tx.send(pending)
            .await
            .map_err(|_| anyhow::anyhow!("approval UI channel closed"))?;
        match reply_rx.await {
            Ok(d) => Ok(Some(d)),
            Err(_) => Ok(Some(ApprovalDecision::Deny)),
        }
    }

    fn apply_web_decision(&self, tool: &str, decision: &str) -> bool {
        match decision {
            "allow_once" => true,
            "allow_tool" => {
                self.allow_tool_for_project(tool);
                true
            }
            _ => false,
        }
    }
}

#[async_trait]
impl ApprovalCallback for WorkbenchApprovalCallback {
    async fn request_approval(
        &self,
        tool: &str,
        input: &serde_json::Value,
        _policy: &SecurityPolicy,
    ) -> anyhow::Result<bool> {
        if self.tool_allowed_for_project(tool) {
            return Ok(true);
        }

        let preview = serde_json::to_string_pretty(input).unwrap_or_else(|_| "{}".to_string());
        let web_id = if approval_ipc::web_approvals_enabled() {
            self.session_id()
                .and_then(|sid| approval_ipc::register_pending(&sid, tool, &preview).ok())
        } else {
            None
        };

        if let (Some(ref id), Some(ref sid)) = (&web_id, self.session_id()) {
            tracing::info!(
                target: "anycode_dashboard",
                session_id = %sid,
                approval_id = %id,
                tool = %tool,
                "tool approval pending — respond in dashboard Security inbox"
            );
        }

        if let Some(id) = web_id.clone() {
            if self.tui_tx.is_some() {
                tokio::select! {
                    tui = self.wait_tui(tool, input) => {
                        approval_ipc::clear_pending(&id);
                        if let Ok(Some(decision)) = tui {
                            return Ok(match decision {
                                ApprovalDecision::AllowOnce => true,
                                ApprovalDecision::AllowToolForProject => {
                                    self.allow_tool_for_project(tool);
                                    true
                                }
                                ApprovalDecision::Deny => false,
                            });
                        }
                    }
                    web = self.wait_web(&id) => {
                        if let Some(d) = web {
                            return Ok(self.apply_web_decision(tool, &d));
                        }
                    }
                }
                approval_ipc::clear_pending(&id);
            } else {
                if let Some(d) = self.wait_web(&id).await {
                    return Ok(self.apply_web_decision(tool, &d));
                }
            }
        }

        if let Ok(Some(decision)) = self.wait_tui(tool, input).await {
            return Ok(match decision {
                ApprovalDecision::AllowOnce => true,
                ApprovalDecision::AllowToolForProject => {
                    self.allow_tool_for_project(tool);
                    true
                }
                ApprovalDecision::Deny => false,
            });
        }

        if let Some(ref fb) = self.fallback {
            return fb.request_approval(tool, input, _policy).await;
        }

        Ok(false)
    }
}
