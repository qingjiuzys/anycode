//! 交互式 REPL（stdio / 全屏 TUI 行编辑）与斜杠分发。

use std::io::{IsTerminal, Write};

use crate::app_config::{
    apply_optional_repl_model, security_wants_interactive_approval_callback, Config,
};
use crate::bootstrap::initialize_runtime;
use crate::builtin_agents::parse_agent_slash_command;
use crate::i18n::{tr, tr_args};
use crate::repl_banner::{self, ReplWelcomeKind};
use crate::slash_commands::{self, ParsedSlashCommand};
use crate::tui::transcript::build_stream_turn_plain;
use crate::tui::{ApprovalDecision, PendingApproval, PendingUserQuestion, TuiApprovalCallback};
use crate::workspace;
use anycode_agent::AgentRuntime;
use anycode_core::prelude::*;
use anycode_tools::{iter_cli_tool_help, workflows};
use fluent_bundle::FluentArgs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use uuid::Uuid;

use super::repl_line_session::{self, ReplLineSession};
use super::tasks_sink::ReplSink;
use super::workflow_exec::{run_workflow_definition, run_workflow_path};

/// 执行态每 tick 重写 transcript，去掉尾部多余 `\n`，避免与主区 padding 叠出「空行带」。
fn normalize_stream_plain_for_transcript(s: String) -> String {
    let t = s.trim_end_matches(['\n', '\r']);
    if t.is_empty() {
        String::new()
    } else {
        format!("{t}\n")
    }
}
use anycode_core::strip_llm_reasoning_xml_blocks;
use anycode_security::ApprovalCallback;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// [`repl_dispatch_inner`] 的结果：管道/流式 REPL 共用斜杠与内置命令，自然语言由调用方决定阻塞或 spawn。
enum ReplDispatchOutcome {
    Exit,
    Handled,
    SpawnNatural { prompt: String },
}

/// 流式 / 经典行 REPL 底栏顶行：与全屏 TUI 脚标同一信息维度（provider · model · agent · 审批）。
pub(crate) fn repl_stream_dock_status_line(config: &Config, agent: &str) -> String {
    let skip = config.security.session_skip_interactive_approval;
    let appr = if skip {
        tr("repl-dock-approval-skipped")
    } else if security_wants_interactive_approval_callback(config) {
        tr("repl-dock-approval-on")
    } else {
        tr("repl-dock-approval-off")
    };
    format!(
        "{} · {} · {} · {}",
        config.llm.provider, config.llm.model, agent, appr
    )
}

fn sync_repl_dock_status(
    line_state: &Arc<Mutex<crate::repl_inline::ReplLineState>>,
    config: &Config,
    agent: &str,
    turn_in_progress: bool,
) {
    if let Ok(mut s) = line_state.lock() {
        // 执行态 `✶ thinking…` 在绘制时经 [`stream_dock_activity_prefix`] 拼在脚标前；此处只写 provider · model · agent · 审批。
        s.dock_status = repl_stream_dock_status_line(config, agent);
        if !turn_in_progress {
            s.executing_since = None;
        }
    }
}

async fn repl_clear_session(
    runtime: &Arc<AgentRuntime>,
    line_session: &mut ReplLineSession,
    agent: &str,
    sink: &mut ReplSink,
) -> anyhow::Result<()> {
    line_session.rebuild_for_agent(runtime, agent).await?;
    sink.line(tr("repl-session-cleared"));
    Ok(())
}

async fn repl_compact_line_session(
    runtime: &Arc<AgentRuntime>,
    line_session: &mut ReplLineSession,
    agent: &str,
    custom: Option<String>,
    sink: &mut ReplSink,
) -> anyhow::Result<()> {
    let snap = line_session.messages.lock().await.clone();
    if snap.len() < 2 {
        sink.line(tr("tui-err-compact-empty"));
        return Ok(());
    }
    let at = AgentType::new(agent.to_string());
    let instr = custom.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let (new_msgs, _) = runtime
        .compact_session_messages(
            &at,
            &line_session.working_dir_str,
            &snap,
            instr,
            false,
            None,
        )
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    *line_session.messages.lock().await = new_msgs;
    sink.line(tr("tui-compact-done"));
    Ok(())
}

