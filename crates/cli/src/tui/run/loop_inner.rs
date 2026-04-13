use super::*;

use ratatui::backend::Backend;

use super::exec_completion::{consume_finished_compact, CompactFollowup};
use super::resize_debounce::ResizeDebounce;
use crate::app_config::effective_session_context_window_tokens;
use crate::i18n::tr_args;
use crate::tui::transcript::{apply_tool_transcript_pipeline, TranscriptEntry};
use crate::tui::PendingUserQuestion;
use anycode_core::{Message, Usage};
use ratatui::text::{Line, Span};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::debug;
use uuid::Uuid;

use crate::tui::styles::style_dim;
use fluent_bundle::FluentArgs;

async fn apply_tui_resume_snapshot(
    id: Uuid,
    working_dir_str: &str,
    agent_type: &mut anycode_core::AgentType,
    messages: &Arc<Mutex<Vec<Message>>>,
    transcript: &mut Vec<TranscriptEntry>,
    transcript_gen: &mut u64,
    next_tool_fold_id: &mut u64,
    tool_folds_expanded: &mut HashSet<u64>,
    fold_layout_rev: &mut u64,
    session_uuid: &mut Uuid,
    tui_session_id: &mut String,
    exec_prev_len: &mut usize,
    transcript_scroll_up: &mut usize,
    exec_live_tail: &mut Option<(usize, u64)>,
    exec_live_sync_fp: &mut Option<(usize, Option<Uuid>)>,
    last_turn_error: &mut Option<String>,
) -> anyhow::Result<()> {
    let snap = crate::tui::tui_session_persist::load_tui_session(id)?
        .ok_or_else(|| anyhow::anyhow!("{}", crate::i18n::tr("tui-resume-not-found")))?;
    if snap.workspace_root != working_dir_str {
        tracing::warn!("{}", crate::i18n::tr("tui-resume-cwd-warn"));
    }
    *agent_type = anycode_core::AgentType::new(snap.agent.clone());
    *session_uuid = snap.id;
    *tui_session_id = session_uuid.to_string();
    {
        let mut g = messages.lock().await;
        *g = snap.messages;
    }
    tool_folds_expanded.clear();
    *exec_live_tail = None;
    *exec_live_sync_fp = None;
    *fold_layout_rev = fold_layout_rev.wrapping_add(1);
    transcript.clear();
    *transcript_gen = 0;
    *next_tool_fold_id = 0;
    let frozen = messages.lock().await.clone();
    super::exec_completion::rebuild_transcript_from_messages(
        transcript,
        &frozen,
        transcript_gen,
        next_tool_fold_id,
    );
    apply_tool_transcript_pipeline(transcript, next_tool_fold_id);
    *exec_prev_len = messages.lock().await.len();
    *transcript_scroll_up = 0;
    *last_turn_error = None;
    let mut a = FluentArgs::new();
    a.set("id", session_uuid.to_string());
    transcript.push(TranscriptEntry::Plain(vec![Line::from(Span::styled(
        tr_args("tui-session-resumed", &a),
        style_dim(),
    ))]));
    *transcript_gen = transcript_gen.wrapping_add(1);
    Ok(())
}

/// Workspace 缓存键，用于判断是否需要重新渲染
#[derive(Debug, Clone, PartialEq)]
struct WorkspaceCacheKey {
    gen: u64,
    w: usize,
    fold_rev: u64,
    executing: bool,
    working_secs: Option<u64>,
    // 注意：pulse_frame 变化不触发缓存失效，避免频繁重绘
}

impl WorkspaceCacheKey {
    fn should_invalidate(&self, new: &WorkspaceCacheKey) -> bool {
        self.gen != new.gen
            || self.w != new.w
            || self.fold_rev != new.fold_rev
            || self.executing != new.executing
            || self.working_secs != new.working_secs
    }
}

