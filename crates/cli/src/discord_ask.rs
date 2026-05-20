//! Discord `AskUserQuestion`: numbered reply fallback (see ADR 008).

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
    static DISCORD_CHANNEL_SCOPE: String;
}

pub(crate) fn current_discord_channel_id() -> Option<String> {
    DISCORD_CHANNEL_SCOPE.try_with(|id| id.clone()).ok()
}

pub(crate) async fn with_discord_channel_scope<Fut>(channel_id: String, f: Fut) -> Fut::Output
where
    Fut: std::future::Future + Send,
    Fut::Output: Send,
{
    DISCORD_CHANNEL_SCOPE.scope(channel_id, f).await
}

struct PendingQuestion {
    gen: u64,
    labels: Vec<String>,
    tx: tokio::sync::oneshot::Sender<Result<Vec<String>, ()>>,
    _timeout: tokio::task::JoinHandle<()>,
}

pub(crate) struct DiscordQuestionBroker {
    inner: Mutex<BrokerInner>,
}

struct BrokerInner {
    pending: HashMap<String, PendingQuestion>,
    gen: u64,
}

impl DiscordQuestionBroker {
    pub(crate) fn new() -> Self {
        Self {
            inner: Mutex::new(BrokerInner {
                pending: HashMap::new(),
                gen: 0,
            }),
        }
    }

    /// Returns true when the message was consumed as a numeric option reply.
    pub(crate) async fn try_resolve_numeric(&self, channel_id: &str, text: &str) -> bool {
        let trimmed = text.trim();
        let Ok(idx) = trimmed.parse::<usize>() else {
            return false;
        };
        if idx == 0 {
            return false;
        }
        let option_index = idx - 1;
        let mut g = self.inner.lock().await;
        let Some(p) = g.pending.remove(channel_id) else {
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

    async fn cancel_by_timeout(&self, channel_id: &str, gen: u64) {
        let mut g = self.inner.lock().await;
        let hit = matches!(g.pending.get(channel_id), Some(p) if p.gen == gen);
        if hit {
            if let Some(p) = g.pending.remove(channel_id) {
                let _ = p.tx.send(Err(()));
            }
        }
    }
}

pub(crate) struct DiscordAskUserQuestionHost {
    broker: Arc<DiscordQuestionBroker>,
    client: Client,
    bot_token: String,
    channel_id: String,
}

impl DiscordAskUserQuestionHost {
    pub(crate) fn new(
        broker: Arc<DiscordQuestionBroker>,
        client: Client,
        bot_token: String,
        channel_id: String,
    ) -> Self {
        Self {
            broker,
            client,
            bot_token,
            channel_id,
        }
    }

    pub(crate) fn into_arc(self) -> Arc<dyn AskUserQuestionHost> {
        Arc::new(self) as Arc<dyn AskUserQuestionHost>
    }
}

#[async_trait]
impl AskUserQuestionHost for DiscordAskUserQuestionHost {
    async fn ask_user_question(
        &self,
        request: AskUserQuestionRequest,
    ) -> Result<AskUserQuestionResponse, AskUserQuestionHostError> {
        if request.multi_select {
            return Err(AskUserQuestionHostError(
                "multi_select is not supported on Discord bridge; set multiSelect false".into(),
            ));
        }
        let scope_channel = current_discord_channel_id().ok_or_else(|| {
            AskUserQuestionHostError(
                "AskUserQuestion is only available during Discord bridge task execution".into(),
            )
        })?;
        if scope_channel != self.channel_id {
            return Err(AskUserQuestionHostError(
                "Discord AskUserQuestion channel scope mismatch".into(),
            ));
        }
        let n = request.options.len();
        if n == 0 {
            return Err(AskUserQuestionHostError("no options".into()));
        }
        if n > 9 {
            return Err(AskUserQuestionHostError(
                "Discord bridge supports at most 9 numbered options".into(),
            ));
        }

        let labels: Vec<String> = request.options.iter().map(|o| o.label.clone()).collect();
        let (tx, rx) = tokio::sync::oneshot::channel();
        let labels_for_pending = labels.clone();
        let channel_id = self.channel_id.clone();
        let my_gen = {
            let mut g = self.broker.inner.lock().await;
            if let Some(prev) = g.pending.remove(&channel_id) {
                prev._timeout.abort();
                let _ = prev.tx.send(Err(()));
            }
            g.gen += 1;
            let gen = g.gen;
            let b = Arc::clone(&self.broker);
            let ch = channel_id.clone();
            let timeout = tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(UQ_TIMEOUT_SECS)).await;
                b.cancel_by_timeout(&ch, gen).await;
            });
            g.pending.insert(
                channel_id.clone(),
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
        let mut body = if header.is_empty() {
            question.to_string()
        } else if question.is_empty() {
            header.to_string()
        } else {
            format!("{header}\n{question}")
        };
        body.push_str("\n\nReply with a number:");
        for (i, lab) in labels.iter().enumerate() {
            body.push_str(&format!("\n{}. {lab}", i + 1));
        }

        let send_url = format!(
            "https://discord.com/api/v10/channels/{}/messages",
            self.channel_id
        );
        let payload = json!({ "content": body });
        let res = self
            .client
            .post(&send_url)
            .header("Authorization", format!("Bot {}", self.bot_token))
            .json(&payload)
            .send()
            .await
            .map_err(|e| AskUserQuestionHostError(format!("discord sendMessage: {e}")))?;
        if !res.status().is_success() {
            let _ = self
                .broker
                .cancel_by_timeout(&self.channel_id, my_gen)
                .await;
            let err_body = res.text().await.unwrap_or_default();
            return Err(AskUserQuestionHostError(format!(
                "discord sendMessage HTTP error: {err_body}"
            )));
        }

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
