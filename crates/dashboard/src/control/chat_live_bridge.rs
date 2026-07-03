//! Bridge in-process [`LiveTraceEvent`] → dashboard `chat_event` SSE.

use crate::events::EventBus;
use crate::observability::chat_events::chat_event_from_live_trace;
use anycode_core::LiveTraceEvent;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::UnboundedReceiver;

const DELTA_FLUSH: Duration = Duration::from_millis(50);

struct BridgeState {
    assistant_buffers: HashMap<u32, String>,
    pending_delta_turn: Option<u32>,
    last_delta_flush: HashMap<u32, Instant>,
}

impl BridgeState {
    fn new() -> Self {
        Self {
            assistant_buffers: HashMap::new(),
            pending_delta_turn: None,
            last_delta_flush: HashMap::new(),
        }
    }

    fn should_flush_delta(&mut self, turn: u32) -> bool {
        let now = Instant::now();
        let last = self.last_delta_flush.entry(turn).or_insert(now);
        if now.duration_since(*last) >= DELTA_FLUSH {
            *last = now;
            true
        } else {
            false
        }
    }

    fn flush_pending_delta(&mut self, events: &EventBus, session_id: &str, project_id: &str) {
        let Some(turn) = self.pending_delta_turn.take() else {
            return;
        };
        let full = self
            .assistant_buffers
            .get(&turn)
            .cloned()
            .unwrap_or_default();
        if full.is_empty() {
            return;
        }
        let chat_evt = crate::observability::chat_events::assistant_delta_event(
            session_id, project_id, turn, "", &full,
        );
        events.publish_chat(chat_evt);
    }
}

/// Consume runtime live trace events and publish `chat_event` SSE immediately.
pub fn spawn_live_bridge(
    events: Arc<EventBus>,
    session_id: String,
    project_id: String,
    mut rx: UnboundedReceiver<LiveTraceEvent>,
) {
    tokio::spawn(async move {
        let mut state = BridgeState::new();
        let mut flush_timer = tokio::time::interval(DELTA_FLUSH);
        flush_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                msg = rx.recv() => {
                    let Some(evt) = msg else { break };
                    match &evt {
                        LiveTraceEvent::AssistantDelta { turn, .. } => {
                            if let Some(chat_evt) = chat_event_from_live_trace(
                                &session_id,
                                &project_id,
                                &evt,
                                &mut state.assistant_buffers,
                            ) {
                                if state.should_flush_delta(*turn) {
                                    events.publish_chat(chat_evt);
                                } else {
                                    state.pending_delta_turn = Some(*turn);
                                }
                            }
                            continue;
                        }
                        LiveTraceEvent::AssistantDone { .. }
                        | LiveTraceEvent::TurnDone { .. } => {
                            state.flush_pending_delta(&events, &session_id, &project_id);
                        }
                        _ => {}
                    }
                    if let Some(chat_evt) = chat_event_from_live_trace(
                        &session_id,
                        &project_id,
                        &evt,
                        &mut state.assistant_buffers,
                    ) {
                        events.publish_chat(chat_evt);
                    }
                }
                _ = flush_timer.tick() => {
                    if state.pending_delta_turn.is_some() {
                        state.flush_pending_delta(&events, &session_id, &project_id);
                    }
                }
            }
        }
        state.flush_pending_delta(&events, &session_id, &project_id);
    });
}

#[must_use]
pub fn log_tail_fallback_enabled() -> bool {
    matches!(
        std::env::var("ANYCODE_DASHBOARD_LOG_TAIL_FALLBACK").as_deref(),
        Ok("1") | Ok("true") | Ok("on")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_tail_fallback_defaults_off() {
        std::env::remove_var("ANYCODE_DASHBOARD_LOG_TAIL_FALLBACK");
        assert!(!log_tail_fallback_enabled());
    }
}
