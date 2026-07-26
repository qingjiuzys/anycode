//! Bridge in-process [`LiveTraceEvent`] → dashboard `chat_event` SSE.

use crate::db::DashboardDb;
use crate::events::EventBus;
use crate::observability::chat_events::{chat_event_from_live_trace, turn_phase_event};
use crate::observability::chat_turn_log::persist_and_enrich;
use crate::schema::ChatStreamEvent;
use anycode_core::LiveTraceEvent;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::UnboundedReceiver;

const DELTA_FLUSH: Duration = Duration::from_millis(50);

struct BridgeState {
    assistant_raw_buffers: HashMap<u32, String>,
    assistant_display_buffers: HashMap<u32, String>,
    thinking_buffers: HashMap<u32, String>,
    pending_delta_turn: Option<u32>,
    last_delta_flush: HashMap<u32, Instant>,
    streaming_phases: HashSet<u32>,
    tool_phases: HashSet<u32>,
    narration_turns: HashSet<u32>,
}

impl BridgeState {
    fn new() -> Self {
        Self {
            assistant_raw_buffers: HashMap::new(),
            assistant_display_buffers: HashMap::new(),
            thinking_buffers: HashMap::new(),
            pending_delta_turn: None,
            last_delta_flush: HashMap::new(),
            streaming_phases: HashSet::new(),
            tool_phases: HashSet::new(),
            narration_turns: HashSet::new(),
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

    async fn flush_pending_delta(
        &mut self,
        db: &DashboardDb,
        events: &EventBus,
        session_id: &str,
        project_id: &str,
        user_turn_id: u32,
    ) {
        let Some(turn) = self.pending_delta_turn.take() else {
            return;
        };
        let full = self
            .assistant_display_buffers
            .get(&turn)
            .cloned()
            .unwrap_or_default();
        if full.is_empty() {
            return;
        }
        let chat_evt = crate::observability::chat_events::assistant_delta_event(
            session_id,
            project_id,
            user_turn_id,
            turn,
            "",
            &full,
            self.narration_turns.contains(&turn),
        );
        publish_persisted(db, events, chat_evt, user_turn_id).await;
    }

    fn phase_for_event(&mut self, evt: &LiveTraceEvent) -> Option<&'static str> {
        match evt {
            LiveTraceEvent::LlmRequestStart { .. } => Some("waiting_first_token"),
            LiveTraceEvent::AssistantDelta { turn, .. } => {
                if self.streaming_phases.insert(*turn) {
                    Some("streaming")
                } else {
                    None
                }
            }
            LiveTraceEvent::ToolCallStart { turn, .. } => {
                if self.tool_phases.insert(*turn) {
                    Some("running_tools")
                } else {
                    None
                }
            }
            LiveTraceEvent::TurnDone { .. } => {
                self.streaming_phases.clear();
                self.tool_phases.clear();
                self.narration_turns.clear();
                self.thinking_buffers.clear();
                None
            }
            _ => None,
        }
    }
}

async fn publish_persisted(
    db: &DashboardDb,
    events: &EventBus,
    chat_evt: ChatStreamEvent,
    conversation_turn_id: u32,
) {
    match persist_and_enrich(db, chat_evt, conversation_turn_id).await {
        Ok(enriched) => events.publish_chat(enriched),
        Err(error) => {
            tracing::warn!(%error, "chat turn event persist failed");
        }
    }
}

async fn publish_turn_phase(
    db: &DashboardDb,
    events: &EventBus,
    session_id: &str,
    project_id: &str,
    user_turn_id: u32,
    turn: u32,
    phase: &str,
) {
    let chat_evt = turn_phase_event(session_id, project_id, user_turn_id, turn, phase);
    publish_persisted(db, events, chat_evt, user_turn_id).await;
}

/// Consume runtime live trace events, persist canonical events, then publish SSE.
pub fn spawn_live_bridge(
    events: Arc<EventBus>,
    db: DashboardDb,
    session_id: String,
    project_id: String,
    user_turn_id: u32,
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
                    let phase_turn = match &evt {
                        LiveTraceEvent::LlmRequestStart { turn } => Some(*turn),
                        LiveTraceEvent::AssistantDelta { turn, .. } => Some(*turn),
                        LiveTraceEvent::ToolCallStart { turn, .. } => Some(*turn),
                        _ => None,
                    };
                    if let Some(phase) = state.phase_for_event(&evt) {
                        if let Some(turn) = phase_turn {
                            publish_turn_phase(
                                &db,
                                &events,
                                &session_id,
                                &project_id,
                                user_turn_id,
                                turn,
                                phase,
                            ).await;
                        }
                    }
                    match &evt {
                        LiveTraceEvent::AssistantNarrationMark { turn } => {
                            state.narration_turns.insert(*turn);
                            state.flush_pending_delta(
                                &db,
                                &events,
                                &session_id,
                                &project_id,
                                user_turn_id,
                            )
                            .await;
                            if let Some(chat_evt) = chat_event_from_live_trace(
                                &session_id,
                                &project_id,
                                user_turn_id,
                                &evt,
                                &mut state.assistant_raw_buffers,
                                &mut state.assistant_display_buffers,
                            ) {
                                publish_persisted(&db, &events, chat_evt, user_turn_id).await;
                            }
                            continue;
                        }
                        LiveTraceEvent::ThinkingDelta { turn, delta } => {
                            let buf = state.thinking_buffers.entry(*turn).or_default();
                            buf.push_str(delta);
                            let chat_evt = crate::observability::chat_events::thinking_delta_event(
                                &session_id,
                                &project_id,
                                user_turn_id,
                                *turn,
                                buf,
                            );
                            publish_persisted(&db, &events, chat_evt, user_turn_id).await;
                            continue;
                        }
                        LiveTraceEvent::AssistantDelta { turn, narration, .. } => {
                            if *narration {
                                state.narration_turns.insert(*turn);
                            }
                            if let Some(chat_evt) = chat_event_from_live_trace(
                                &session_id,
                                &project_id,
                                user_turn_id,
                                &evt,
                                &mut state.assistant_raw_buffers,
                                &mut state.assistant_display_buffers,
                            ) {
                                if state.should_flush_delta(*turn) {
                                    publish_persisted(&db, &events, chat_evt, user_turn_id).await;
                                } else {
                                    state.pending_delta_turn = Some(*turn);
                                }
                            }
                            continue;
                        }
                        LiveTraceEvent::AssistantDone { .. }
                        | LiveTraceEvent::TurnDone { .. } => {
                            state.flush_pending_delta(&db, &events, &session_id, &project_id, user_turn_id).await;
                        }
                        _ => {}
                    }
                    if let Some(chat_evt) = chat_event_from_live_trace(
                        &session_id,
                        &project_id,
                        user_turn_id,
                        &evt,
                        &mut state.assistant_raw_buffers,
                        &mut state.assistant_display_buffers,
                    ) {
                        if let LiveTraceEvent::ArtifactReady { artifact, .. } = &evt {
                            if let Some(path) = artifact.path.as_deref() {
                                let kind = artifact.resolved_kind();
                                let title = artifact
                                    .title
                                    .clone()
                                    .unwrap_or_else(|| anycode_core::artifact_title_for_path(path));
                                let _ = db
                                    .upsert_artifact(
                                        &project_id,
                                        &session_id,
                                        path,
                                        kind,
                                        &title,
                                    )
                                    .await;
                            }
                        }
                        publish_persisted(&db, &events, chat_evt, user_turn_id).await;
                    }
                }
                _ = flush_timer.tick() => {
                    if state.pending_delta_turn.is_some() {
                        state.flush_pending_delta(&db, &events, &session_id, &project_id, user_turn_id).await;
                    }
                }
            }
        }
        state
            .flush_pending_delta(&db, &events, &session_id, &project_id, user_turn_id)
            .await;
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
