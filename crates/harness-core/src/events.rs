use crate::{digest::digest, Result, RunContext};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub version: u32,
    pub run_id: Uuid,
    pub root_id: Uuid,
    pub parent_run_id: Option<Uuid>,
    pub scope_digest: String,
    pub kind: String,
    pub data: Value,
}
impl Event {
    pub fn new(ctx: &RunContext, kind: impl Into<String>, data: Value) -> Result<Self> {
        Ok(Self {
            version: 1,
            run_id: ctx.id(),
            root_id: ctx.root_id(),
            parent_run_id: ctx.parent_id(),
            scope_digest: digest(ctx.scope())?,
            kind: kind.into(),
            data,
        })
    }
}
/// Durable events are written before delivery. A full UI channel can drop previews,
/// never change execution truth: the UI reconnects/replays using the journal cursor.
#[derive(Clone, Default)]
pub struct PreviewBus {
    sender: Option<tokio::sync::mpsc::Sender<Value>>,
}
impl PreviewBus {
    pub fn bounded(capacity: usize) -> (Self, tokio::sync::mpsc::Receiver<Value>) {
        let (tx, rx) = tokio::sync::mpsc::channel(capacity.clamp(1, 4096));
        (Self { sender: Some(tx) }, rx)
    }
    pub fn publish(&self, preview: Value) {
        if let Some(tx) = &self.sender {
            let _ = tx.try_send(preview);
        }
    }
}
