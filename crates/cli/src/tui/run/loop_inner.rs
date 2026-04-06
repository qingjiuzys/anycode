use super::*;

use crate::tui::transcript::TranscriptEntry;
use anycode_core::Artifact;
use ratatui::text::Line;
use std::collections::HashSet;
use std::time::Instant;
use tracing::debug;
use uuid::Uuid;

pub async fn run_tui(
    mut config: Config,
    agent: String,
    directory: Option<PathBuf>,
    model: Option<String>,
    debug: bool,
) -> anyhow::Result<()> {
    crate::app_config::apply_optional_repl_model(&mut config, model)?;

    let working_dir = directory.unwrap_or_else(|| std::env::current_dir().unwrap());
    let working_dir = std::fs::canonicalize(&working_dir).unwrap_or(working_dir);
    crate::workspace::apply_project_overlays(&mut config, &working_dir);
    let working_dir_str = working_dir.to_string_lossy().to_string();

    debug!("Starting TUI mode...");
    debug!(path = %working_dir_str, "TUI working directory");

    let permission_mode = config.security.permission_mode.clone();
    let require_approval = crate::app_config::security_wants_interactive_approval_callback(&config);
    let llm_provider = config.llm.provider.clone();
    let llm_plan = config.llm.plan.clone();
    let llm_model = config.llm.model.clone();

    let (approval_tx, mut approval_rx) = mpsc::channel::<PendingApproval>(4);
    // 无审批回调时若 drop Sender，channel 关闭，`approval_rx.recv()` 会立刻返回 None。
    // `tokio::select!` 且 `biased` 时第一分支持续就绪，键盘轮询分支永远轮不到 → Prompt 无法输入。
    let (_approval_tx_keepalive, approval_override) = if require_approval {
        (
            None,
            Some(Box::new(TuiApprovalCallback::new(approval_tx)) as Box<dyn ApprovalCallback>),
        )
    } else {
        (Some(approval_tx), None)
    };

    let runtime = initialize_runtime(&config, approval_override).await?;

    let mut agent_type = anycode_core::AgentType::new(agent.clone());
    let messages = Arc::new(Mutex::new(vec![
        runtime
            .build_system_message(&agent_type, &working_dir_str)
            .await?,
    ]));

    let mut transcript: Vec<TranscriptEntry> = vec![];
    let mut transcript_gen: u64 = 0;
    let workspace_line_count = Cell::new(0usize);
    let mut workspace_cache_lines: Vec<Line<'static>> = Vec::new();
    let mut workspace_cache_gen: u64 = 0;
    let mut workspace_cache_w: usize = 0;
    let mut workspace_cache_fold_rev: u64 = 0;
    let mut workspace_cache_executing: bool = false;
    let mut workspace_cache_working_secs: Option<u64> = None;
    let mut workspace_cache_pulse_frame: u64 = 0;
    let mut fold_layout_rev: u64 = 0;
    let mut tool_folds_expanded: HashSet<u64> = HashSet::new();
    let mut next_tool_fold_id: u64 = 0;
    let mut input = InputState::default();
    let mut input_history: Vec<String> = Vec::new();
    let mut history_idx: Option<usize> = None;
    // Workspace 从底部向上滚动的行数（0 = 跟随最新）。
    let mut transcript_scroll_up: usize = 0;
    let mut rev_search: Option<RevSearchState> = None;
    let mut executing = false;
    let mut executing_since: Option<Instant> = None;
    let mut last_turn_error: Option<String> = None;
    let mut last_key: Option<String> = None;
    let mut help_open = false;

    let mut pending_approval: Option<PendingApproval> = None;
    let mut exec_handle: Option<JoinHandle<anyhow::Result<(String, Vec<Artifact>, u32)>>> = None;
    let mut exec_prev_len: usize = 0;
    let mut last_max_input_tokens: u32 = 0;
    let mut exec_live_tail: Option<(usize, u64)> = None;
    // (messages.len(), last_message_id)：有变化时才重排 transcript，避免每帧整表重算。
    let mut exec_live_sync_fp: Option<(usize, Option<Uuid>)> = None;

    let _tui_guard = super::terminal_guard::TuiTerminalGuard::enter()?;
    let used_alternate_screen = _tui_guard.used_alternate_screen();
    debug!(
        alt = if used_alternate_screen { "on" } else { "off" },
        "TUI alternate screen"
    );

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    // 主缓冲模式：清屏并归零光标，避免启动阶段 stderr 日志与 ratatui 首帧叠在同一视口。
    if !used_alternate_screen {
        use crossterm::{cursor::MoveTo, execute, terminal::{Clear, ClearType}};
        let mut out = stdout();
        let _ = execute!(out, Clear(ClearType::All), MoveTo(0, 0));
    }

    'ui: loop {
        let size = terminal.size()?;
        let show_buddy = size.width >= 52;
        let main_avail_cell = Cell::new(0usize);
        let nl_extra = input.as_string().matches('\n').count().min(12);
        // 底栏：横线(1) + Dock（正文区含外框）；快捷键在 `?` 帮助
        let mut bottom_h: u16 = if pending_approval.is_some() { 15 } else { 7 };
        if rev_search.is_some() {
            bottom_h = bottom_h.max(14);
        }
        bottom_h = bottom_h.saturating_add(nl_extra as u16);
        let max_bottom = size.height.saturating_sub(5);
        bottom_h = bottom_h.min(max_bottom);
        if max_bottom >= 6 {
            bottom_h = bottom_h.max(6);
        }

        let working_elapsed_secs = if executing {
            executing_since.map(|t| t.elapsed().as_secs())
        } else {
            None
        };

        let pet_anim_frame: u64 = if executing {
            executing_since
                .map(|t| ((t.elapsed().as_millis() / 250) % 4) as u64)
                .unwrap_or(0)
        } else {
            0
        };

        terminal.draw(|f| {
            super::draw::draw_tui_frame(
                f,
                super::draw::DrawFrameCtx {
                    size,
                    bottom_h,
                    show_buddy,
                    pet_anim_frame,
                    working_dir_str: &working_dir_str,
                    agent_type: &agent_type,
                    permission_mode: permission_mode.as_str(),
                    require_approval,
                    llm_provider: llm_provider.as_str(),
                    llm_plan: llm_plan.as_str(),
                    llm_model: llm_model.as_str(),
                    debug,
                    last_key: last_key.as_deref(),
                    pending_approval: pending_approval.as_ref(),
                    executing,
                    working_elapsed_secs,
                    help_open,
                    transcript: &transcript,
                    transcript_scroll_up,
                    rev_search: rev_search.as_ref(),
                    input: &input,
                    input_history: &input_history,
                    workspace_cache_lines: &mut workspace_cache_lines,
                    workspace_cache_gen: &mut workspace_cache_gen,
                    workspace_cache_w: &mut workspace_cache_w,
                    workspace_cache_fold_rev: &mut workspace_cache_fold_rev,
                    workspace_cache_executing: &mut workspace_cache_executing,
                    workspace_cache_working_secs: &mut workspace_cache_working_secs,
                    workspace_cache_pulse_frame: &mut workspace_cache_pulse_frame,
                    transcript_gen,
                    fold_layout_rev,
                    expanded_tool_folds: &tool_folds_expanded,
                    main_avail_cell: &main_avail_cell,
                    workspace_line_count: &workspace_line_count,
                },
            );
        })?;

        let _ = super::terminal_guard::refresh_mouse_capture_after_draw();

        let avail = main_avail_cell.get().max(1);
        let max_sc = workspace_line_count.get().saturating_sub(avail);
        transcript_scroll_up = transcript_scroll_up.min(max_sc);

        // 勿使用 `biased`：原先 `biased` + `approval_rx.recv()` 在前时，若 `recv` 与定时器同时就绪会优先
        // `recv`；在 channel 异常等情况下可能反复选中 recv，饿死定时分支 → 永不 poll 键盘/鼠标。
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(16)) => {
                if executing {
                    if let Some((tail_start, fold_base)) = exec_live_tail {
                        if let Ok(g) = messages.try_lock() {
                            let fp = (g.len(), g.last().map(|m| m.id));
                            if exec_live_sync_fp != Some(fp) {
                                super::exec_completion::sync_transcript_with_messages_tail(
                                    &mut transcript,
                                    tail_start,
                                    fold_base,
                                    &mut next_tool_fold_id,
                                    &g,
                                    exec_prev_len,
                                );
                                transcript_gen = transcript_gen.wrapping_add(1);
                                exec_live_sync_fp = Some(fp);
                            }
                        }
                    }
                }

                if let Some(h) = exec_handle.as_ref() {
                    if h.is_finished() {
                        let h = exec_handle.take().unwrap();
                        executing = false;
                        executing_since = None;
                        exec_live_sync_fp = None;
                        let anchor = exec_live_tail.take();
                        super::exec_completion::consume_finished_turn(
                            h,
                            &messages,
                            exec_prev_len,
                            &mut transcript,
                            &mut transcript_gen,
                            &mut next_tool_fold_id,
                            &mut last_turn_error,
                            &mut last_max_input_tokens,
                            anchor,
                        )
                        .await;
                    }
                }

                while ct_event::poll(Duration::ZERO)? {
                    let ev = ct_event::read()?;
                    let mut ectx = super::event::TuiEventCtx {
                        last_key: &mut last_key,
                        transcript_scroll_up: &mut transcript_scroll_up,
                        pending_approval: &mut pending_approval,
                        rev_search: &mut rev_search,
                        input: &mut input,
                        input_history: &mut input_history,
                        history_idx: &mut history_idx,
                        executing: &mut executing,
                        executing_since: &mut executing_since,
                        help_open: &mut help_open,
                        transcript: &mut transcript,
                        transcript_gen: &mut transcript_gen,
                        last_turn_error: &mut last_turn_error,
                        exec_handle: &mut exec_handle,
                        exec_prev_len: &mut exec_prev_len,
                        last_max_input_tokens: &mut last_max_input_tokens,
                        session_cfg: &config.session,
                        llm_provider: llm_provider.as_str(),
                        llm_model: llm_model.as_str(),
                        main_avail_cell: &main_avail_cell,
                        workspace_line_count: &workspace_line_count,
                        tool_folds_expanded: &mut tool_folds_expanded,
                        fold_layout_rev: &mut fold_layout_rev,
                        next_tool_fold_id: &mut next_tool_fold_id,
                        exec_live_tail: &mut exec_live_tail,
                    };
                    match super::event::dispatch_crossterm_event(
                        ev,
                        &mut ectx,
                        &runtime,
                        &messages,
                        &mut agent_type,
                        &working_dir_str,
                    )
                    .await?
                    {
                        // Continue：本帧不再排空后续事件（与原先单事件行为一致）
                        super::event::TuiLoopCtl::Continue => break,
                        super::event::TuiLoopCtl::Break => break 'ui,
                        super::event::TuiLoopCtl::Ok => {}
                    }
                }
            }
            req = approval_rx.recv() => {
                if let Some(r) = req {
                    if let Some(old) = pending_approval.replace(r) {
                        let _ = old.reply.send(false);
                    }
                }
            }
        }
    }

    // 先 drop Terminal 再恢复终端模式：备用屏关闭后把会话写入主缓冲，退出后仍可滚动回看。
    std::mem::drop(terminal);
    std::mem::drop(_tui_guard);

    let skip_dump = std::env::var("ANYCODE_TUI_NO_SCROLLBACK_DUMP")
        .map(|s| matches!(s.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false);
    // 主缓冲模式下会话已在滚动区，不再整段 echo（避免重复）。
    if used_alternate_screen && !skip_dump && !transcript.is_empty() {
        println!("{}", crate::tui::transcript::transcript_dump_plain_text(&transcript));
    }

    Ok(())
}
