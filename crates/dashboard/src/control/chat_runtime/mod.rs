//! In-process web chat turns via [`AgentRuntime::execute_turn_from_messages`].

pub mod bootstrap;
pub mod hydrate;

use crate::control::chat_live_bridge::{log_tail_fallback_enabled, spawn_live_bridge};
use crate::control::chat_runtime::hydrate::{load_prior_history, HydratedHistory};
use crate::control::vision_payload::{self, VisionImagePayload};
use crate::control::web_chat::WebChatSendResult;
use crate::control::web_chat_tail::WebChatTailHub;
use crate::db::DashboardDb;
use crate::events::EventBus;
use crate::recorder::{DashboardRecorder, RunSessionKind};
use anycode_agent::AgentRuntime;
use anycode_core::prelude::*;
use bootstrap::{build_embedded_runtime, embedded_chat_enabled, web_chat_log_dir};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use uuid::Uuid;

const EMBEDDED_INGEST_INTERVAL: Duration = Duration::from_secs(5);
/// Legacy cancel-IPC fallback poll. Direct cancel goes through
/// [`ChatRuntimeHost::cancel`]; this only covers out-of-process signals.
const EMBEDDED_CANCEL_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Reject overlapping embedded chat turns or sends while AskUserQuestion is pending.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatSendConflict {
    TurnInFlight,
    PendingQuestion,
}

impl std::fmt::Display for ChatSendConflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TurnInFlight => write!(f, "session turn already in flight"),
            Self::PendingQuestion => write!(f, "session has pending AskUserQuestion"),
        }
    }
}

impl std::error::Error for ChatSendConflict {}

fn embedded_loop_limits() -> anycode_core::AgentLoopLimits {
    let Ok((_, cfg)) = crate::config_patch::read_config_root() else {
        return anycode_core::resolve_agent_loop_limits(None, None);
    };
    let turns = cfg
        .get("runtime")
        .and_then(|r| r.get("max_agent_turns"))
        .and_then(|v| v.as_u64())
        .map(|n| n as usize);
    let tools = cfg
        .get("runtime")
        .and_then(|r| r.get("max_tool_calls"))
        .and_then(|v| v.as_u64())
        .map(|n| n as usize);
    anycode_core::resolve_agent_loop_limits(turns, tools)
}

fn poll_embedded_cancel(session_id: &str, coop: &AtomicBool) {
    if crate::cancel_ipc::poll_cancel_requested(session_id) {
        coop.store(true, Ordering::Release);
        crate::cancel_ipc::consume_cancel(session_id);
    }
}

#[derive(Clone)]
pub struct ChatRuntimeHost {
    sessions: Arc<Mutex<HashMap<String, Arc<EmbeddedSession>>>>,
    runtimes: Arc<Mutex<HashMap<String, Arc<AgentRuntime>>>>,
    runtime_generation: Arc<AtomicU64>,
    disk: Arc<DiskTaskOutput>,
}

struct EmbeddedSession {
    messages: Arc<Mutex<Vec<Message>>>,
    agent_type: AgentType,
    working_directory: String,
    task_id: TaskId,
    log_path: std::path::PathBuf,
    /// Monotonic id per user message in this session (SSE scope key).
    user_turn_seq: Arc<AtomicU32>,
    runtime_generation: AtomicU64,
    turn_in_flight: Arc<AtomicBool>,
    /// Cooperative-cancel flag of the currently running turn, if any.
    active_cancel: std::sync::Mutex<Option<Arc<AtomicBool>>>,
    /// Turn epoch: incremented when a new turn starts. A finished turn may
    /// only write session state while its epoch is still current.
    epoch: AtomicU64,
}

impl EmbeddedSession {
    /// Signal the active turn to cancel. Returns true when a turn was running.
    fn signal_cancel(&self) -> bool {
        let guard = self.active_cancel.lock().ok();
        if let Some(flag) = guard.as_ref().and_then(|g| g.as_ref()) {
            flag.store(true, Ordering::Release);
            return self.turn_in_flight.load(Ordering::Acquire);
        }
        false
    }
}

