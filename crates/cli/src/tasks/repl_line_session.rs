//! 行式 REPL 多轮会话状态（与 TUI 共用 `tui-sessions` 快照与 `execute_turn_from_messages`）。

use super::tasks_sink::ReplSink;
use crate::artifact_summary::claude_turn_written_lines;
use crate::i18n::{tr, tr_args};
use crate::tui::tui_session_persist::{
    list_session_index_entries, load_tui_session, resolve_session_for_reopen, sessions_dir,
    workspace_paths_equal_for_session, TuiSessionSnapshot,
};
use anycode_agent::AgentRuntime;
use anycode_core::{
    AgentType, CoreError, Message, MessageContent, MessageRole, TurnOutput,
    NESTED_TASK_COOPERATIVE_CANCEL_ERROR,
};
use fluent_bundle::FluentArgs;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use uuid::Uuid;

pub(crate) struct ReplLineSession {
    pub messages: Arc<Mutex<Vec<Message>>>,
    pub session_file_id: Uuid,
    pub working_dir_str: String,
    pub model_persist: String,
    /// 与全屏 TUI / `execute_turn_from_messages` 一致：流式 REPL 在回合进行中可置位以协作结束本轮。
    pub turn_coop_cancel: Arc<AtomicBool>,
}

impl ReplLineSession {
    pub async fn bootstrap(
        runtime: &AgentRuntime,
        working_dir: &Path,
        agent: &str,
        resume: Option<Uuid>,
        model: &str,
    ) -> anyhow::Result<Self> {
        let working_dir_str = std::fs::canonicalize(working_dir)
            .unwrap_or_else(|_| working_dir.to_path_buf())
            .to_string_lossy()
            .to_string();
        let at = AgentType::new(agent.to_string());
        let turn_coop_cancel = Arc::new(AtomicBool::new(false));
        let (messages, session_file_id) = if let Some(id) = resume {
            match load_tui_session(id)? {
                Some(snap) => {
                    if !workspace_paths_equal_for_session(&snap.workspace_root, &working_dir_str) {
                        tracing::warn!("{}", crate::i18n::tr("tui-resume-cwd-warn"));
                    }
                    (Arc::new(Mutex::new(snap.messages)), snap.id)
                }
                None => anyhow::bail!("{}", tr("repl-resume-not-found")),
            }
        } else {
            let msgs = runtime
                .build_session_messages(&at, &working_dir_str)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            (Arc::new(Mutex::new(msgs)), Uuid::new_v4())
        };
        Ok(Self {
            messages,
            session_file_id,
            working_dir_str,
            model_persist: model.to_string(),
            turn_coop_cancel,
        })
    }

    pub async fn rebuild_for_agent(
        &mut self,
        runtime: &AgentRuntime,
        agent: &str,
    ) -> anyhow::Result<()> {
        let at = AgentType::new(agent.to_string());
        let msgs = runtime
            .build_session_messages(&at, &self.working_dir_str)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        *self.messages.lock().await = msgs;
        Ok(())
    }

    pub async fn apply_snapshot(&mut self, snap: TuiSessionSnapshot, agent_out: &mut String) {
        if !workspace_paths_equal_for_session(&snap.workspace_root, &self.working_dir_str) {
            tracing::warn!("{}", crate::i18n::tr("tui-resume-cwd-warn"));
        }
        self.session_file_id = snap.id;
        *agent_out = snap.agent.clone();
        *self.messages.lock().await = snap.messages;
        self.turn_coop_cancel.store(false, Ordering::Release);
    }
}

async fn pop_trailing_assistant_if_present(session: &ReplLineSession) {
    let mut g = session.messages.lock().await;
    if g.last().is_some_and(|m| m.role == MessageRole::Assistant) {
        g.pop();
    }
}