/// `embedded_main_entry`: 仅 **无子命令 + 非 TTY**（stdio）入口为 true，欢迎框与 `anycode repl` 区分。
pub(crate) async fn run_interactive(
    mut config: Config,
    agent: String,
    directory: Option<PathBuf>,
    model: Option<String>,
    session_skip_approval: bool,
    _debug: bool,
    repl_debug_events: bool,
    resume: Option<Uuid>,
    embedded_main_entry: bool,
) -> anyhow::Result<()> {
    use tokio::io::{AsyncBufReadExt, BufReader};

    apply_optional_repl_model(&mut config, model)?;

    let working_dir = directory.unwrap_or_else(|| std::env::current_dir().unwrap());
    let working_dir = std::fs::canonicalize(&working_dir).unwrap_or(working_dir);
    workspace::apply_project_overlays(&mut config, &working_dir);

    let is_tty = std::io::stdin().is_terminal();
    let welcome_kind = if embedded_main_entry {
        ReplWelcomeKind::EmbeddedMain
    } else {
        ReplWelcomeKind::ReplSubcommand
    };
    // TTY：主缓冲流式 dock（ratatui Inline）；非 TTY：stdio 行读。
    let use_stream_tty = is_tty;

    let (runtime, approval_rx, question_rx) = if use_stream_tty {
        let require_approval = security_wants_interactive_approval_callback(&config);
        let (approval_tx, approval_rx) = mpsc::channel::<PendingApproval>(4);
        let (uq_tx, uq_rx) = mpsc::channel::<PendingUserQuestion>(4);
        let uq_host = crate::ask_user_host::ChannelAskUserQuestionHost::new(uq_tx).into_arc();
        let approval_override: Option<Box<dyn ApprovalCallback>> = if require_approval {
            Some(Box::new(TuiApprovalCallback::new(approval_tx)))
        } else {
            None
        };
        let runtime = initialize_runtime(&config, approval_override, Some(uq_host)).await?;
        (
            runtime,
            if require_approval {
                Some(approval_rx)
            } else {
                None
            },
            Some(uq_rx),
        )
    } else {
        let runtime = initialize_runtime(&config, None, None).await?;
        (runtime, None, None)
    };

    let disk = DiskTaskOutput::new_default()?;

    let mut agent = agent;
    let mut line_session =
        ReplLineSession::bootstrap(&runtime, &working_dir, &agent, resume, &config.llm.model)
            .await?;

    if is_tty {
        run_interactive_tty(
            &runtime,
            &disk,
            &working_dir,
            &mut agent,
            &mut config,
            session_skip_approval,
            &mut line_session,
            welcome_kind,
            approval_rx,
            question_rx,
            repl_debug_events,
        )
        .await?;
    } else {
        repl_banner::print_repl_welcome(&working_dir, &agent, session_skip_approval, welcome_kind);
        let stdin = BufReader::new(tokio::io::stdin());
        let mut lines = stdin.lines();
        loop {
            repl_banner::print_repl_prompt();
            let _ = std::io::stdout().flush();

            let line = match lines.next_line().await? {
                None => break,
                Some(l) => l,
            };
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let mut sink = ReplSink::Stdio;
            if repl_dispatch_one_line(
                &runtime,
                &disk,
                &working_dir,
                &mut agent,
                &mut config,
                &mut line_session,
                trimmed,
                &mut sink,
                None,
            )
            .await?
            {
                break;
            }
        }
    }

    repl_banner::print_repl_goodbye(line_session.session_file_id);
    Ok(())
}

async fn run_interactive_tty(
    runtime: &Arc<AgentRuntime>,
    disk: &DiskTaskOutput,
    working_dir: &PathBuf,
    agent: &mut String,
    config: &mut Config,
    session_skip_approval: bool,
    line_session: &mut ReplLineSession,
    welcome_kind: ReplWelcomeKind,
    approval_rx: Option<mpsc::Receiver<PendingApproval>>,
    question_rx: Option<mpsc::Receiver<PendingUserQuestion>>,
    repl_debug_events: bool,
) -> anyhow::Result<()> {
    repl_banner::print_repl_welcome(working_dir, agent, session_skip_approval, welcome_kind);
    run_interactive_tty_stream(
        runtime,
        disk,
        working_dir,
        agent,
        config,
        session_skip_approval,
        line_session,
        welcome_kind,
        approval_rx,
        question_rx,
        repl_debug_events,
    )
    .await
}

/// 从常见 JSON 错误体里抽出 `"message":"…"`（宽松扫描，适配 Google/OpenAI 风格 body）。
fn extract_json_error_message(s: &str) -> Option<String> {
    let key = "\"message\"";
    let mut start = 0usize;
    while start < s.len() {
        let Some(rel) = s.get(start..).and_then(|t| t.find(key)) else {
            break;
        };
        let abs = start + rel;
        let rest = s.get(abs + key.len()..).unwrap_or("");
        let mut after = rest.trim_start();
        after = after.strip_prefix(':').unwrap_or(after).trim_start();
        let Some(q) = after.strip_prefix('"') else {
            start = abs.saturating_add(1);
            continue;
        };
        let mut out = String::new();
        let mut chars = q.chars();
        while let Some(c) = chars.next() {
            match c {
                '"' => break,
                '\\' => {
                    let Some(n) = chars.next() else {
                        break;
                    };
                    out.push(match n {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        '\\' => '\\',
                        '"' => '"',
                        other => other,
                    });
                }
                c => out.push(c),
            }
        }
        let t = out.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
        start = abs.saturating_add(1);
    }
    None
}

