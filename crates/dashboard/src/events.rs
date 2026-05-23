use crate::schema::ProjectEvent;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;

const BROADCAST_CAP: usize = 256;

/// Broadcasts project events to SSE subscribers.
#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<ProjectEvent>,
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
        let (tx, _) = broadcast::channel(BROADCAST_CAP);
        Self {
            tx,
            last_event_at: Arc::new(RwLock::new(None)),
        }
    }

    pub fn publish(&self, event: ProjectEvent) {
        if let Ok(mut last) = self.last_event_at.write() {
            *last = Some(event.occurred_at.clone());
        }
        let _ = self.tx.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ProjectEvent> {
        self.tx.subscribe()
    }

    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
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
}
