//! Telegram `AskUserQuestion`: inline keyboard + callback routing (see ADR 008).
//!
//! Uses `tokio::task_local` so concurrent chats do not share state. Tool execution must
//! stay on this task (as today); if inner code spawns before AskUserQuestion, the host
//! may not see context.

use anycode_tools::{
    AskUserQuestionHost, AskUserQuestionHostError, AskUserQuestionRequest, AskUserQuestionResponse,
};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task_local;

const UQ_TIMEOUT_SECS: u64 = 300;

task_local! {
    static TELEGRAM_CHAT_SCOPE: i64;
}

pub(crate) fn current_telegram_chat_id() -> Option<i64> {
    TELEGRAM_CHAT_SCOPE.try_with(|&id| id).ok()
}

pub(crate) async fn with_telegram_chat_scope<Fut>(chat_id: i64, f: Fut) -> Fut::Output
where
    Fut: std::future::Future + Send,
    Fut::Output: Send,
{
    TELEGRAM_CHAT_SCOPE.scope(chat_id, f).await
}

struct PendingQuestion {
    gen: u64,
    labels: Vec<String>,
    tx: tokio::sync::oneshot::Sender<Result<Vec<String>, ()>>,
    _timeout: tokio::task::JoinHandle<()>,
}

/// One pending `AskUserQuestion` per chat; new registration evicts the previous (same pattern as `wx/permission`).
pub(crate) struct TelegramQuestionBroker {
    inner: Mutex<BrokerInner>,
}

struct BrokerInner {
    pending: HashMap<i64, PendingQuestion>,
    gen: u64,
}

impl TelegramQuestionBroker {
    pub(crate) fn new() -> Self {
        Self {
            inner: Mutex::new(BrokerInner {
                pending: HashMap::new(),
                gen: 0,
            }),
        }
    }

    /// Returns whether a callback was consumed for this chat.
    pub(crate) async fn resolve_callback(&self, chat_id: i64, option_index: usize) -> bool {
        let mut g = self.inner.lock().await;
        let Some(p) = g.pending.remove(&chat_id) else {
            return false;
        };
        p._timeout.abort();
        let Some(label) = p.labels.get(option_index) else {
            let _ = p.tx.send(Err(()));
            return false;
        };
        let _ = p.tx.send(Ok(vec![label.clone()]));
        true
    }

    pub(crate) async fn cancel_by_timeout(&self, chat_id: i64, gen: u64) {
        let mut g = self.inner.lock().await;
        let hit = matches!(g.pending.get(&chat_id), Some(p) if p.gen == gen);
        if hit {
            if let Some(p) = g.pending.remove(&chat_id) {
                let _ = p.tx.send(Err(()));
            }
        }
    }
}

pub(crate) struct TelegramAskUserQuestionHost {
    broker: Arc<TelegramQuestionBroker>,
    client: Client,
    bot_token: String,
}

impl TelegramAskUserQuestionHost {
    pub(crate) fn new(
        broker: Arc<TelegramQuestionBroker>,
        client: Client,
        bot_token: String,
    ) -> Self {
        Self {
            broker,
            client,
            bot_token,
        }
    }

    pub(crate) fn into_arc(self) -> Arc<dyn AskUserQuestionHost> {
        Arc::new(self) as Arc<dyn AskUserQuestionHost>
    }
}

#[async_trait]
impl AskUserQuestionHost for TelegramAskUserQuestionHost {
    async fn ask_user_question(
        &self,
        request: AskUserQuestionRequest,
    ) -> Result<AskUserQuestionResponse, AskUserQuestionHostError> {
        if request.multi_select {
            return Err(AskUserQuestionHostError(
                "multi_select is not supported on Telegram; set multiSelect false".into(),
            ));
        }
        let chat_id = current_telegram_chat_id().ok_or_else(|| {
            AskUserQuestionHostError(
                "AskUserQuestion is only available during Telegram bridge task execution".into(),
            )
        })?;
        let n = request.options.len();
        if n == 0 {
            return Err(AskUserQuestionHostError("no options".into()));
        }
        if n > 8 {
            return Err(AskUserQuestionHostError(
                "Telegram bridge supports at most 8 options".into(),
            ));
        }

        let labels: Vec<String> = request.options.iter().map(|o| o.label.clone()).collect();

        let (tx, rx) = tokio::sync::oneshot::channel();
        let labels_for_pending = labels.clone();
        let my_gen = {
            let mut g = self.broker.inner.lock().await;
            if let Some(prev) = g.pending.remove(&chat_id) {
                prev._timeout.abort();
                let _ = prev.tx.send(Err(()));
            }
            g.gen += 1;
            let gen = g.gen;
            let b = Arc::clone(&self.broker);
            let timeout = tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(UQ_TIMEOUT_SECS)).await;
                b.cancel_by_timeout(chat_id, gen).await;
            });
            g.pending.insert(
                chat_id,
                PendingQuestion {
                    gen,
                    labels: labels_for_pending,
                    tx,
                    _timeout: timeout,
                },
            );
            gen
        };

        let header = request.header.trim();
        let question = request.question.trim();
        let body = if header.is_empty() {
            question.to_string()
        } else if question.is_empty() {
            header.to_string()
        } else {
            format!("{header}\n{question}")
        };

        let keyboard: Vec<Vec<serde_json::Value>> = (0..n)
            .map(|i| {
                let lab = labels.get(i).cloned().unwrap_or_else(|| format!("{i}"));
                let data = format!("uq:{i}");
                vec![json!({
                    "text": lab.chars().take(64).collect::<String>(),
                    "callback_data": data,
                })]
            })
            .collect();

        let send_url = format!("https://api.telegram.org/bot{}/sendMessage", self.bot_token);
        let payload = json!({
            "chat_id": chat_id,
            "text": body,
            "reply_markup": {
                "inline_keyboard": keyboard,
            }
        });
        let res = self
            .client
            .post(&send_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AskUserQuestionHostError(format!("telegram sendMessage: {e}")))?;
        if !res.status().is_success() {
            let _ = self.broker.cancel_by_timeout(chat_id, my_gen).await;
            let err_body = res.text().await.unwrap_or_default();
            return Err(AskUserQuestionHostError(format!(
                "telegram sendMessage HTTP error: {err_body}"
            )));
        }

        match rx.await {
            Ok(Ok(selected)) => Ok(AskUserQuestionResponse {
                selected_labels: selected,
            }),
            Ok(Err(())) => Err(AskUserQuestionHostError(
                "question timed out or was replaced".into(),
            )),
            Err(_) => Err(AskUserQuestionHostError("question channel closed".into())),
        }
    }
}

/// Parse `uq:<n>` callback payloads (Telegram `callback_data` max 64 bytes).
pub(crate) fn parse_uq_callback_data(data: &str) -> Option<usize> {
    let rest = data.strip_prefix("uq:")?;
    rest.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_uq_callback_data_accepts_indices() {
        assert_eq!(parse_uq_callback_data("uq:0"), Some(0));
        assert_eq!(parse_uq_callback_data("uq:7"), Some(7));
        assert_eq!(parse_uq_callback_data("bad"), None);
    }
}