/// 流式主区只保留短可读摘要；完整错误走 stderr，避免整段 JSON 与底栏横线叠在一起像「断在横线里」。
fn compact_stream_turn_fail_for_transcript(full: &str) -> String {
    const MAX_LINE: usize = 280;
    if full.contains("User location is not supported") {
        return format!("Turn failed: {}", tr("repl-stream-error-google-geo"));
    }
    if let Some(m) = extract_json_error_message(full) {
        let m = m.trim();
        if m.is_empty() {
            // fall through
        } else if m.chars().count() <= MAX_LINE && m.lines().count() <= 3 {
            return format!("Turn failed: {m}");
        } else {
            let first = m.lines().next().unwrap_or(m).trim();
            let head: String = first.chars().take(MAX_LINE).collect();
            let ell = if first.chars().count() > MAX_LINE {
                "…"
            } else {
                ""
            };
            return format!(
                "Turn failed: {head}{ell}\n{}",
                tr("repl-stream-error-stderr-hint")
            );
        }
    }
    let mut lines = full.lines();
    let first = lines.next().unwrap_or("").trim();
    if first.is_empty() {
        return format!("Turn failed: {}", tr("repl-stream-error-stderr-hint"));
    }
    if first.chars().count() > MAX_LINE {
        let head: String = first.chars().take(MAX_LINE).collect();
        return format!(
            "Turn failed: {head}…\n{}",
            tr("repl-stream-error-stderr-hint")
        );
    }
    if lines.next().is_some() {
        return format!(
            "Turn failed: {first}\n{}",
            tr("repl-stream-error-stderr-hint")
        );
    }
    format!("Turn failed: {first}")
}

fn write_stream_turn_failure(sink: &mut ReplSink, prefix: &str, e: impl std::fmt::Display) {
    let full = format!("{prefix}{e}");
    sink.eprint_line(&full);
    sink.line("");
    sink.line(compact_stream_turn_fail_for_transcript(&full));
}

/// 回合失败时去掉末尾 `Assistant`：流式占位里可能已有错误 JSON/碎片，不删会与 `Turn failed` 摘要叠成两段。
async fn pop_trailing_assistant_after_failed_turn(session: &ReplLineSession) {
    let mut g = session.messages.lock().await;
    if let Some(last) = g.last() {
        if last.role == MessageRole::Assistant {
            g.pop();
        }
    }
}

async fn finish_stream_spawned_turn(
    result: Result<anyhow::Result<anycode_core::TurnOutput>, tokio::task::JoinError>,
    _exec_prev_len: usize,
    line_session: &ReplLineSession,
    agent: &str,
    stream_emitted: &str,
    sink: &mut ReplSink,
) -> anyhow::Result<()> {
    let turn_failed = matches!(&result, Ok(Err(_)) | Err(_));
    if turn_failed {
        pop_trailing_assistant_after_failed_turn(line_session).await;
    }
    match result {
        Ok(Ok(out)) => {
            sink.line(tr("repl-task-ok"));
            let ft = strip_llm_reasoning_xml_blocks(out.final_text.trim_end());
            let show_block = !ft.trim().is_empty()
                && !stream_emitted
                    .trim()
                    .contains(ft.trim().trim_matches(|c: char| c == '\n' || c == ' '));
            if show_block {
                sink.line("");
                sink.line(tr("repl-output-header"));
                sink.line(&out.final_text);
            }
            let written = crate::artifact_summary::claude_turn_written_lines(&out.artifacts);
            if !written.is_empty() {
                sink.line("");
                sink.line(tr("repl-written-header"));
                for line in written {
                    let mut wl = FluentArgs::new();
                    wl.set("line", line);
                    sink.line(tr_args("repl-written-line", &wl));
                }
            }
        }
        Ok(Err(e)) => {
            write_stream_turn_failure(sink, "Turn failed: ", e);
        }
        Err(e) => {
            write_stream_turn_failure(sink, "Turn join error: ", e);
        }
    }
    crate::tui::tui_session_persist::spawn_persist_tui_session(
        line_session.session_file_id,
        line_session.working_dir_str.clone(),
        agent.to_string(),
        line_session.model_persist.clone(),
        line_session.messages.clone(),
    );
    Ok(())
}