pub(crate) async fn run_line_repl_turn(
    runtime: &AgentRuntime,
    session: &ReplLineSession,
    agent: &str,
    prompt: &str,
    sink: &mut ReplSink,
) -> anyhow::Result<()> {
    let at = AgentType::new(agent.to_string());
    let user_msg = Message {
        id: Uuid::new_v4(),
        role: MessageRole::User,
        content: MessageContent::Text(prompt.to_string()),
        timestamp: chrono::Utc::now(),
        metadata: HashMap::new(),
    };
    {
        let mut g = session.messages.lock().await;
        g.push(user_msg);
    }
    session.turn_coop_cancel.store(false, Ordering::Release);
    let task_id = Uuid::new_v4();
    let msgs = session.messages.clone();
    let wd = session.working_dir_str.clone();
    let coop = session.turn_coop_cancel.clone();
    sink.eprint_line(tr("repl-task-run"));

    // 非 TTY stdio：无 ratatui 键盘路径时用 Ctrl+C 置位协作取消（与 TUI / stream REPL 一致）。
    let coop_sig = session.turn_coop_cancel.clone();
    let sig_watch = tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            coop_sig.store(true, Ordering::Release);
        }
    });

    let exec_res = runtime
        .execute_turn_from_messages(task_id, &at, msgs, &wd, Some(coop))
        .await;

    sig_watch.abort();
    let _ = sig_watch.await;

    let out = match exec_res {
        Ok(o) => o,
        Err(e) => {
            let is_coop = matches!(
                &e,
                CoreError::LLMError(s) if s.as_str() == NESTED_TASK_COOPERATIVE_CANCEL_ERROR
            );
            if is_coop {
                pop_trailing_assistant_if_present(session).await;
                let msg = tr("tui-turn-cooperative-cancelled");
                sink.eprint_line(&msg);
                sink.line("");
                sink.line(&msg);
                crate::tui::tui_session_persist::spawn_persist_tui_session(
                    session.session_file_id,
                    session.working_dir_str.clone(),
                    agent.to_string(),
                    session.model_persist.clone(),
                    session.messages.clone(),
                );
                return Ok(());
            }
            return Err(anyhow::anyhow!("{}", e));
        }
    };

    sink.eprint_line(tr("repl-task-ok"));
    sink.line("");
    sink.line(tr("repl-output-header"));
    sink.line(&out.final_text);
    let written = claude_turn_written_lines(&out.artifacts);
    if !written.is_empty() {
        sink.line("");
        sink.eprint_line(tr("repl-written-header"));
        for line in written {
            let mut wl = FluentArgs::new();
            wl.set("line", line);
            sink.eprint_line(tr_args("repl-written-line", &wl));
        }
    }
    crate::tui::tui_session_persist::spawn_persist_tui_session(
        session.session_file_id,
        session.working_dir_str.clone(),
        agent.to_string(),
        session.model_persist.clone(),
        session.messages.clone(),
    );
    Ok(())
}

/// 追加用户消息并 `spawn` 回合（与 TUI `append_user_line_and_spawn_turn` 对齐）。返回 `(handle, exec_prev_len)`。
pub(crate) async fn append_user_spawn_turn(
    runtime: &Arc<AgentRuntime>,
    session: &ReplLineSession,
    agent: &str,
    prompt: &str,
) -> anyhow::Result<(JoinHandle<anyhow::Result<TurnOutput>>, usize)> {
    let at = AgentType::new(agent.to_string());
    let user_msg = Message {
        id: Uuid::new_v4(),
        role: MessageRole::User,
        content: MessageContent::Text(prompt.to_string()),
        timestamp: chrono::Utc::now(),
        metadata: HashMap::new(),
    };
    let exec_prev_len = {
        let mut g = session.messages.lock().await;
        g.push(user_msg);
        g.len()
    };
    session.turn_coop_cancel.store(false, Ordering::Release);
    let task_id = Uuid::new_v4();
    let rt = Arc::clone(runtime);
    let msgs = session.messages.clone();
    let wd = session.working_dir_str.clone();
    let coop = session.turn_coop_cancel.clone();
    let handle = tokio::spawn(async move {
        rt.execute_turn_from_messages(task_id, &at, msgs, &wd, Some(coop))
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    });
    Ok((handle, exec_prev_len))
}

pub(crate) fn format_session_list_for_repl() -> String {
    let dir = sessions_dir();
    let mut rows = match list_session_index_entries(&dir) {
        Ok(r) => r,
        Err(e) => return format!("{} {e}", tr("repl-session-list-err")),
    };
    if rows.is_empty() {
        return tr("repl-session-list-empty");
    }
    #[allow(clippy::unnecessary_sort_by)]
    rows.sort_by(|a, b| b.mtime.cmp(&a.mtime));
    let mut s = tr("repl-session-list-title");
    s.push('\n');
    for e in rows.iter().take(40) {
        s.push_str(&format!(
            "  {}  {}  {}  {}\n",
            e.id, e.workspace_root, e.agent, e.model
        ));
    }
    s
}

/// `arg`: `None` = cwd 优先 + 全局回退；`Some(uuid)` = 显式 id。
pub(crate) fn load_repl_session_choice(
    arg: Option<String>,
    working_dir_str: &str,
) -> anyhow::Result<TuiSessionSnapshot> {
    match arg.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        None => {
            let id = resolve_session_for_reopen(working_dir_str)?;
            load_tui_session(id)?.ok_or_else(|| anyhow::anyhow!("{}", tr("repl-resume-not-found")))
        }
        Some(rest) => {
            let id = Uuid::parse_str(rest)
                .map_err(|_| anyhow::anyhow!("{}", tr("repl-session-bad-uuid")))?;
            load_tui_session(id)?.ok_or_else(|| anyhow::anyhow!("{}", tr("repl-resume-not-found")))
        }
    }
}
