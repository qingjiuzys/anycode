//! WeChat `AskUserQuestion`: numeric reply fallback (see ADR 008).

use anycode_tools::{
    AskUserQuestionHost, AskUserQuestionHostError, AskUserQuestionRequest, AskUserQuestionResponse,
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task_local;

use super::wx::WxSender;

const UQ_TIMEOUT_SECS: u64 = 300;

task_local! {
    static WECHAT_USER_SCOPE: String;
}

pub(crate) fn current_wechat_user_id() -> Option<String> {
    WECHAT_USER_SCOPE.try_with(|id| id.clone()).ok()
}

pub(crate) async fn with_wechat_user_scope<Fut>(user_id: String, f: Fut) -> Fut::Output
where
    Fut: std::future::Future + Send,
    Fut::Output: Send,
{
    WECHAT_USER_SCOPE.scope(user_id, f).await
}

struct PendingQuestion {
    gen: u64,
    labels: Vec<String>,
    tx: tokio::sync::oneshot::Sender<Result<Vec<String>, ()>>,
    _timeout: tokio::task::JoinHandle<()>,
}

pub(crate) struct WechatQuestionBroker {
    inner: Mutex<BrokerInner>,
}

struct BrokerInner {
    pending: HashMap<String, PendingQuestion>,
    gen: u64,
}

impl WechatQuestionBroker {
    pub(crate) fn new() -> Self {
        Self {
            inner: Mutex::new(BrokerInner {
                pending: HashMap::new(),
                gen: 0,
            }),
        }
    }

    pub(crate) async fn try_resolve_numeric(&self, user_id: &str, text: &str) -> bool {
        let trimmed = text.trim();
        let Ok(idx) = trimmed.parse::<usize>() else {
            return false;
        };
        if idx == 0 {
            return false;
        }
        let option_index = idx - 1;
        let mut g = self.inner.lock().await;
        let Some(p) = g.pending.remove(user_id) else {
            return false;
        };
        p._timeout.abort();
        let Some(label) = p.labels.get(option_index) else {
            let _ = p.tx.send(Err(()));
            return true;
        };
        let _ = p.tx.send(Ok(vec![label.clone()]));
        true
    }

    async fn cancel_by_timeout(&self, user_id: &str, gen: u64) {
        let mut g = self.inner.lock().await;
        let hit = matches!(g.pending.get(user_id), Some(p) if p.gen == gen);
        if hit {
            if let Some(p) = g.pending.remove(user_id) {
                let _ = p.tx.send(Err(()));
            }
        }
    }
}

pub(crate) struct WechatAskUserQuestionHost {
    broker: Arc<WechatQuestionBroker>,
    sender: Arc<WxSender>,
}

impl WechatAskUserQuestionHost {
    pub(crate) fn new(broker: Arc<WechatQuestionBroker>, sender: Arc<WxSender>) -> Self {
        Self { broker, sender }
    }

    pub(crate) fn into_arc(self) -> Arc<dyn AskUserQuestionHost> {
        Arc::new(self) as Arc<dyn AskUserQuestionHost>
    }
}

#[async_trait]
impl AskUserQuestionHost for WechatAskUserQuestionHost {
    async fn ask_user_question(
        &self,
        request: AskUserQuestionRequest,
    ) -> Result<AskUserQuestionResponse, AskUserQuestionHostError> {
        if request.multi_select {
            return Err(AskUserQuestionHostError(
                "multi_select is not supported on WeChat bridge; set multiSelect false".into(),
            ));
        }
        let user_id = current_wechat_user_id().ok_or_else(|| {
            AskUserQuestionHostError(
                "AskUserQuestion is only available during WeChat bridge task execution".into(),
            )
        })?;
        let n = request.options.len();
        if n == 0 {
            return Err(AskUserQuestionHostError("no options".into()));
        }
        if n > 9 {
            return Err(AskUserQuestionHostError(
                "WeChat bridge supports at most 9 numbered options".into(),
            ));
        }

        let labels: Vec<String> = request.options.iter().map(|o| o.label.clone()).collect();
        let (tx, rx) = tokio::sync::oneshot::channel();
        let labels_for_pending = labels.clone();
        let my_gen = {
            let mut g = self.broker.inner.lock().await;
            if let Some(prev) = g.pending.remove(&user_id) {
                prev._timeout.abort();
                let _ = prev.tx.send(Err(()));
            }
            g.gen += 1;
            let gen = g.gen;
            let b = Arc::clone(&self.broker);
            let uid = user_id.clone();
            let timeout = tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(UQ_TIMEOUT_SECS)).await;
                b.cancel_by_timeout(&uid, gen).await;
            });
            g.pending.insert(
                user_id.clone(),
                PendingQuestion {
                    gen,
                    labels: labels_for_pending,
                    tx,
                    _timeout: timeout,
                },
            );
            gen
        };
        let _ = my_gen;

        let header = request.header.trim();
        let question = request.question.trim();
        let mut body = if header.is_empty() {
            question.to_string()
        } else if question.is_empty() {
            header.to_string()
        } else {
            format!("{header}\n{question}")
        };
        body.push_str("\n\n请回复数字：");
        for (i, lab) in labels.iter().enumerate() {
            body.push_str(&format!("\n{}. {lab}", i + 1));
        }

        // context_token is carried by the active chat; AskUserQuestion runs inside task scope
        // where the bridge already knows the chat — use empty token placeholder path via sender API.
        // WxSender requires context_token; host stores it in task_local would be better but
        // we reuse user_id routing only for broker; send uses gate's active chat token in bridge.
        // Bridge passes context_token through WECHAT_USER_SCOPE extension below.
        let context_token = wechat_context_token().ok_or_else(|| {
            AskUserQuestionHostError("WeChat context token missing during AskUserQuestion".into())
        })?;

        self.sender
            .send_text(&user_id, &context_token, &body)
            .await
            .map_err(|e| AskUserQuestionHostError(format!("wechat send_text: {e:#}")))?;

        match rx.await {
            Ok(Ok(selected)) => Ok(AskUserQuestionResponse {
                selected_labels: selected,
            }),
            Ok(Err(())) => Err(AskUserQuestionHostError(
                "user cancelled or timed out".into(),
            )),
            Err(_) => Err(AskUserQuestionHostError("question channel closed".into())),
        }
    }
}

task_local! {
    static WECHAT_CONTEXT_TOKEN: String;
}

pub(crate) fn wechat_context_token() -> Option<String> {
    WECHAT_CONTEXT_TOKEN.try_with(|t| t.clone()).ok()
}

pub(crate) async fn with_wechat_task_scope<Fut>(
    user_id: String,
    context_token: String,
    f: Fut,
) -> Fut::Output
where
    Fut: std::future::Future + Send,
    Fut::Output: Send,
{
    WECHAT_CONTEXT_TOKEN
        .scope(context_token, async {
            with_wechat_user_scope(user_id, f).await
        })
        .await
}
