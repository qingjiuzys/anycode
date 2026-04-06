//! 微信 y/n 工具审批（与 Node permission broker 对齐）。

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

const TIMEOUT_SECS: u64 = 120;

struct Pending {
    gen: u64,
    tool_name: String,
    tool_input: String,
    tx: tokio::sync::oneshot::Sender<bool>,
    _abort: tokio::task::JoinHandle<()>,
}

#[derive(Clone)]
pub struct PermissionBroker {
    account_id: String,
    inner: Arc<Mutex<BrokerInner>>,
}

struct BrokerInner {
    pending: Option<Pending>,
    timed_out: HashMap<String, std::time::Instant>,
    gen: u64,
}

impl PermissionBroker {
    pub fn new(account_id: String) -> Self {
        Self {
            account_id,
            inner: Arc::new(Mutex::new(BrokerInner {
                pending: None,
                timed_out: HashMap::new(),
                gen: 0,
            })),
        }
    }

    pub async fn create_pending(
        &self,
        tool_name: String,
        tool_input: String,
    ) -> tokio::sync::oneshot::Receiver<bool> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let mut g = self.inner.lock().await;
        if let Some(p) = g.pending.take() {
            let _ = p.tx.send(false);
            p._abort.abort();
        }
        g.gen += 1;
        let my_gen = g.gen;
        let inner = self.inner.clone();
        let acc = self.account_id.clone();
        let abort = tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(TIMEOUT_SECS)).await;
            let mut g = inner.lock().await;
            let hit = matches!(&g.pending, Some(p) if p.gen == my_gen);
            if hit {
                if let Some(p) = g.pending.take() {
                    let _ = p.tx.send(false);
                    g.timed_out.insert(acc, std::time::Instant::now());
                }
            }
        });
        g.pending = Some(Pending {
            gen: my_gen,
            tool_name,
            tool_input,
            tx,
            _abort: abort,
        });
        rx
    }

    pub async fn resolve(&self, allowed: bool) -> bool {
        let mut g = self.inner.lock().await;
        let Some(p) = g.pending.take() else {
            return false;
        };
        p._abort.abort();
        let _ = p.tx.send(allowed);
        true
    }

    pub async fn get_pending(&self) -> Option<(String, String)> {
        let g = self.inner.lock().await;
        g.pending
            .as_ref()
            .map(|p| (p.tool_name.clone(), p.tool_input.clone()))
    }

    pub fn format_message(tool_name: &str, tool_input: &str) -> String {
        use crate::i18n::tr_args;
        use fluent_bundle::FluentArgs;
        let preview: String = tool_input.chars().take(500).collect();
        let mut a = FluentArgs::new();
        a.set("tool", tool_name);
        a.set("input", preview);
        tr_args("wx-permission-request", &a)
    }

    pub async fn is_timed_out(&self) -> bool {
        let g = self.inner.lock().await;
        g.timed_out.contains_key(&self.account_id)
    }

    pub async fn clear_timed_out(&self) {
        let mut g = self.inner.lock().await;
        g.timed_out.remove(&self.account_id);
    }

    pub async fn reject_pending(&self) -> bool {
        self.resolve(false).await
    }
}