/// 流式输出 + 底部 dock：ratatui `Viewport::Inline` + 专用 UI 线程内 `poll`/`read`（与全屏 TUI 栈一致）。
async fn run_interactive_tty_stream(
    runtime: &Arc<AgentRuntime>,
    disk: &DiskTaskOutput,
    working_dir: &PathBuf,
    agent: &mut String,
    config: &mut Config,
    _session_skip_approval: bool,
    line_session: &mut ReplLineSession,
    _welcome_kind: ReplWelcomeKind,
    mut approval_rx: Option<mpsc::Receiver<PendingApproval>>,
    mut question_rx: Option<mpsc::Receiver<PendingUserQuestion>>,
    repl_debug_events: bool,
) -> anyhow::Result<()> {
    use crate::repl_inline::ReplLineState;
    use crate::repl_stream_ratatui::{
        run_stream_repl_ui_thread, StreamReplAsyncCtl, StreamReplUiMsg,
    };
    use std::sync::mpsc as std_mpsc;

    let state = Arc::new(Mutex::new(ReplLineState::default()));
    let (ui_tx, mut ui_rx) = tokio::sync::mpsc::unbounded_channel::<StreamReplUiMsg>();
    let (ctrl_tx, ctrl_rx) = std_mpsc::channel::<StreamReplAsyncCtl>();

    let state_th = state.clone();
    let ui_join = std::thread::Builder::new()
        .name("anycode-repl-stream-ui".into())
        .spawn(move || run_stream_repl_ui_thread(state_th, ui_tx, ctrl_rx, repl_debug_events))
        .map_err(|e| anyhow::anyhow!("spawn anycode-repl-stream-ui: {e}"))?;

    let mut exec_handle: Option<JoinHandle<anyhow::Result<anycode_core::TurnOutput>>> = None;
    let mut executing = false;
    let mut exec_prev_len: usize = 0;
    // 本轮任务写入前 `transcript` 字节偏移；执行中按 tick 截断至此再重算 `build_stream_turn_plain`。
    let mut turn_transcript_anchor: usize = 0;
    let mut stream_scroll_emitted = String::new();

    sync_repl_dock_status(&state, config, agent, false);
    let mut stream_tick = tokio::time::interval(Duration::from_millis(50));
    stream_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        if let Ok(mut st) = state.lock() {
            if let Some(until) = st.finished_turn_summary_until {
                if Instant::now() >= until {
                    st.finished_turn_summary_until = None;
                    st.finished_turn_summary = None;
                }
            }
        }
        if let Some(ref mut rx) = approval_rx {
            loop {
                match rx.try_recv() {
                    Ok(r) => {
                        let mut st = state.lock().unwrap_or_else(|e| e.into_inner());
                        st.approval_menu_selected = 0;
                        if let Some(old) = st.pending_approval.replace(r) {
                            let _ = old.reply.send(ApprovalDecision::Deny);
                        }
                    }
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => break,
                }
            }
        }
        if let Some(ref mut qrx) = question_rx {
            loop {
                match qrx.try_recv() {
                    Ok(r) => {
                        let mut st = state.lock().unwrap_or_else(|e| e.into_inner());
                        st.user_question_menu_selected = 0;
                        if let Some(old) = st.pending_user_question.replace(r) {
                            let _ = old.reply.send(Err(()));
                        }
                    }
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => break,
                }
            }
        }

        if let Some(h) = exec_handle.as_ref() {
            if h.is_finished() {
                let h = exec_handle.take().unwrap();
                executing = false;
                let turn_wall_secs = {
                    let mut st = state.lock().unwrap_or_else(|e| e.into_inner());
                    st.executing_since
                        .take()
                        .map(|t| t.elapsed().as_secs().max(1))
                        .unwrap_or(1)
                };
                let join_result = h.await;
                let usage_for_hud = match &join_result {
                    Ok(Ok(ref o)) => o.usage,
                    _ => anycode_core::TurnTokenUsage::default(),
                };
                if let Ok(mut st) = state.lock() {
                    st.last_turn_token_usage = match &join_result {
                        Ok(Ok(ref o)) => Some(o.usage),
                        _ => st.last_turn_token_usage,
                    };
                    st.finished_turn_summary =
                        Some(crate::tui::hud_text::prompt_hud_stream_turn_summary_text(
                            turn_wall_secs,
                            &usage_for_hud,
                        ));
                    st.finished_turn_summary_until = Some(Instant::now() + Duration::from_secs(10));
                }
                let mut sink = ReplSink::Stream {
                    state: state.clone(),
                    tail: String::new(),
                };
                let transcript_len_before_finish = state
                    .lock()
                    .ok()
                    .and_then(|st| st.transcript.lock().ok().map(|t| t.len()));
                finish_stream_spawned_turn(
                    join_result,
                    exec_prev_len,
                    line_session,
                    agent.as_str(),
                    &stream_scroll_emitted,
                    &mut sink,
                )
                .await?;
                let appended_after_finish = transcript_len_before_finish.and_then(|start| {
                    state.lock().ok().and_then(|st| {
                        st.transcript
                            .lock()
                            .ok()
                            .map(|t| t.as_str()[start..].to_string())
                    })
                });
                {
                    let w = state
                        .lock()
                        .map(|s| s.stream_viewport_width.max(40))
                        .unwrap_or(80) as usize;
                    let guard = line_session.messages.lock().await;
                    let plain = normalize_stream_plain_for_transcript(build_stream_turn_plain(
                        exec_prev_len,
                        &guard,
                        w,
                        false,
                    ));
                    drop(guard);
                    if let Ok(st) = state.lock() {
                        if let Ok(mut t) = st.transcript.lock() {
                            t.truncate(turn_transcript_anchor);
                            t.push_str(&plain);
                            if let Some(tail) = appended_after_finish.as_deref() {
                                t.push_str(tail);
                            }
                        }
                    }
                }
                stream_scroll_emitted.clear();
                if let Ok(mut st) = state.lock() {
                    st.stream_transcript_scroll = 0;
                }
                sync_repl_dock_status(&state, config, agent, false);
            }
        }

        if executing {
            if let Ok(guard) = line_session.messages.try_lock() {
                let w = state
                    .lock()
                    .map(|s| s.stream_viewport_width.max(40))
                    .unwrap_or(80) as usize;
                let plain = normalize_stream_plain_for_transcript(build_stream_turn_plain(
                    exec_prev_len,
                    &guard,
                    w,
                    true,
                ));
                drop(guard);
                stream_scroll_emitted.clear();
                stream_scroll_emitted.push_str(&plain);
                if let Ok(st) = state.lock() {
                    if let Ok(mut t) = st.transcript.lock() {
                        t.truncate(turn_transcript_anchor);
                        t.push_str(&plain);
                    }
                }
            }
        }

        tokio::select! {
            biased;
            msg = ui_rx.recv() => {
                let Some(msg) = msg else {
                    break;
                };
                match msg {
                    StreamReplUiMsg::Eof => {
                        break;
                    }
                    StreamReplUiMsg::ClearSession => {
                        let mut sink = ReplSink::Stream {
                            state: state.clone(),
                            tail: String::new(),
                        };
                        repl_clear_session(runtime, line_session, agent, &mut sink).await?;
                        if let Ok(mut st) = state.lock() {
                            st.stream_transcript_scroll = 0;
                            st.executing_since = None;
                            st.finished_turn_summary = None;
                            st.finished_turn_summary_until = None;
                            st.last_turn_token_usage = None;
                            st.stream_exit_dump_anchor = 0;
                        }
                        sync_repl_dock_status(&state, config, agent, executing);
                    }
                    StreamReplUiMsg::Submit(text) => {
                        let t = crate::tui::util::trim_or_default(text.as_str());
                        if t.is_empty() {
                            continue;
                        }
                        if let Ok(mut st) = state.lock() {
                            st.stream_transcript_scroll = 0;
                        }
                        let workflow_esc = t.trim_start().starts_with("/workflow");
                        let busy_executing = executing;
                        let paste_state = state.clone();
                        let done = if workflow_esc {
                            let (ack_tx, ack_rx) = std_mpsc::channel();
                            let _ = ctrl_tx.send(StreamReplAsyncCtl::SuspendForSubprocess(ack_tx));
                            tokio::task::spawn_blocking(move || ack_rx.recv())
                                .await
                                .map_err(|_| anyhow::anyhow!("suspend join"))?
                                .map_err(|_| anyhow::anyhow!("repl UI ended during suspend"))?;
                            let mut sink = ReplSink::Stdio;
                            let d = repl_dispatch_one_line(
                                runtime,
                                disk,
                                working_dir,
                                agent,
                                config,
                                line_session,
                                t,
                                &mut sink,
                                Some(paste_state.clone()),
                            )
                            .await?;
                            let (ack_tx, ack_rx) = std_mpsc::channel();
                            let _ = ctrl_tx.send(StreamReplAsyncCtl::ResumeAfterSubprocess(ack_tx));
                            tokio::task::spawn_blocking(move || ack_rx.recv())
                                .await
                                .map_err(|_| anyhow::anyhow!("resume join"))?
                                .map_err(|_| anyhow::anyhow!("repl UI ended during resume"))?;
                            d
                        } else {
                            let mut sink = ReplSink::Stream {
                                state: state.clone(),
                                tail: String::new(),
                            };
                            let dispatch = repl_dispatch_inner(
                                runtime,
                                disk,
                                working_dir,
                                agent,
                                config,
                                line_session,
                                t,
                                &mut sink,
                                Some(paste_state),
                            )
                            .await?;
                            let mut done_nl = false;
                            match dispatch {
                                ReplDispatchOutcome::Exit => done_nl = true,
                                ReplDispatchOutcome::Handled => {}
                                ReplDispatchOutcome::SpawnNatural { prompt } => {
                                    if busy_executing {
                                        sink.line(tr("repl-busy-natural"));
                                    } else {
                                        {
                                            let mut st = state.lock().unwrap();
                                            turn_transcript_anchor = {
                                                let t = st.transcript.lock().unwrap();
                                                t.len()
                                            };
                                            st.stream_exit_dump_anchor = turn_transcript_anchor;
                                            st.executing_since = Some(std::time::Instant::now());
                                            st.finished_turn_summary = None;
                                            st.finished_turn_summary_until = None;
                                        }
                                        let (handle, prev) = repl_line_session::append_user_spawn_turn(
                                            runtime,
                                            line_session,
                                            agent,
                                            &prompt,
                                        )
                                        .await?;
                                        exec_handle = Some(handle);
                                        exec_prev_len = prev;
                                        executing = true;
                                        stream_scroll_emitted.clear();
                                    }
                                }
                            }
                            done_nl
                        };
                        sync_repl_dock_status(&state, config, agent, executing);
                        if done {
                            break;
                        }
                    }
                }
            }
            _ = stream_tick.tick() => {}
        }
    }

    let _ = ctrl_tx.send(StreamReplAsyncCtl::Shutdown);
    ui_join
        .join()
        .map_err(|_| anyhow::anyhow!("repl stream UI thread panicked"))??;

    Ok(())
}

