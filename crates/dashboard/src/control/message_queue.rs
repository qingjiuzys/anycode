//! Persistent session message queue: enqueue while a turn is in flight, drain when idle.

use crate::api::state::AppState;
use crate::control::agent_resolve::resolve_web_chat_agent;
use crate::db::QueuedMessagePop;
use std::path::Path;
use std::sync::Arc;

pub async fn session_accepts_enqueue(
    state: &AppState,
    session_id: &str,
    session_status: &str,
) -> bool {
    if crate::control::chat_runtime::ChatRuntimeHost::enabled() {
        return state.chat_runtime.is_turn_in_flight(session_id).await;
    }
    session_status == "running"
}

/// Drain stale pending items when no turn is active (legacy queue / race recovery).
pub fn spawn_drain_if_idle(state: &AppState, session_id: &str) {
    let state = Arc::new(state.clone());
    let session_id = session_id.to_string();
    tokio::spawn(async move {
        if state.chat_runtime.is_turn_in_flight(&session_id).await {
            return;
        }
        drain_session_message_queue(&state, &session_id).await;
    });
}

pub async fn drain_session_message_queue(state: &AppState, session_id: &str) {
    loop {
        if !crate::question_ipc::list_pending_for_session(Some(session_id), 1).is_empty() {
            return;
        }
        if state.chat_runtime.is_turn_in_flight(session_id).await {
            return;
        }
        let session = match state.db.get_session(session_id).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if session.status == "running" && state.chat_runtime.is_turn_in_flight(session_id).await {
            return;
        }
        let project = match state.db.get_project(&session.project_id).await {
            Ok(Some(p)) => p,
            _ => return,
        };
        let root_path = std::path::PathBuf::from(&project.root_path);
        let root = match crate::project_root::ensure_project_root(&root_path, false) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(%e, session_id, "message queue drain: bad project root");
                return;
            }
        };

        let next = match state.db.pop_next_pending_queue_message(session_id).await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(%e, session_id, "message queue pop failed");
                return;
            }
        };
        let Some(item) = next else {
            return;
        };

        if let Err(e) =
            dispatch_queued_item(state, &session.project_id, session_id, &root, item).await
        {
            tracing::warn!(%e, session_id, "message queue dispatch failed");
            return;
        }
    }
}

async fn dispatch_queued_item(
    state: &AppState,
    project_id: &str,
    session_id: &str,
    root: &Path,
    item: QueuedMessagePop,
) -> anyhow::Result<()> {
    let prompt = item.prompt.trim();
    let prompt_for_chat = crate::task_trigger::prompt_with_skills(prompt, item.skills.as_deref());
    let agent_raw = item.agent.as_deref();
    let effective_agent = resolve_web_chat_agent(agent_raw);

    if let Ok(evt) = state
        .db
        .insert_event(crate::schema::InsertEventRequest {
            project_id: project_id.to_string(),
            session_id: Some(session_id.to_string()),
            task_id: None,
            agent_id: None,
            event_type: "message_dequeued".into(),
            severity: Some("info".into()),
            title: "Queued message dispatched".into(),
            body: Some(prompt.chars().take(8000).collect()),
            payload: Some(serde_json::json!({
                "queue_id": item.id,
                "seq": item.seq,
                "source": "message_queue",
            })),
        })
        .await
    {
        crate::control::web_chat_tail::publish_project_chat_event(&state.events, &evt);
    }

    match crate::control::web_chat_dispatch::dispatch_web_chat_prompt(
        state,
        project_id,
        session_id,
        root,
        Some(effective_agent.as_str()),
        prompt,
        &prompt_for_chat,
        item.vision_images.as_deref(),
        item.text_files.as_deref(),
        item.lang.as_deref(),
        false,
        "conversation_message_queued",
        item.composer_mode.as_deref(),
    )
    .await
    {
        Ok(_) => Ok(()),
        Err((status, message)) => {
            let _ = state.db.mark_queue_message_failed(&item.id, &message).await;
            Err(anyhow::anyhow!("dispatch failed ({status}): {message}"))
        }
    }
}

#[derive(Clone)]
pub struct QueueDrainContext {
    state: Arc<AppState>,
}

impl QueueDrainContext {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    pub fn spawn_drain(&self, session_id: String) {
        let state = Arc::clone(&self.state);
        tokio::spawn(async move {
            drain_session_message_queue(&state, &session_id).await;
        });
    }
}