impl ChatRuntimeHost {
    #[must_use]
    pub fn enabled() -> bool {
        embedded_chat_enabled()
    }

    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            runtimes: Arc::new(Mutex::new(HashMap::new())),
            runtime_generation: Arc::new(AtomicU64::new(1)),
            disk: Arc::new(DiskTaskOutput::new(web_chat_log_dir())),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn send(
        &self,
        db: DashboardDb,
        events: Arc<EventBus>,
        tail_hub: &WebChatTailHub,
        session_id: &str,
        project_id: &str,
        project_root: &Path,
        agent: Option<&str>,
        prompt: &str,
        vision_images: Option<&[VisionImagePayload]>,
        reply_lang: Option<&str>,
        drain: Option<crate::control::message_queue::QueueDrainContext>,
    ) -> anyhow::Result<WebChatSendResult> {
        if !crate::question_ipc::list_pending_for_session(Some(session_id), 1).is_empty() {
            return Err(ChatSendConflict::PendingQuestion.into());
        }

        let root = crate::project_root::ensure_project_root(project_root, false)?;
        let agent_type =
            AgentType::new(crate::control::agent_resolve::resolve_web_chat_agent(agent));
        let working_directory = root.to_string_lossy().to_string();

        let runtime = self.runtime(&root).await?;
        let runtime_generation = self.runtime_generation.load(Ordering::Acquire);
        let session = {
            let existing = {
                let guard = self.sessions.lock().await;
                guard.get(session_id).cloned()
            };
            if let Some(existing) = existing {
                existing
            } else {
                // Rebuild outside the map lock: system prompt + DB transcript hydrate.
                let mut base = runtime
                    .build_session_messages(&agent_type, &working_directory)
                    .await?;
                let hydrated = match load_prior_history(&db, session_id).await {
                    Ok(h) => h,
                    Err(error) => {
                        tracing::warn!(
                            %error,
                            session_id,
                            "failed to hydrate chat history; continuing without prior turns"
                        );
                        HydratedHistory::default()
                    }
                };
                if !hydrated.messages.is_empty() {
                    tracing::info!(
                        session_id,
                        prior_messages = hydrated.messages.len(),
                        max_user_turn_id = hydrated.max_user_turn_id,
                        "hydrated embedded chat history from chat_turn_events"
                    );
                }
                base.extend(hydrated.messages);
                let task_id = TaskId::from(Uuid::new_v4());
                let log_path = self.disk.output_path(task_id);
                let embedded = Arc::new(EmbeddedSession {
                    messages: Arc::new(Mutex::new(base)),
                    agent_type: agent_type.clone(),
                    working_directory,
                    task_id,
                    log_path: log_path.clone(),
                    user_turn_seq: Arc::new(AtomicU32::new(hydrated.max_user_turn_id)),
                    runtime_generation: AtomicU64::new(runtime_generation),
                    turn_in_flight: Arc::new(AtomicBool::new(false)),
                    active_cancel: std::sync::Mutex::new(None),
                    epoch: AtomicU64::new(0),
                });
                let mut guard = self.sessions.lock().await;
                if let Some(existing) = guard.get(session_id) {
                    existing.clone()
                } else {
                    guard.insert(session_id.to_string(), Arc::clone(&embedded));
                    embedded
                }
            }
        };

        if session.runtime_generation.load(Ordering::Acquire) != runtime_generation {
            let mut refreshed = runtime
                .build_session_messages(&session.agent_type, &session.working_directory)
                .await?;
            let mut messages = session.messages.lock().await;
            refreshed.extend(
                messages
                    .iter()
                    .filter(|message| !matches!(&message.role, MessageRole::System))
                    .cloned(),
            );
            *messages = refreshed;
            session
                .runtime_generation
                .store(runtime_generation, Ordering::Release);
        }

        if session
            .turn_in_flight
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(ChatSendConflict::TurnInFlight.into());
        }

        let user_turn_id = session.user_turn_seq.fetch_add(1, Ordering::Relaxed) + 1;
        let turn_epoch = session.epoch.fetch_add(1, Ordering::AcqRel) + 1;
        let coop = Arc::new(AtomicBool::new(false));
        if let Ok(mut guard) = session.active_cancel.lock() {
            *guard = Some(Arc::clone(&coop));
        }