async fn repl_dispatch_inner(
    runtime: &Arc<AgentRuntime>,
    disk: &DiskTaskOutput,
    working_dir: &PathBuf,
    agent: &mut String,
    config: &mut Config,
    line_session: &mut ReplLineSession,
    trimmed: &str,
    sink: &mut ReplSink,
    stream_paste_state: Option<Arc<Mutex<crate::repl_inline::ReplLineState>>>,
) -> anyhow::Result<ReplDispatchOutcome> {
    if let Some(id) = parse_agent_slash_command(trimmed) {
        *agent = id.to_string();
        line_session.rebuild_for_agent(runtime, agent).await?;
        let mut a = FluentArgs::new();
        a.set("id", id);
        sink.line(tr_args("repl-agent-switched", &a));
        sink.line("");
        return Ok(ReplDispatchOutcome::Handled);
    }

    if let Some(cmd) = slash_commands::parse(trimmed) {
        match cmd {
            ParsedSlashCommand::Mode(arg) => {
                if let Some(mode) = arg {
                    if let Some(parsed) = RuntimeMode::parse(&mode) {
                        *agent = parsed.default_agent().as_str().to_string();
                        line_session.rebuild_for_agent(runtime, agent).await?;
                        sink.line(format!("mode -> {} (agent: {})", parsed.as_str(), agent));
                    } else {
                        sink.line(format!("unknown mode: {}", mode));
                    }
                } else {
                    sink.line(format!("current agent: {}", agent));
                }
            }
            ParsedSlashCommand::Status => {
                sink.line(format!("agent: {}", agent));
                sink.line(format!("provider: {}", config.llm.provider));
                sink.line(format!("model: {}", config.llm.model));
                sink.line(format!(
                    "default_mode: {}",
                    config.runtime.default_mode.as_str()
                ));
            }
            ParsedSlashCommand::Workflow(arg) => {
                let maybe_path = arg.as_deref().and_then(|raw| {
                    let trimmed = raw.trim();
                    trimmed
                        .strip_prefix("run ")
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(PathBuf::from)
                });
                if let Some(path) = maybe_path {
                    run_workflow_path(runtime, disk, working_dir, &path, Some(trimmed.to_string()))
                        .await?;
                } else {
                    match workflows::discover_workflow(working_dir) {
                        Ok(Some((path, workflow))) => {
                            if arg.as_deref().map(|s| s.trim()) == Some("run") {
                                run_workflow_definition(
                                    runtime,
                                    disk,
                                    working_dir,
                                    &workflow,
                                    &path,
                                    Some(trimmed.to_string()),
                                )
                                .await?;
                            } else {
                                sink.line(format!(
                                    "workflow: {} ({})",
                                    workflow.name,
                                    path.display()
                                ));
                            }
                        }
                        Ok(None) => sink.line("workflow: none"),
                        Err(e) => sink.line(format!("workflow error: {}", e)),
                    }
                }
            }
            ParsedSlashCommand::Compact(custom) => {
                repl_compact_line_session(runtime, line_session, agent, custom, sink).await?;
            }
            ParsedSlashCommand::Clear => {
                repl_clear_session(runtime, line_session, agent, sink).await?;
            }
            ParsedSlashCommand::Context => {
                let n = line_session.messages.lock().await.len();
                let win = crate::app_config::effective_session_context_window_tokens(
                    &config.session,
                    config.llm.provider.as_str(),
                    config.llm.model.as_str(),
                );
                let (last_in, usage_opt) = if let Some(arc) = stream_paste_state.as_ref() {
                    if let Ok(st) = arc.lock() {
                        if let Some(tu) = st.last_turn_token_usage {
                            let u = tu.to_usage();
                            (tu.max_input_tokens, Some(u))
                        } else {
                            (0u32, None)
                        }
                    } else {
                        (0u32, None)
                    }
                } else {
                    (0u32, None)
                };
                let lines = crate::session_transcript_export::format_context_lines(
                    n,
                    win,
                    last_in,
                    usage_opt.as_ref(),
                );
                for line in lines {
                    sink.line(line);
                }
            }
            ParsedSlashCommand::Cost => {
                let n = line_session.messages.lock().await.len();
                let win = crate::app_config::effective_session_context_window_tokens(
                    &config.session,
                    config.llm.provider.as_str(),
                    config.llm.model.as_str(),
                );
                let (last_in, usage_opt) = if let Some(arc) = stream_paste_state.as_ref() {
                    if let Ok(st) = arc.lock() {
                        if let Some(tu) = st.last_turn_token_usage {
                            let u = tu.to_usage();
                            (tu.max_input_tokens, Some(u))
                        } else {
                            (0u32, None)
                        }
                    } else {
                        (0u32, None)
                    }
                } else {
                    (0u32, None)
                };
                let lines = crate::session_transcript_export::format_cost_lines(
                    n,
                    win,
                    last_in,
                    usage_opt.as_ref(),
                );
                for line in lines {
                    sink.line(line);
                }
            }
            ParsedSlashCommand::Export(arg) => {
                let msgs = line_session.messages.lock().await.clone();
                let text = crate::session_transcript_export::messages_to_plain_export(&msgs);
                let path = match arg.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                    Some(p) => {
                        let pb = PathBuf::from(p);
                        if pb.is_absolute() {
                            pb
                        } else {
                            working_dir.join(pb)
                        }
                    }
                    None => {
                        let id = line_session.session_file_id.simple().to_string();
                        let short: String = id.chars().take(8).collect();
                        working_dir.join(format!("anycode-export-{short}.txt"))
                    }
                };
                std::fs::write(&path, text.as_bytes())
                    .map_err(|e| anyhow::anyhow!("{} {path:?}: {e}", tr("repl-export-failed")))?;
                let mut a = FluentArgs::new();
                a.set("path", path.display().to_string());
                sink.line(tr_args("repl-export-done", &a));
            }
            ParsedSlashCommand::Session(arg) => {
                if arg.as_deref().map(str::trim) == Some("list") {
                    sink.line(repl_line_session::format_session_list_for_repl());
                    sink.line("");
                    return Ok(ReplDispatchOutcome::Handled);
                }
                let cwd_s = working_dir.to_string_lossy().to_string();
                let snap = match repl_line_session::load_repl_session_choice(arg, &cwd_s) {
                    Ok(s) => s,
                    Err(e) => {
                        let m = format!("{e:#}");
                        if m.contains("no_saved_sessions") {
                            sink.line(tr("repl-session-resolve-none"));
                        } else {
                            sink.line(m);
                        }
                        sink.line("");
                        return Ok(ReplDispatchOutcome::Handled);
                    }
                };
                line_session.apply_snapshot(snap, agent).await;
                let mut a = FluentArgs::new();
                a.set("id", line_session.session_file_id.to_string());
                sink.line(tr_args("repl-session-applied", &a));
            }
            ParsedSlashCommand::Paste => {
                use crate::repl_inline::reset_slash_state;
                use crate::tui::util::{sanitize_paste, MAX_PASTE_CHARS};
                if let Some(st_arc) = stream_paste_state.as_ref() {
                    if let Some(raw) = crate::repl_clipboard::read_system_clipboard() {
                        let (clean, truncated) = sanitize_paste(raw);
                        if truncated {
                            let mut a = FluentArgs::new();
                            a.set("n", MAX_PASTE_CHARS as i64);
                            sink.eprint_line(tr_args("tui-err-paste-truncated", &a));
                        }
                        if let Ok(mut st) = st_arc.lock() {
                            st.input.insert_str(&clean);
                            st.history_idx = None;
                            reset_slash_state(&mut st);
                        }
                    } else {
                        sink.line(tr("repl-paste-clipboard-failed"));
                    }
                } else {
                    sink.line(tr("repl-paste-need-line-edit"));
                }
            }
        }
        sink.line("");
        return Ok(ReplDispatchOutcome::Handled);
    }

    match trimmed {
        "exit" | "quit" | ":q" | "/exit" => return Ok(ReplDispatchOutcome::Exit),
        "help" | "?" | "/help" => {
            let mut h = FluentArgs::new();
            h.set("cwd", format!("{:?}", working_dir));
            h.set("agent", agent.clone());
            sink.line(tr_args("repl-help-equiv", &h));
            sink.line(tr("repl-help-cmds"));
            for line in slash_commands::help_lines() {
                sink.line(line);
            }
        }
        "agents" | "list-agents" | "/agents" => list_agents(sink),
        "tools" | "list-tools" | "/tools" => list_tools(sink),
        prompt => {
            return Ok(ReplDispatchOutcome::SpawnNatural {
                prompt: prompt.to_string(),
            });
        }
    }
    sink.line("");
    Ok(ReplDispatchOutcome::Handled)
}

