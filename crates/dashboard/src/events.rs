use crate::observability::chat_events::chat_event_from_project_event;
use crate::schema::{ChatStreamEvent, ProjectEvent};
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;

const BROADCAST_CAP: usize = 512;

/// Broadcasts project events and chat stream events to SSE subscribers.
#[derive(Clone)]
pub struct EventBus {
    project_tx: broadcast::Sender<ProjectEvent>,
    chat_tx: broadcast::Sender<ChatStreamEvent>,
    last_event_at: Arc<RwLock<Option<String>>>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    #[must_use]
    pub fn new() -> Self {
        let (project_tx, _) = broadcast::channel(BROADCAST_CAP);
        let (chat_tx, _) = broadcast::channel(BROADCAST_CAP);
        Self {
            project_tx,
            chat_tx,
            last_event_at: Arc::new(RwLock::new(None)),
        }
    }

    pub fn publish(&self, event: ProjectEvent) {
        if let Ok(mut last) = self.last_event_at.write() {
            *last = Some(event.occurred_at.clone());
        }
        if let Some(chat_evt) = chat_event_from_project_event(&event) {
            let _ = self.chat_tx.send(chat_evt);
        }
        let _ = self.project_tx.send(event);
    }

    pub fn publish_chat(&self, event: ChatStreamEvent) {
        if let Ok(mut last) = self.last_event_at.write() {
            *last = Some(event.at.clone());
        }
        let _ = self.chat_tx.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ProjectEvent> {
        self.project_tx.subscribe()
    }

    pub fn subscribe_chat(&self) -> broadcast::Receiver<ChatStreamEvent> {
        self.chat_tx.subscribe()
    }

    pub fn subscriber_count(&self) -> usize {
        self.project_tx.receiver_count()
    }

    pub fn last_event_at(&self) -> Option<String> {
        self.last_event_at.read().ok().and_then(|g| g.clone())
    }
}

/// Thin wrapper used by runtime integrations.
#[derive(Clone)]
pub struct EventSink {
    bus: Arc<EventBus>,
}

impl EventSink {
    #[must_use]
    pub fn new(bus: Arc<EventBus>) -> Self {
        Self { bus }
    }

    pub fn publish(&self, event: ProjectEvent) {
        self.bus.publish(event);
    }

    pub fn publish_chat(&self, event: ChatStreamEvent) {
        self.bus.publish_chat(event);
    }
}