        let user_evt = crate::observability::chat_turn_log::user_message_event(
            session_id,
            project_id,
            user_turn_id,
            prompt.trim(),
        );
        match crate::observability::chat_turn_log::persist_and_enrich(&db, user_evt, user_turn_id)
            .await
        {
            Ok(enriched) => events.publish_chat(enriched),
            Err(error) => tracing::warn!(%error, "user message canonical persist failed"),
        }

        if log_tail_fallback_enabled() {
            tail_hub.ensure_tail(
                Arc::clone(&events),
                session_id,
                project_id,
                &session.log_path,
            );
        }

        let (live_tx, live_rx) = mpsc::unbounded_channel();
        spawn_live_bridge(
            Arc::clone(&events),
            db.clone(),
            session_id.to_string(),
            project_id.to_string(),
            user_turn_id,
            live_rx,
        );

        let prompt = prompt.trim().to_string();
        let vision_images = vision_images
            .map(vision_payload::to_core_images)
            .unwrap_or_default();
        let reply_lang_owned = reply_lang.map(str::to_string);
        let db2 = db.clone();
        let events2 = Arc::clone(&events);
        let session_id_owned = session_id.to_string();
        let project_id_owned = project_id.to_string();
        let runtime2 = Arc::clone(&runtime);
        let disk = Arc::clone(&self.disk);
        let session2 = Arc::clone(&session);
        let live_tx2 = live_tx;
        let turn_guard = Arc::clone(&session.turn_in_flight);
        tokio::spawn(async move {
            let _clear_turn = TurnInFlightGuard(turn_guard);
            if let Err(e) = run_embedded_turn(
                runtime2,
                disk,
                db2,
                events2,
                session2,
                session_id_owned.clone(),
                project_id_owned,
                prompt,
                vision_images,
                live_tx2,
                reply_lang_owned,
                user_turn_id,
                coop,
                turn_epoch,
            )
            .await
            {
                tracing::warn!(error = %e, "embedded chat turn failed");
            }
            if let Some(ctx) = drain {
                ctx.spawn_drain(session_id_owned);
            }
        });

