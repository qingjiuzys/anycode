use crate::i18n::tr;
use crate::wx::ilink::WxSender;
use crate::wx::permission::PermissionBroker;
use crate::wx::store::{save_session, SessionState, WcSession};
use anycode_security::approval_presenter::{render_approval_request, ApprovalSurface};
use anycode_security::ApprovalCallback;
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct ActiveChat {
    pub from_user_id: String,
    pub context_token: String,
}

#[derive(Clone)]
pub struct WechatApprovalGate {
    data_root: PathBuf,
    account_id: String,
    session: Arc<Mutex<WcSession>>,
    active: Arc<Mutex<Option<ActiveChat>>>,
    sender: Arc<WxSender>,
    perm_broker: PermissionBroker,
}

impl WechatApprovalGate {
    pub fn new(
        data_root: PathBuf,
        account_id: String,
        session: Arc<Mutex<WcSession>>,
        active: Arc<Mutex<Option<ActiveChat>>>,
        sender: Arc<WxSender>,
        broker: PermissionBroker,
    ) -> Self {
        Self {
            data_root,
            account_id,
            session,
            active,
            sender,
            perm_broker: broker,
        }
    }

    pub async fn set_active_chat(&self, chat: Option<ActiveChat>) {
        *self.active.lock().await = chat;
    }

    pub fn permission_broker(&self) -> PermissionBroker {
        self.perm_broker.clone()
    }
}

#[async_trait]
impl ApprovalCallback for WechatApprovalGate {
    async fn request_approval(
        &self,
        tool: &str,
        input: &serde_json::Value,
        _policy: &anycode_core::SecurityPolicy,
    ) -> anyhow::Result<bool> {
        let chat = self
            .active
            .lock()
            .await
            .clone()
            .ok_or_else(|| anyhow::anyhow!("{}", tr("wx-no-active-session")))?;

        {
            let mut s = self.session.lock().await;
            s.state = SessionState::WaitingPermission;
            save_session(&self.data_root, &self.account_id, &s)?;
        }

        let input_str = input.to_string();
        let rx = self
            .perm_broker
            .create_pending(tool.to_string(), input_str.clone())
            .await;
        let msg = format!(
            "{}\n\n{}",
            render_approval_request(ApprovalSurface::WeChat, tool, input),
            PermissionBroker::format_message(tool, &input_str)
        );
        self.sender
            .send_text(&chat.from_user_id, &chat.context_token, &msg)
            .await?;

        let ok = rx.await.unwrap_or(false);

        {
            let mut s = self.session.lock().await;
            s.state = SessionState::Processing;
            let _ = save_session(&self.data_root, &self.account_id, &s);
        }

        Ok(ok)
    }
}