/// 处理单行 REPL 输入；返回 `true` 表示应退出循环。
/// `Stream` 写入主缓冲滚动区并重绘底部 dock；
/// `Stdio` 走真实终端。
async fn repl_dispatch_one_line(
    runtime: &Arc<AgentRuntime>,
    disk: &DiskTaskOutput,
    working_dir: &PathBuf,
    agent: &mut String,
    config: &mut Config,
    line_session: &mut ReplLineSession,
    trimmed: &str,
    sink: &mut ReplSink,
    stream_paste_state: Option<Arc<Mutex<crate::repl_inline::ReplLineState>>>,
) -> anyhow::Result<bool> {
    match repl_dispatch_inner(
        runtime,
        disk,
        working_dir,
        agent,
        config,
        line_session,
        trimmed,
        sink,
        stream_paste_state,
    )
    .await?
    {
        ReplDispatchOutcome::Exit => Ok(true),
        ReplDispatchOutcome::Handled => {
            sink.line("");
            Ok(false)
        }
        ReplDispatchOutcome::SpawnNatural { prompt } => {
            repl_line_session::run_line_repl_turn(
                runtime,
                line_session,
                agent.as_str(),
                &prompt,
                sink,
            )
            .await?;
            sink.line("");
            Ok(false)
        }
    }
}