        Ok(WebChatSendResult {
            session_id: session_id.to_string(),
            pid: std::process::id(),
            log_path: session.log_path.display().to_string(),
            started_at: chrono::Utc::now().to_rfc3339(),
            queued: true,
        })
    }

    pub async fn evict(&self, session_id: &str) {
        self.sessions.lock().await.remove(session_id);
    }

    /// Signal the active turn of a session to cancel immediately (no IPC
    /// polling latency). The session object is kept alive so the running turn
    /// can unwind; its epoch guards against stale state writes.
    pub async fn cancel(&self, session_id: &str) -> bool {
        let sessions = self.sessions.lock().await;
        sessions
            .get(session_id)
            .map(|s| s.signal_cancel())
            .unwrap_or(false)
    }

    pub async fn is_turn_in_flight(&self, session_id: &str) -> bool {
        self.sessions
            .lock()
            .await
            .get(session_id)
            .is_some_and(|s| s.turn_in_flight.load(Ordering::Acquire))
    }

    /// Rebuild the embedded runtime on the next turn so rescanned skills and governance apply.
    pub async fn invalidate_runtime(&self) {
        self.runtimes.lock().await.clear();
        self.runtime_generation.fetch_add(1, Ordering::AcqRel);
    }

    async fn runtime(&self, project_root: &Path) -> anyhow::Result<Arc<AgentRuntime>> {
        let key = project_root.to_string_lossy().to_string();
        let mut guard = self.runtimes.lock().await;
        if let Some(rt) = guard.get(&key) {
            return Ok(Arc::clone(rt));
        }
        let rt = build_embedded_runtime(Some((*self.disk).clone()), project_root).await?;
        guard.insert(key, Arc::clone(&rt));
        Ok(rt)
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_embedded_turn(
    runtime: Arc<AgentRuntime>,
    disk: Arc<DiskTaskOutput>,
    db: DashboardDb,
    events: Arc<EventBus>,
    session: Arc<EmbeddedSession>,
    session_id: String,
    project_id: String,
    prompt: String,
    user_vision_images: Vec<anycode_core::VisionImage>,
    live_trace_tx: mpsc::UnboundedSender<LiveTraceEvent>,
    reply_lang: Option<String>,
    user_turn_id: u32,
    coop: Arc<AtomicBool>,
    turn_epoch: u64,
) -> anyhow::Result<()> {
    let reply_lang = normalize_reply_lang(reply_lang.as_deref());
    let start_intent = crate::control::start_server_intent::is_start_server_intent(&prompt);
    let preview_status = if start_intent {
        Some(
            crate::control::start_server_intent::ensure_local_preview_server(
                &session.working_directory,
            )
            .await,
        )
    } else {
        None
    };
    let host_intent_hint = if start_intent {
        Some(crate::control::start_server_intent::start_server_host_hint(
            &session.working_directory,
            preview_status.as_ref(),
        ))
    } else {
        None
    };
    let preview_ok = preview_status.as_ref().is_some_and(|s| s.ok());
    // Appendix only when the agent still has to start/verify (host ensure failed).
    let agent_prompt = if start_intent && !preview_ok {
        crate::control::start_server_intent::with_start_server_user_appendix(
            &prompt,
            &session.working_directory,
            preview_status.as_ref(),
        )
    } else {
        prompt.clone()
    };
    let chat_turn = anycode_core::ChatTurnContext {
        dashboard_session_id: Some(session_id.clone()),
        user_turn_id: Some(user_turn_id),
        reply_language: Some(reply_lang.clone()),
        host_intent_hint,
    };
    // Task-local scope replaces the old process-env plumbing
    // (ANYCODE_REPLY_LANG / SESSION_ENV / USER_TURN_ENV / dashboard DB vars):
    // recorder, approval and question callbacks read it via anycode_core.
    anycode_core::scope_chat_turn(
        chat_turn,
        run_embedded_turn_scoped(
            runtime,
            disk,
            db,
            events,
            session,
            session_id,
            project_id,
            agent_prompt,
            user_vision_images,
            live_trace_tx,
            reply_lang,
            coop,
            turn_epoch,
            user_turn_id,
            preview_status,
        ),
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn run_embedded_turn_scoped(
    runtime: Arc<AgentRuntime>,
    disk: Arc<DiskTaskOutput>,
    db: DashboardDb,
    events: Arc<EventBus>,
    session: Arc<EmbeddedSession>,
    session_id: String,
    project_id: String,
    prompt: String,
    user_vision_images: Vec<anycode_core::VisionImage>,
    live_trace_tx: mpsc::UnboundedSender<LiveTraceEvent>,
    reply_lang: String,
    coop: Arc<AtomicBool>,
    turn_epoch: u64,
    user_turn_id: u32,
    preview_status: Option<crate::control::start_server_intent::PreviewServerStatus>,
) -> anyhow::Result<()> {
    refresh_embedded_system_message(&runtime, &session, &reply_lang).await?;
    crate::notify::register_inprocess_bus(Arc::clone(&events));

    // Host already started the preview — skip the agent loop so weak models
    // cannot mark the session failed after Glob/FileRead-only exploration.
    if let Some(st) = preview_status.as_ref().filter(|s| s.ok()) {
        return finish_host_preview_turn(
            disk,
            db,
            events,
            session,
            session_id,
            project_id,
            prompt,
            user_vision_images,
            live_trace_tx,
            turn_epoch,
            user_turn_id,
            st,
        )
        .await;
    }

    let tool_deny_names = if preview_status.is_some() {
        crate::control::start_server_intent::start_server_tool_deny_names()
    } else {
        vec![]
    };

    let task = Task {
        id: session.task_id,
        agent_type: session.agent_type.clone(),
        prompt: prompt.clone(),
        context: TaskContext {
            session_id: Uuid::new_v4(),
            working_directory: session.working_directory.clone(),
            environment: Default::default(),
            user_id: None,
            system_prompt_append: None,
            context_injections: vec![],
            nested_model_override: None,
            nested_worktree_path: None,
            nested_worktree_repo_root: None,
            nested_cancel: None,
            channel_progress_tx: None,
            live_trace_tx: Some(live_trace_tx.clone()),
            tool_deny_names: tool_deny_names.clone(),
            tool_deny_prefixes: vec![],
            user_vision_images: user_vision_images.clone(),
            budget: TaskBudget::default(),
            loop_limits: embedded_loop_limits(),
            chat_turn: anycode_core::current_chat_turn(),
        },
        created_at: chrono::Utc::now(),
    };

    let recorder_db = Arc::new(db);
    let mut recorder = DashboardRecorder::begin(
        Arc::clone(&recorder_db),
        RunSessionKind::Repl,
        &task,
        &truncate(&prompt, 80),
    )
    .await?;
    recorder.ingest_delta(&disk, session.task_id).await;

    {
        let mut user_metadata = std::collections::HashMap::new();
        if !user_vision_images.is_empty() {
            anycode_core::attach_vision_images(&mut user_metadata, &user_vision_images);
        }
        let mut msgs = session.messages.lock().await;
        msgs.push(Message {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: MessageContent::Text(prompt),
            timestamp: chrono::Utc::now(),
            metadata: user_metadata,
        });
    }

    let session_id_cancel = session_id.clone();
    let exec = runtime.execute_turn_from_messages(
        session.task_id,
        &session.agent_type,
        Arc::clone(&session.messages),
        &session.working_directory,
        Some(Arc::clone(&coop)),
        &tool_deny_names,
        &[],
        TaskBudget::default(),
        embedded_loop_limits(),
        Some(live_trace_tx),
    );
    tokio::pin!(exec);

    let mut cancel_poll = tokio::time::interval(EMBEDDED_CANCEL_POLL_INTERVAL);
    cancel_poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut ingest_tick = tokio::time::interval(EMBEDDED_INGEST_INTERVAL);
    ingest_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let result = loop {
        tokio::select! {
            res = &mut exec => break res,
            _ = cancel_poll.tick() => {
                poll_embedded_cancel(&session_id_cancel, &coop);
            }
            _ = ingest_tick.tick() => {
                recorder.ingest_delta(&disk, session.task_id).await;
            }
        }
    };

    // Stale-turn guard: only the active epoch may persist terminal session
    // state or publish error events. A newer turn (started after cancel)
    // owns the session now; the DB terminal-status guard is the second line
    // of defense.
    let epoch_current = session.epoch.load(Ordering::Acquire) == turn_epoch;
    if !epoch_current {
        tracing::info!(
            target: "anycode_dashboard",
            session_id = %session_id,
            turn_epoch,
            "stale embedded turn finished; skipping session state writes"
        );
        return Ok(());
    }

    recorder.scan_workspace_artifacts().await;
    recorder.ingest_full_log(&disk, session.task_id).await;
    let terminal_status = match &result {
        Ok(out) => {
            let status = crate::control::session_status::session_status_for_termination(
                out.termination_reason,
            );
            recorder
                .finish_with_status(
                    status,
                    Some(out.final_text.trim()).filter(|s| !s.is_empty()),
                )
                .await;
            status
        }
        Err(e) if e.is_cooperative_cancel() => {
            recorder
                .finish_with_status(crate::control::session_status::STATUS_CANCELLED, None)
                .await;
            crate::control::session_status::STATUS_CANCELLED
        }
        Err(e) => {
            recorder
                .finish_with_status(
                    crate::control::session_status::STATUS_FAILED,
                    Some(&e.to_string()),
                )
                .await;
            crate::control::session_status::STATUS_FAILED
        }
    };
    publish_embedded_turn_done(
        &events,
        &session_id,
        &project_id,
        user_turn_id,
        terminal_status,
    );

    if let Err(e) = &result {
        if e.is_cooperative_cancel() {
            return Ok(());
        }
        if let Ok(evt) = recorder_db
            .insert_event(crate::schema::InsertEventRequest {
                project_id: project_id.clone(),
                session_id: Some(session_id.clone()),
                task_id: None,
                agent_id: None,
                event_type: "session_error".into(),
                severity: Some("error".into()),
                title: "Embedded chat turn failed".into(),
                body: Some(e.to_string()),
                payload: None,
            })
            .await
        {
            crate::control::web_chat_tail::publish_project_chat_event(&events, &evt);
        }
    }

    Ok(())
}

/// Host-started preview: write assistant reply + completed status without the agent loop.
#[allow(clippy::too_many_arguments)]
async fn finish_host_preview_turn(
    disk: Arc<DiskTaskOutput>,
    db: DashboardDb,
    events: Arc<EventBus>,
    session: Arc<EmbeddedSession>,
    session_id: String,
    project_id: String,
    prompt: String,
    user_vision_images: Vec<anycode_core::VisionImage>,
    live_trace_tx: mpsc::UnboundedSender<LiveTraceEvent>,
    turn_epoch: u64,
    user_turn_id: u32,
    status: &crate::control::start_server_intent::PreviewServerStatus,
) -> anyhow::Result<()> {
    let reply = status.user_reply_zh();
    let task = Task {
        id: session.task_id,
        agent_type: session.agent_type.clone(),
        prompt: prompt.clone(),
        context: TaskContext {
            session_id: Uuid::new_v4(),
            working_directory: session.working_directory.clone(),
            environment: Default::default(),
            user_id: None,
            system_prompt_append: None,
            context_injections: vec![],
            nested_model_override: None,
            nested_worktree_path: None,
            nested_worktree_repo_root: None,
            nested_cancel: None,
            channel_progress_tx: None,
            live_trace_tx: Some(live_trace_tx.clone()),
            tool_deny_names: vec![],
            tool_deny_prefixes: vec![],
            user_vision_images: user_vision_images.clone(),
            budget: TaskBudget::default(),
            loop_limits: embedded_loop_limits(),
            chat_turn: anycode_core::current_chat_turn(),
        },
        created_at: chrono::Utc::now(),
    };

    let recorder_db = Arc::new(db);
    let mut recorder = DashboardRecorder::begin(
        Arc::clone(&recorder_db),
        RunSessionKind::Repl,
        &task,
        &truncate(&prompt, 80),
    )
    .await?;
    recorder.ingest_delta(&disk, session.task_id).await;

    {
        let mut user_metadata = std::collections::HashMap::new();
        if !user_vision_images.is_empty() {
            anycode_core::attach_vision_images(&mut user_metadata, &user_vision_images);
        }
        let mut msgs = session.messages.lock().await;
        msgs.push(Message {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: MessageContent::Text(prompt),
            timestamp: chrono::Utc::now(),
            metadata: user_metadata,
        });
        msgs.push(Message {
            id: Uuid::new_v4(),
            role: MessageRole::Assistant,
            content: MessageContent::Text(reply.clone()),
            timestamp: chrono::Utc::now(),
            metadata: Default::default(),
        });
    }

    let _ = live_trace_tx.send(LiveTraceEvent::TurnStart { turn: 1 });
    let _ = live_trace_tx.send(LiveTraceEvent::AssistantDelta {
        turn: 1,
        delta: reply.clone(),
        narration: false,
    });
    let _ = live_trace_tx.send(LiveTraceEvent::AssistantDone {
        turn: 1,
        text: reply.clone(),
    });
    let _ = live_trace_tx.send(LiveTraceEvent::TurnDone {
        status: crate::control::session_status::STATUS_COMPLETED.to_string(),
    });

    // Give the live bridge a moment to persist assistant_delta before we finalize.
    tokio::time::sleep(Duration::from_millis(50)).await;

    let epoch_current = session.epoch.load(Ordering::Acquire) == turn_epoch;
    if !epoch_current {
        tracing::info!(
            target: "anycode_dashboard",
            session_id = %session_id,
            turn_epoch,
            "stale host-preview turn finished; skipping session state writes"
        );
        return Ok(());
    }

    recorder.scan_workspace_artifacts().await;
    recorder.ingest_full_log(&disk, session.task_id).await;
    recorder
        .finish_with_status(
            crate::control::session_status::STATUS_COMPLETED,
            Some(reply.as_str()),
        )
        .await;
    publish_embedded_turn_done(
        &events,
        &session_id,
        &project_id,
        user_turn_id,
        crate::control::session_status::STATUS_COMPLETED,
    );
    tracing::info!(
        target: "anycode_dashboard",
        session_id = %session_id,
        url = %status.url,
        already_running = status.already_running,
        "host preview server ensured; skipped agent loop"
    );
    Ok(())
}

/// Belt-and-suspenders: ensure session SSE always receives `turn_done` after the
/// DB session row is finalized (live trace may have already emitted one).
fn publish_embedded_turn_done(
    events: &EventBus,
    session_id: &str,
    project_id: &str,
    user_turn_id: u32,
    status: &str,
) {
    use crate::schema::ChatStreamEvent;
    use serde_json::json;
    events.publish_chat(ChatStreamEvent {
        session_id: session_id.to_string(),
        project_id: project_id.to_string(),
        kind: "turn_done".into(),
        turn: None,
        conversation_turn_id: Some(user_turn_id),
        seq: None,
        event_id: None,
        tool_key: None,
        tool_name: None,
        text: Some(status.to_string()),
        block: None,
        payload: json!({
            "status": status,
            "user_turn_id": user_turn_id,
            "source": "embedded_finish",
        }),
        at: chrono::Utc::now().to_rfc3339(),
    });
}

fn normalize_reply_lang(lang: Option<&str>) -> String {
    match lang.map(str::trim).filter(|s| !s.is_empty()) {
        Some(l) if l.to_ascii_lowercase().starts_with("zh") => "zh".to_string(),
        Some(l) if l.to_ascii_lowercase().starts_with("en") => "en".to_string(),
        _ => std::env::var("ANYCODE_REPLY_LANG").unwrap_or_else(|_| "zh".to_string()),
    }
}

struct TurnInFlightGuard(Arc<AtomicBool>);

impl Drop for TurnInFlightGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

async fn refresh_embedded_system_message(
    runtime: &AgentRuntime,
    session: &EmbeddedSession,
    _reply_lang: &str,
) -> anyhow::Result<()> {
    let fresh_system = runtime
        .build_system_message(&session.agent_type, &session.working_directory)
        .await?;
    let mut messages = session.messages.lock().await;
    if let Some(first) = messages.first_mut() {
        if matches!(first.role, MessageRole::System) {
            *first = fresh_system;
            return Ok(());
        }
    }
    messages.insert(0, fresh_system);
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    s.chars().take(max).collect::<String>() + "…"
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;
    use tempfile::tempdir;

    fn test_session() -> EmbeddedSession {
        EmbeddedSession {
            messages: Arc::new(Mutex::new(vec![])),
            agent_type: AgentType::new("general-purpose"),
            working_directory: ".".into(),
            task_id: TaskId::from(Uuid::new_v4()),
            log_path: std::path::PathBuf::from("/tmp/embedded-test.log"),
            user_turn_seq: Arc::new(AtomicU32::new(0)),
            runtime_generation: AtomicU64::new(1),
            turn_in_flight: Arc::new(AtomicBool::new(false)),
            active_cancel: std::sync::Mutex::new(None),
            epoch: AtomicU64::new(0),
        }
    }

    #[test]
    fn signal_cancel_sets_active_turn_flag() {
        let session = test_session();
        // No active turn yet: nothing to cancel.
        assert!(!session.signal_cancel());

        let coop = Arc::new(AtomicBool::new(false));
        *session.active_cancel.lock().unwrap() = Some(Arc::clone(&coop));
        session.turn_in_flight.store(true, Ordering::Release);
        assert!(session.signal_cancel());
        assert!(coop.load(Ordering::Acquire));
    }

    #[test]
    fn stale_epoch_is_detected_after_new_turn_starts() {
        let session = test_session();
        let old_epoch = session.epoch.fetch_add(1, Ordering::AcqRel) + 1;
        assert_eq!(session.epoch.load(Ordering::Acquire), old_epoch);
        // A new turn bumps the epoch; the old turn must observe the mismatch.
        let new_epoch = session.epoch.fetch_add(1, Ordering::AcqRel) + 1;
        assert_ne!(session.epoch.load(Ordering::Acquire), old_epoch);
        assert_eq!(session.epoch.load(Ordering::Acquire), new_epoch);
    }

    #[test]
    fn embedded_cancel_poll_sets_coop_flag() {
        let dir = tempdir().unwrap();
        std::env::set_var("ANYCODE_DASHBOARD_STATE_DIR", dir.path().join("dashboard"));
        let session_id = "sess_embedded_cancel";
        crate::cancel_ipc::register_active(session_id, "task_1").unwrap();
        crate::cancel_ipc::request_cancel(session_id).unwrap();

        let coop = AtomicBool::new(false);
        poll_embedded_cancel(session_id, &coop);
        assert!(coop.load(Ordering::Acquire));
        assert!(!crate::cancel_ipc::poll_cancel_requested(session_id));

        crate::cancel_ipc::unregister_active(session_id);
    }
}
