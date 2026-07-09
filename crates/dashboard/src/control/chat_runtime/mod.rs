//! In-process web chat turns via [`AgentRuntime::execute_turn_from_messages`].

pub mod bootstrap;

use crate::control::chat_live_bridge::{log_tail_fallback_enabled, spawn_live_bridge};
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
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use uuid::Uuid;

const EMBEDDED_INGEST_INTERVAL: Duration = Duration::from_secs(5);

fn poll_embedded_cancel(session_id: &str, coop: &AtomicBool) {
    if crate::cancel_ipc::poll_cancel_requested(session_id) {
        coop.store(true, Ordering::Release);
        crate::cancel_ipc::consume_cancel(session_id);
    }
}

#[derive(Clone)]
pub struct ChatRuntimeHost {
    sessions: Arc<Mutex<HashMap<String, Arc<EmbeddedSession>>>>,
    runtime: Arc<Mutex<Option<Arc<AgentRuntime>>>>,
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
}

impl ChatRuntimeHost {
    #[must_use]
    pub fn enabled() -> bool {
        embedded_chat_enabled()
    }

    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            runtime: Arc::new(Mutex::new(None)),
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
    ) -> anyhow::Result<WebChatSendResult> {
        let root = crate::project_root::ensure_project_root(project_root, false)?;
        let agent_type = AgentType::new(
            agent
                .map(str::trim)
                .filter(|a| !a.is_empty())
                .unwrap_or("workspace-assistant")
                .to_string(),
        );
        let working_directory = root.to_string_lossy().to_string();

        let runtime = self.runtime().await?;
        let session = {
            let mut guard = self.sessions.lock().await;
            if let Some(existing) = guard.get(session_id) {
                existing.clone()
            } else {
                let task_id = TaskId::from(Uuid::new_v4());
                let log_path = self.disk.output_path(task_id);
                let messages = Arc::new(Mutex::new(
                    runtime
                        .build_session_messages(&agent_type, &working_directory)
                        .await?,
                ));
                let embedded = Arc::new(EmbeddedSession {
                    messages,
                    agent_type: agent_type.clone(),
                    working_directory,
                    task_id,
                    log_path: log_path.clone(),
                    user_turn_seq: Arc::new(AtomicU32::new(0)),
                });
                guard.insert(session_id.to_string(), Arc::clone(&embedded));
                embedded
            }
        };

        let user_turn_id = session.user_turn_seq.fetch_add(1, Ordering::Relaxed) + 1;

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
        let db2 = db.clone();
        let events2 = Arc::clone(&events);
        let session_id_owned = session_id.to_string();
        let project_id_owned = project_id.to_string();
        let runtime2 = Arc::clone(&runtime);
        let disk = Arc::clone(&self.disk);
        let session2 = Arc::clone(&session);
        let live_tx2 = live_tx;
        tokio::spawn(async move {
            if let Err(e) = run_embedded_turn(
                runtime2,
                disk,
                db2,
                events2,
                session2,
                session_id_owned,
                project_id_owned,
                prompt,
                live_tx2,
            )
            .await
            {
                tracing::warn!(error = %e, "embedded chat turn failed");
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

    async fn runtime(&self) -> anyhow::Result<Arc<AgentRuntime>> {
        let mut guard = self.runtime.lock().await;
        if let Some(rt) = guard.as_ref() {
            return Ok(Arc::clone(rt));
        }
        let rt = build_embedded_runtime(Some((*self.disk).clone())).await?;
        *guard = Some(Arc::clone(&rt));
        Ok(rt)
    }
}

async fn run_embedded_turn(
    runtime: Arc<AgentRuntime>,
    disk: Arc<DiskTaskOutput>,
    db: DashboardDb,
    events: Arc<EventBus>,
    session: Arc<EmbeddedSession>,
    session_id: String,
    project_id: String,
    prompt: String,
    live_trace_tx: mpsc::UnboundedSender<LiveTraceEvent>,
) -> anyhow::Result<()> {
    std::env::set_var("ANYCODE_DASHBOARD_RECORD", "1");
    std::env::set_var("ANYCODE_DASHBOARD_INPROCESS_EVENTS", "1");
    std::env::set_var("ANYCODE_DASHBOARD_DB", db.path());
    std::env::set_var(crate::ipc::approval_ipc::SESSION_ENV, &session_id);
    crate::notify::register_inprocess_bus(Arc::clone(&events));

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
            user_vision_images: vec![],
            budget: TaskBudget::default(),
            loop_limits: anycode_core::resolve_agent_loop_limits(None, None),
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
        let mut msgs = session.messages.lock().await;
        msgs.push(Message {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: MessageContent::Text(prompt),
            timestamp: chrono::Utc::now(),
            metadata: Default::default(),
        });
    }

    let coop = Arc::new(AtomicBool::new(false));
    let session_id_cancel = session_id.clone();
    let exec = runtime.execute_turn_from_messages(
        session.task_id,
        &session.agent_type,
        Arc::clone(&session.messages),
        &session.working_directory,
        Some(Arc::clone(&coop)),
        &[],
        &[],
        TaskBudget::default(),
        anycode_core::resolve_agent_loop_limits(None, None),
        Some(live_trace_tx),
    );
    tokio::pin!(exec);

    let result = loop {
        tokio::select! {
            res = &mut exec => break res,
            _ = tokio::time::sleep(EMBEDDED_INGEST_INTERVAL) => {
                poll_embedded_cancel(&session_id_cancel, &coop);
                recorder.ingest_delta(&disk, session.task_id).await;
            }
        }
    };

    recorder.scan_workspace_artifacts().await;
    recorder.ingest_full_log(&disk, session.task_id).await;
    recorder.finish_run(&disk, session.task_id, None).await;

    if let Err(e) = result {
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