pub(crate) fn list_agents(sink: &mut ReplSink) {
    use crate::builtin_agents::BUILTIN_AGENT_IDS;
    sink.line(tr("repl-list-agents-title"));
    sink.line("");
    for id in BUILTIN_AGENT_IDS {
        let desc = match id {
            "general-purpose" => tr("repl-agent-desc-gp"),
            "explore" => tr("repl-agent-desc-explore"),
            "plan" => tr("repl-agent-desc-plan"),
            "workspace-assistant" => "Workspace-first assistant for channel mode.".to_string(),
            "goal" => "Goal loop agent with retries and progress tracking.".to_string(),
            _ => String::new(),
        };
        sink.line(format!("  • {}", id));
        if !desc.is_empty() {
            sink.line(desc);
        }
        sink.line("");
    }
    sink.line(tr("repl-list-switch"));
    sink.line("");
    sink.line(tr("repl-list-usage"));
    sink.line(tr("repl-list-usage-line"));
}

pub(crate) fn list_tools(sink: &mut ReplSink) {
    sink.line(tr("repl-list-tools-title"));
    sink.line("");
    for (name, desc) in iter_cli_tool_help() {
        sink.line(format!("  • {}", name));
        sink.line(format!("    {}", desc));
        sink.line("");
    }
    sink.line(tr("repl-security-title"));
    sink.line(tr("repl-security-read"));
    sink.line(tr("repl-security-approval"));
    sink.line(tr("repl-security-sandbox"));
}

#[cfg(test)]
mod stream_fail_compact_tests {
    use super::extract_json_error_message;

    #[test]
    fn extracts_message_from_google_style_json() {
        let s = r#"LLM error: google … body=[{ "error": { "message": "User location is not supported for the API use.", "status": "FAILED_PRECONDITION" } }]"#;
        let m = extract_json_error_message(s).expect("message");
        assert!(m.contains("User location"), "unexpected message: {m:?}");
    }
}