pub async fn run_tui(
    mut config: Config,
    agent: String,
    directory: Option<PathBuf>,
    model: Option<String>,
    debug: bool,
    resume: Option<Uuid>,
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

    let (uq_tx, mut uq_rx) = mpsc::channel::<PendingUserQuestion>(4);
    let uq_host = crate::ask_user_host::ChannelAskUserQuestionHost::new(uq_tx).into_arc();

    let runtime = initialize_runtime(&config, approval_override, Some(uq_host)).await?;

    let snap_loaded = if let Some(id) = resume {
        match crate::tui::tui_session_persist::load_tui_session(id)? {
            Some(s) => Some(s),
            None => anyhow::bail!("{}", crate::i18n::tr("tui-resume-not-found")),
        }
    } else {
        None
    };

    let mut agent_type = if let Some(ref s) = snap_loaded {
        anycode_core::AgentType::new(s.agent.clone())
    } else {
        anycode_core::AgentType::new(agent.clone())
    };

    let messages = if let Some(ref s) = snap_loaded {
        if s.workspace_root != working_dir_str {
            tracing::warn!("{}", crate::i18n::tr("tui-resume-cwd-warn"));
        }
        Arc::new(Mutex::new(s.messages.clone()))
    } else {
        Arc::new(Mutex::new(
            runtime
                .build_session_messages(&agent_type, &working_dir_str)
                .await?,
        ))
    };

    let mut session_uuid = snap_loaded
        .as_ref()
        .map(|s| s.id)
        .unwrap_or_else(Uuid::new_v4);

    let mut transcript: Vec<TranscriptEntry> = vec![];
    let mut transcript_gen: u64 = 0;
    let workspace_line_count = Cell::new(0usize);
    let mut workspace_cache_lines: Vec<Line<'static>> = Vec::new();
    let mut workspace_cache_gen: u64 = 0;
    let mut workspace_cache_w: usize = 0;
    let mut workspace_cache_fold_rev: u64 = 0;
    let mut workspace_cache_executing: bool = false;
    let mut workspace_cache_working_secs: Option<u64> = None;
    let mut workspace_cache_key: Option<WorkspaceCacheKey> = None;
    let mut fold_layout_rev: u64 = 0;
    let mut tool_folds_expanded: HashSet<u64> = HashSet::new();
    let mut next_tool_fold_id: u64 = 0;
    let mut resize_debounce = ResizeDebounce::new();
    if snap_loaded.is_some() {
        let frozen = messages.lock().await.clone();
        super::exec_completion::rebuild_transcript_from_messages(
            &mut transcript,
            &frozen,
            &mut transcript_gen,
            &mut next_tool_fold_id,
        );
        apply_tool_transcript_pipeline(&mut transcript, &mut next_tool_fold_id);
    }
    let mut input = InputState::default();
    let mut input_history: Vec<String> = Vec::new();
    let mut history_idx: Option<usize> = None;
    // Workspace 从底部向上滚动的行数（0 = 跟随最新）。
    let mut transcript_scroll_up: usize = 0;
    let mut rev_search: Option<RevSearchState> = None;
    // `/` 命令补全：候选高亮；采纳后 `suppress` 隐藏列表（对齐 Claude `clearSuggestions`）。
    let mut slash_suggest_pick: usize = 0;
    let mut slash_suggest_suppress: bool = false;
    let mut executing = false;
    let mut executing_since: Option<Instant> = None;
    let mut last_turn_error: Option<String> = None;
    let mut last_key: Option<String> = None;
    let mut help_open = false;

    let mut pending_approval: Option<PendingApproval> = None;
    let mut pending_user_question: Option<PendingUserQuestion> = None;
    // 审批三选项菜单高亮（↑↓ / Enter；y/p/n 仍为快捷键）。
    let mut approval_menu_selected: usize = 0;
    let mut user_question_menu_selected: usize = 0;
    let mut exec_handle: Option<JoinHandle<anyhow::Result<anycode_core::TurnOutput>>> = None;
    let mut exec_prev_len: usize = messages.lock().await.len();
    let mut last_max_input_tokens: u32 = 0;
    let mut exec_live_tail: Option<(usize, u64)> = None;
    // (messages.len(), last_message_id)：有变化时才重排 transcript，避免每帧整表重算。
    let mut exec_live_sync_fp: Option<(usize, Option<Uuid>)> = None;
    let mut compact_handle: Option<JoinHandle<anyhow::Result<(Vec<Message>, Usage)>>> = None;
    let mut compact_followup: Option<CompactFollowup> = None;

    let status_line_cfg = config.status_line.clone();
    let session_for_sl = config.session.clone();
    let mut tui_session_id = session_uuid.to_string();
    let mut quit_confirm = false;
    let mut status_line_text = String::from(" ");
    let mut status_line_fire_at: Option<Instant> = None;
    let mut status_line_task: Option<JoinHandle<anyhow::Result<String>>> = None;
    let mut last_sl_transcript_gen: u64 = u64::MAX;
    let mut prev_executing_sl = false;
    let mut last_turn_usage: Option<Usage> = None;

    let tui_started_at = Instant::now();

    let _tui_guard = super::terminal_guard::TuiTerminalGuard::enter(config.tui.alternate_screen)?;
    let used_alternate_screen = _tui_guard.used_alternate_screen();
    debug!(
        alt = if used_alternate_screen { "on" } else { "off" },
        "TUI alternate screen"
    );

    // 主缓冲 + 默认不清屏：用 Inline 视口锚在 shell 光标下，避免 MoveTo(0,0)+Fullscreen 把 UI 画在屏顶、与下方 shell 历史叠成「重复底栏」。
    // 主缓冲 + CLEAR_ON_START=1：首帧 Clear(All) 后仍用全屏视口（与备用屏一致的全幅矩阵）。
    let backend = CrosstermBackend::new(stdout());
    let initial_term_size = backend.size()?;
    if !used_alternate_screen && super::terminal_guard::tui_main_buffer_clear_all_on_start() {
        use crossterm::{
            cursor::MoveTo,
            execute,
            terminal::{Clear, ClearType},
        };
        let mut out = stdout();
        let _ = execute!(out, Clear(ClearType::All), MoveTo(0, 0));
    }
    let mut terminal = if used_alternate_screen {
        Terminal::new(backend)?
    } else if super::terminal_guard::tui_main_buffer_clear_all_on_start() {
        Terminal::new(backend)?
    } else {
        Terminal::with_options(
            backend,
            super::TerminalOptions {
                viewport: super::Viewport::Inline(initial_term_size.height.max(1)),
            },
        )?
    };

    'ui: loop {
        let size = terminal.size()?;
        let show_buddy = size.width >= 52;
        let main_avail_cell = Cell::new(0usize);
        let nl_extra = input.as_string().matches('\n').count().min(12);
        // 内置 token 行已并入脚标，避免与 `statusLine.show_builtin` 重复占行。
        let status_line_show_draw = status_line_cfg.command.is_some();
        let status_rows: u16 = if status_line_show_draw { 1 } else { 0 };
        let working_elapsed_secs_pre = if executing {
            executing_since.map(|t| t.elapsed().as_secs())
        } else {
            None
        };
        let context_window_tokens = effective_session_context_window_tokens(
            &session_for_sl,
            llm_provider.as_str(),
            llm_model.as_str(),
        );
        let last_output_tokens = last_turn_usage
            .as_ref()
            .map(|u| u.output_tokens)
            .unwrap_or(0);
        let footer_inp = super::draw::FooterLayoutInput {
            permission_mode: permission_mode.as_str(),
            require_approval,
            llm_provider: llm_provider.as_str(),
            llm_model: llm_model.as_str(),
            transcript_scroll_up,
            debug,
            last_key: last_key.as_deref(),
            context_window_tokens,
            last_max_input_tokens,
            last_output_tokens,
        };
        let footer_h = super::draw::footer_wrapped_line_count(size.width, &footer_inp);
        // 底栏：横线 + 可选 status + Claude HUD（空闲 0 行）+ 横线 + Min(dock) + 横线 + 折行脚标。
        let hud_rows_effective: u16 =
            if pending_approval.is_some() || pending_user_question.is_some() || executing {
                2
            } else {
                0
            };
        // 与 `draw` 一致：无 status 且无 HUD 时合并顶横线与 Prompt 上横线，底栏总高少 1 行。
        let compact_bottom_chrome = status_rows == 0 && hud_rows_effective == 0;
        let merge_rule_rows: u16 = u16::from(compact_bottom_chrome);
        // 主缓冲空闲（含短对话后静止）：Dock 1 行即可；执行中 / 审批 / rev-search / 退出确认等仍放宽。
        let dock_need: u16 = if pending_approval.is_some() || pending_user_question.is_some() {
            17
        } else if !used_alternate_screen && rev_search.is_none() && !quit_confirm && !executing {
            1
        } else {
            4
        };
        let mut bottom_h: u16 = 1 + status_rows + hud_rows_effective + 1 + dock_need + 1 + footer_h;
        bottom_h = bottom_h.saturating_sub(merge_rule_rows);
        let min_slash_rev = 1 + status_rows + hud_rows_effective + 1 + 13 + 1 + footer_h;
        let min_slash_rev = min_slash_rev.saturating_sub(merge_rule_rows);
        if rev_search.is_some() {
            bottom_h = bottom_h.max(min_slash_rev);
        } else if pending_approval.is_none()
            && pending_user_question.is_none()
            && !slash_suggest_suppress
            && !crate::slash_commands::slash_suggestions_for_first_line(&input.as_string())
                .is_empty()
        {
            bottom_h = bottom_h.max(min_slash_rev);
        }
        bottom_h = bottom_h.saturating_add(nl_extra as u16);
        let max_bottom = size.height.saturating_sub(2);
        bottom_h = bottom_h.min(max_bottom);
        let mut min_bottom = 1 + status_rows + hud_rows_effective + 1 + dock_need + 1 + footer_h;
        min_bottom = min_bottom.saturating_sub(merge_rule_rows);
        if max_bottom >= min_bottom {
            bottom_h = bottom_h.max(min_bottom);
        } else if max_bottom >= 6 {
            bottom_h = bottom_h.max(6);
        }
        if quit_confirm {
            bottom_h = bottom_h.saturating_add(3);
        }

        let working_elapsed_secs = working_elapsed_secs_pre;

        let pet_anim_frame: u64 = if executing {
            executing_since
                .map(|t| ((t.elapsed().as_millis() / 250) % 4) as u64)
                .unwrap_or(0)
        } else {
            0
        };
        let mut workspace_cache_pulse_frame = pet_anim_frame;

        let hud_tip_slot =
            (tui_started_at.elapsed().as_secs() as usize / 8) % crate::tui::hud_text::HUD_TIP_COUNT;

        // 构建 Workspace 缓存键
        let current_cache_key = WorkspaceCacheKey {
            gen: transcript_gen,
            w: size.width.saturating_sub(2) as usize, // 减去边框
            fold_rev: fold_layout_rev,
            executing,
            working_secs: working_elapsed_secs,
        };

        // 检查是否需要重新渲染 Workspace
        let should_render_workspace = workspace_cache_key
            .as_ref()
            .map(|old_key| old_key.should_invalidate(&current_cache_key))
            .unwrap_or(true);

        // 更新缓存键
        workspace_cache_key = Some(current_cache_key.clone());

        // 同步旧的缓存变量以保持兼容性
        if should_render_workspace {
            workspace_cache_gen = current_cache_key.gen;
            workspace_cache_w = current_cache_key.w;
            workspace_cache_fold_rev = current_cache_key.fold_rev;
            workspace_cache_executing = current_cache_key.executing;
            workspace_cache_working_secs = current_cache_key.working_secs;
        }

        // 检查是否因尺寸变化防抖而跳过渲染
        let current_size = (size.width, size.height);
        if resize_debounce.update(current_size) {
            // 跳过本次渲染，但保持循环运行
            let avail = main_avail_cell.get().max(1);
            let max_sc = workspace_line_count.get().saturating_sub(avail);
            transcript_scroll_up = transcript_scroll_up.min(max_sc);
            continue;
        }

        // 帧率限制：目标 30 FPS，避免过度渲染
        let frame_start = Instant::now();
        const TARGET_FPS: u64 = 30;
        const MIN_FRAME_DURATION: Duration = Duration::from_millis(1000 / TARGET_FPS);

        super::terminal_guard::terminal_draw_with_optional_sync(
            &mut terminal,
            super::terminal_guard::tui_sync_draw_enabled(),
            |f| {
                super::draw::draw_tui_frame(
                    f,
                    super::draw::DrawFrameCtx {
                        bottom_h,
                        footer_line_count: footer_h,
                        show_buddy,
                        pet_anim_frame,
                        agent_type: &agent_type,
                        permission_mode: permission_mode.as_str(),
                        require_approval,
                        llm_provider: llm_provider.as_str(),
                        llm_model: llm_model.as_str(),
                        debug,
                        last_key: last_key.as_deref(),
                        pending_approval: pending_approval.as_ref(),
                        pending_user_question: pending_user_question.as_ref(),
                        approval_menu_selected,
                        user_question_menu_selected,
                        executing,
                        working_elapsed_secs,
                        help_open,
                        transcript: &transcript,
                        transcript_scroll_up,
                        rev_search: rev_search.as_ref(),
                        slash_suggest_pick,
                        slash_suggest_suppress,
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
                        status_line_show: status_line_show_draw,
                        status_line_text: status_line_text.as_str(),
                        status_line_padding: status_line_cfg.padding,
                        quit_confirm_pending: quit_confirm,
                        used_alternate_screen,
                        tui_resume_session_id: tui_session_id.as_str(),
                        hud_tip_slot,
                        last_max_input_tokens,
                        last_output_tokens,
                        context_window_tokens,
                    },
                );
            },
        )?;

        // 帧率节流：如果渲染过快，sleep 到最小帧间隔
        let frame_elapsed = frame_start.elapsed();
        if frame_elapsed < MIN_FRAME_DURATION {
            tokio::time::sleep(MIN_FRAME_DURATION - frame_elapsed).await;
        }

        let _ = super::terminal_guard::refresh_mouse_capture_after_draw(used_alternate_screen);

        let avail = main_avail_cell.get().max(1);
        let max_sc = workspace_line_count.get().saturating_sub(avail);
        transcript_scroll_up = transcript_scroll_up.min(max_sc);

        // 勿使用 `biased`：原先 `biased` + `approval_rx.recv()` 在前时，若 `recv` 与定时器同时就绪会优先
        // `recv`；在 channel 异常等情况下可能反复选中 recv，饿死定时分支 → 永不 poll 键盘/鼠标。
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(16)) => {

                // --- Status line (Claude-style HUD): debounce + optional `sh -c` command ---
                if let Some(h) = status_line_task.as_ref() {
                    if h.is_finished() {
                        let h = status_line_task.take().unwrap();
                        if let Ok(Ok(s)) = h.await {
                            status_line_text = s;
                        }
                    }
                }
                let sl_enabled = status_line_cfg.command.is_some();
                if sl_enabled {
                    if crate::tui::status_line::status_line_arm_refresh(
                        &mut last_sl_transcript_gen,
                        transcript_gen,
                        prev_executing_sl,
                        executing,
                    ) {
                        status_line_fire_at =
                            Some(Instant::now() + crate::tui::status_line::debounce_std());
                    }
                    if let Some(fire_at) = status_line_fire_at {
                        if Instant::now() >= fire_at {
                            status_line_fire_at = None;
                            if let Some(h) = status_line_task.take() {
                                h.abort();
                            }
                            if let Some(ref cmd) = status_line_cfg.command {
                                match crate::tui::status_line::build_status_line_payload(
                                    env!("CARGO_PKG_VERSION"),
                                    &tui_session_id,
                                    &working_dir_str,
                                    &working_dir_str,
                                    llm_model.as_str(),
                                    &session_for_sl,
                                    llm_provider.as_str(),
                                    last_max_input_tokens,
                                    last_turn_usage.as_ref(),
                                ) {
                                    Ok(json) if !json.is_empty() => {
                                        let c = cmd.clone();
                                        let t = status_line_cfg.timeout_ms;
                                        status_line_task = Some(
                                            crate::tui::status_line::spawn_status_line_task(
                                                async move {
                                                    crate::tui::status_line::run_status_line_command(
                                                        &c, &json, t,
                                                    )
                                                    .await
                                                },
                                            ),
                                        );
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                prev_executing_sl = executing;


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
                            &mut last_turn_usage,
                            anchor,
                        )
                        .await;
                        crate::tui::tui_session_persist::spawn_persist_tui_session(
                            session_uuid,
                            working_dir_str.clone(),
                            agent_type.as_str().to_string(),
                            llm_model.clone(),
                            messages.clone(),
                        );
                    }
                }

                if let Some(h) = compact_handle.as_ref() {
                    if h.is_finished() {
                        let h = compact_handle.take().unwrap();
                        if let Some(follow) = compact_followup.take() {
                            let new_turn = consume_finished_compact(
                                h,
                                follow,
                                &messages,
                                &mut transcript,
                                &mut transcript_gen,
                                &mut next_tool_fold_id,
                                &mut tool_folds_expanded,
                                &mut fold_layout_rev,
                                &mut exec_live_tail,
                                &mut exec_prev_len,
                                &mut last_turn_error,
                                &mut last_max_input_tokens,
                                &mut last_turn_usage,
                                &mut transcript_scroll_up,
                                &runtime,
                                &agent_type,
                                &working_dir_str,
                            )
                            .await;
                            crate::tui::tui_session_persist::spawn_persist_tui_session(
                                session_uuid,
                                working_dir_str.clone(),
                                agent_type.as_str().to_string(),
                                llm_model.clone(),
                                messages.clone(),
                            );
                            exec_live_sync_fp = None;
                            match new_turn {
                                Some(eh) => {
                                    exec_handle = Some(eh);
                                    executing = true;
                                    executing_since = Some(Instant::now());
                                }
                                None => {
                                    executing = false;
                                    executing_since = None;
                                }
                            }
                        } else {
                            executing = false;
                            executing_since = None;
                            exec_live_sync_fp = None;
                        }
                    }
                }

                while ct_event::poll(Duration::ZERO)? {
                    let ev = ct_event::read()?;
                    let mut ectx = super::event::TuiEventCtx {
                        last_key: &mut last_key,
                        transcript_scroll_up: &mut transcript_scroll_up,
                        pending_approval: &mut pending_approval,
                        pending_user_question: &mut pending_user_question,
                        approval_menu_selected: &mut approval_menu_selected,
                        user_question_menu_selected: &mut user_question_menu_selected,
                        rev_search: &mut rev_search,
                        slash_suggest_pick: &mut slash_suggest_pick,
                        slash_suggest_suppress: &mut slash_suggest_suppress,
                        input: &mut input,
                        input_history: &mut input_history,
                        history_idx: &mut history_idx,
                        executing: &mut executing,
                        executing_since: &mut executing_since,
                        help_open: &mut help_open,
                        transcript: &mut transcript,
                        transcript_gen: &mut transcript_gen,
                        last_turn_error: &mut last_turn_error,
                        compact_handle: &mut compact_handle,
                        compact_followup: &mut compact_followup,
                        exec_handle: &mut exec_handle,
                        exec_prev_len: &mut exec_prev_len,
                        last_max_input_tokens: &mut last_max_input_tokens,
                        last_turn_usage: &mut last_turn_usage,
                        session_cfg: &config.session,
                        default_mode: config.runtime.default_mode.as_str(),
                        permission_mode: permission_mode.as_str(),
                        require_approval,
                        llm_plan: llm_plan.as_str(),
                        llm_provider: llm_provider.as_str(),
                        llm_model: llm_model.as_str(),
                        memory_backend: config.memory.backend.as_str(),
                        workspace_project_label: config.runtime.workspace_project_label.as_deref(),
                        workspace_channel_profile: config
                            .runtime
                            .workspace_channel_profile
                            .as_deref(),
                        main_avail_cell: &main_avail_cell,
                        workspace_line_count: &workspace_line_count,
                        tool_folds_expanded: &mut tool_folds_expanded,
                        fold_layout_rev: &mut fold_layout_rev,
                        next_tool_fold_id: &mut next_tool_fold_id,
                        exec_live_tail: &mut exec_live_tail,
                        quit_confirm: &mut quit_confirm,
                        session_file_id: &mut session_uuid,
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
                        super::event::TuiLoopCtl::ResumeSession(id) => {
                            apply_tui_resume_snapshot(
                                id,
                                &working_dir_str,
                                &mut agent_type,
                                &messages,
                                &mut transcript,
                                &mut transcript_gen,
                                &mut next_tool_fold_id,
                                &mut tool_folds_expanded,
                                &mut fold_layout_rev,
                                &mut session_uuid,
                                &mut tui_session_id,
                                &mut exec_prev_len,
                                &mut transcript_scroll_up,
                                &mut exec_live_tail,
                                &mut exec_live_sync_fp,
                                &mut last_turn_error,
                            )
                            .await?;
                        }
                    }
                }
            }
            req = approval_rx.recv() => {
                if let Some(r) = req {
                    approval_menu_selected = 0;
                    if let Some(old) = pending_approval.replace(r) {
                        let _ = old.reply.send(crate::tui::approval::ApprovalDecision::Deny);
                    }
                }
            }
            uqr = uq_rx.recv() => {
                if let Some(r) = uqr {
                    user_question_menu_selected = 0;
                    if let Some(old) = pending_user_question.replace(r) {
                        let _ = old.reply.send(Err(()));
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
    // 备用屏下会话不在主缓冲：退出后可选 echo 一份纯文本供 shell 搜索；主缓冲模式会话已在屏上，不再重复打印。
    if used_alternate_screen && !skip_dump && !transcript.is_empty() {
        println!(
            "{}",
            crate::tui::transcript::transcript_dump_plain_text(&transcript)
        );
    }

    let frozen = messages.lock().await.clone();
    let snap = crate::tui::tui_session_persist::TuiSessionSnapshot {
        version: 1,
        id: session_uuid,
        workspace_root: working_dir_str.clone(),
        agent: agent_type.as_str().to_string(),
        model: llm_model.clone(),
        messages: frozen,
    };
    if let Err(e) = crate::tui::tui_session_persist::save_tui_session(&snap) {
        tracing::warn!(target: "anycode_cli", "tui session final save: {e:#}");
    }
    println!(
        "\n{} anycode --resume {session_uuid}",
        crate::i18n::tr("tui-exit-resume-print")
    );

    Ok(())
}
