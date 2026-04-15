//! crossterm 事件分发（与 ratatui 绘制、exec 轮询解耦）。

use super::exec_completion::{append_user_line_and_spawn_turn, CompactFollowup};
use crate::app_config::{
    effective_session_context_window_tokens, should_auto_compact_before_send, SessionConfig,
};
use crate::builtin_agents::parse_agent_slash_command;
use crate::i18n::{tr, tr_args};
use crate::repl::stream_repl_accept_key_event;
use crate::slash_commands;
use crate::tui::approval::{ApprovalDecision, PendingApproval};
use crate::tui::chrome::{agents_lines, tools_lines};
use crate::tui::input::{history_apply_down, history_apply_up, InputState, RevSearchState};
use crate::tui::styles::*;
use crate::tui::transcript::{ctrl_o_fold_cycle, TranscriptEntry};
use crate::tui::util::{sanitize_paste, trim_or_default, MAX_PASTE_CHARS};
use crate::tui::PendingUserQuestion;
use anycode_agent::AgentRuntime;
use anycode_core::{Message, RuntimeMode, TurnOutput, Usage};
use anycode_tools::workflows;
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};
use fluent_bundle::FluentArgs;
use ratatui::text::{Line, Span};
use std::cell::Cell;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use uuid::Uuid;
/// 主 `loop` 内对 `continue` / `break` 的显式结果。
pub(super) enum TuiLoopCtl {
    Ok,
    Continue,
    Break,
    /// 同进程加载 `~/.anycode/tui-sessions/<id>.json`（由主循环应用快照）。
    ResumeSession(Uuid),
}

fn reset_slash_state(ctx: &mut TuiEventCtx<'_>) {
    *ctx.slash_suggest_pick = 0;
    *ctx.slash_suggest_suppress = false;
}

async fn rebuild_session_messages(
    runtime: &Arc<AgentRuntime>,
    messages: &Arc<Mutex<Vec<Message>>>,
    agent_type: &anycode_core::AgentType,
    working_dir_str: &str,
) -> anyhow::Result<()> {
    let fresh = runtime
        .build_session_messages(agent_type, working_dir_str)
        .await?;
    let mut g = messages.lock().await;
    *g = fresh;
    Ok(())
}

fn reset_transcript_state(ctx: &mut TuiEventCtx<'_>) {
    ctx.transcript.clear();
    *ctx.transcript_gen = ctx.transcript_gen.wrapping_add(1);
    *ctx.transcript_scroll_up = 0;
    ctx.tool_folds_expanded.clear();
    *ctx.next_tool_fold_id = 0;
    *ctx.exec_live_tail = None;
    *ctx.fold_layout_rev = ctx.fold_layout_rev.wrapping_add(1);
    *ctx.last_turn_error = None;
    *ctx.help_open = false;
    *ctx.rev_search = None;
    *ctx.executing_since = None;
    *ctx.last_max_input_tokens = 0;
    *ctx.last_turn_usage = None;
}

fn cursor_on_first_line(input: &InputState) -> bool {
    !input.chars[..input.cursor].contains(&'\n')
}

fn slash_suggestions_for_ctx(ctx: &TuiEventCtx<'_>) -> Vec<slash_commands::SlashSuggestionItem> {
    if *ctx.slash_suggest_suppress {
        return Vec::new();
    }
    slash_commands::slash_suggestions_for_first_line(&ctx.input.as_string())
}

fn apply_slash_pick_to_input(ctx: &mut TuiEventCtx<'_>) {
    // 始终基于当前缓冲算候选；不因 `slash_suggest_suppress` 跳过（suppress 只影响展示）。
    let cands = slash_commands::slash_suggestions_for_first_line(&ctx.input.as_string());
    if cands.is_empty() {
        return;
    }
    let len = cands.len();
    let pick = *ctx.slash_suggest_pick % len;
    let new_first = cands[pick].replacement.clone();
    let new_buf = slash_commands::replace_first_line(&ctx.input.as_string(), &new_first);
    ctx.input.set_from_str(&new_buf);
    *ctx.slash_suggest_pick = 0;
    *ctx.history_idx = None;
}

pub(super) struct TuiEventCtx<'a> {
    pub last_key: &'a mut Option<String>,
    /// 工作区向上滚动（与 `append_user_line_and_spawn_turn` 等共用）。
    pub transcript_scroll_up: &'a mut usize,
    pub pending_approval: &'a mut Option<PendingApproval>,
    pub pending_user_question: &'a mut Option<PendingUserQuestion>,
    /// 审批菜单高亮：`0` 允许一次 · `1` 允许（项目） · `2` 拒绝（与 Claude 式 ↑↓ 一致）。
    pub approval_menu_selected: &'a mut usize,
    pub user_question_menu_selected: &'a mut usize,
    pub rev_search: &'a mut Option<RevSearchState>,
    pub slash_suggest_pick: &'a mut usize,
    /// 采纳 Tab/Enter 补全后隐藏下拉，直到用户再次编辑（对齐 Claude `clearSuggestions`）。
    pub slash_suggest_suppress: &'a mut bool,
    pub input: &'a mut InputState,
    pub input_history: &'a mut Vec<String>,
    pub history_idx: &'a mut Option<usize>,
    pub executing: &'a mut bool,
    pub executing_since: &'a mut Option<Instant>,
    pub help_open: &'a mut bool,
    pub transcript: &'a mut Vec<TranscriptEntry>,
    pub transcript_gen: &'a mut u64,
    pub last_turn_error: &'a mut Option<String>,
    /// 后台会话压缩（`/compact` 或发送前自动压缩）；勿在事件里 `.await`，否则 TUI 主循环无法重绘。
    pub compact_handle: &'a mut Option<JoinHandle<anyhow::Result<(Vec<Message>, Usage)>>>,
    pub compact_followup: &'a mut Option<CompactFollowup>,
    pub exec_handle: &'a mut Option<JoinHandle<anyhow::Result<TurnOutput>>>,
    pub exec_prev_len: &'a mut usize,
    pub last_max_input_tokens: &'a mut u32,
    /// 只读引用：上一轮完成后的用量（与 `/context` 展示一致）。
    pub last_turn_usage: &'a mut Option<Usage>,
    pub session_cfg: &'a SessionConfig,
    /// 配置中的 `runtime.default_mode`（与 REPL `/status` 的 `default_mode` 一致）。
    pub default_mode: &'a str,
    pub permission_mode: &'a str,
    pub require_approval: bool,
    pub llm_plan: &'a str,
    pub llm_provider: &'a str,
    pub llm_model: &'a str,
    pub memory_backend: &'a str,
    pub workspace_project_label: Option<&'a str>,
    pub workspace_channel_profile: Option<&'a str>,
    pub main_avail_cell: &'a Cell<usize>,
    pub workspace_line_count: &'a Cell<usize>,
    /// `ToolTurn` 折叠：已展开块的 `fold_id`（Ctrl+O 循环展开）。
    pub tool_folds_expanded: &'a mut HashSet<u64>,
    pub fold_layout_rev: &'a mut u64,
    pub next_tool_fold_id: &'a mut u64,
    /// 当前 turn 的 transcript 尾部锚点（`transcript.len()` 在 user 行之后、`next_tool_fold_id` 快照），供主循环增量同步。
    pub exec_live_tail: &'a mut Option<(usize, u64)>,
    /// 首次 Ctrl+C 已按下，再按一次则退出（对齐 Claude Code）。
    pub quit_confirm: &'a mut bool,
    /// 与 `execute_turn_from_messages` 共享：在 agent 回合进行中按 Ctrl+C 置位以协作结束本轮。
    pub turn_coop_cancel: &'a Arc<AtomicBool>,
    /// `~/.anycode/tui-sessions/<id>.json` 当前会话 id（`/export` 默认文件名）。
    pub session_file_id: &'a mut Uuid,
}

pub(super) async fn dispatch_crossterm_event(
    ev: Event,
    ctx: &mut TuiEventCtx<'_>,
    runtime: &Arc<AgentRuntime>,
    messages: &Arc<Mutex<Vec<Message>>>,
    agent_type: &mut anycode_core::AgentType,
    working_dir_str: &str,
) -> anyhow::Result<TuiLoopCtl> {
    match ev {
        Event::Mouse(me) => {
            *ctx.quit_confirm = false;
            *ctx.last_key = Some(format!("mouse {:?}", me.kind));
            let avail = ctx.main_avail_cell.get().max(1);
            let max_sc = ctx.workspace_line_count.get().saturating_sub(avail);
            // 每格滚轮多行，避免「有反应但几乎不动」；上限避免一次跳太多。
            let step = (avail / 5).max(1).min(12);
            match me.kind {
                MouseEventKind::ScrollUp => {
                    *ctx.transcript_scroll_up = (*ctx.transcript_scroll_up + step).min(max_sc);
                }
                MouseEventKind::ScrollDown => {
                    *ctx.transcript_scroll_up = ctx.transcript_scroll_up.saturating_sub(step);
                }
                _ => {}
            }
            Ok(TuiLoopCtl::Ok)
        }
        Event::Paste(text) => {
            *ctx.quit_confirm = false;
            *ctx.last_key = Some(format!("Paste({} chars)", text.chars().count()));
            if ctx.pending_approval.is_some() || ctx.pending_user_question.is_some() {
                return Ok(TuiLoopCtl::Continue);
            }
            let (clean, truncated) = sanitize_paste(text);
            if truncated {
                let mut a = FluentArgs::new();
                a.set("n", MAX_PASTE_CHARS as i64);
                *ctx.last_turn_error = Some(tr_args("tui-err-paste-truncated", &a));
            }
            if let Some(rs) = ctx.rev_search.as_mut() {
                rs.query.insert_str(&clean);
                rs.pick = 0;
            } else {
                ctx.input.insert_str(&clean);
                *ctx.history_idx = None;
                reset_slash_state(ctx);
            }
            if !truncated {
                *ctx.last_turn_error = None;
            }
            Ok(TuiLoopCtl::Ok)
        }
        Event::Key(key) => {
            // Kitty / 增强键盘协议会发 Release；与 stream REPL 一致，勿当作普通键（重复插入或状态错乱）。
            if key.kind == KeyEventKind::Release {
                return Ok(TuiLoopCtl::Continue);
            }
            if !stream_repl_accept_key_event(&key) {
                return Ok(TuiLoopCtl::Continue);
            }
            *ctx.last_key = Some(format!("{:?} {:?}", key.code, key.modifiers));

            if let Some(p) = ctx.pending_user_question.take() {
                *ctx.quit_confirm = false;
                let n = p.option_labels.len().max(1);
                match key.code {
                    KeyCode::Up => {
                        *ctx.user_question_menu_selected =
                            (*ctx.user_question_menu_selected + n - 1) % n;
                        *ctx.pending_user_question = Some(p);
                    }
                    KeyCode::Down => {
                        *ctx.user_question_menu_selected =
                            (*ctx.user_question_menu_selected + 1) % n;
                        *ctx.pending_user_question = Some(p);
                    }
                    KeyCode::Enter => {
                        let i = *ctx.user_question_menu_selected % n;
                        let label = p.option_labels.get(i).cloned().unwrap_or_default();
                        let _ = p.reply.send(Ok(vec![label]));
                    }
                    KeyCode::Esc => {
                        let _ = p.reply.send(Err(()));
                    }
                    _ => {
                        *ctx.pending_user_question = Some(p);
                    }
                }
                return Ok(TuiLoopCtl::Continue);
            }

            if let Some(p) = ctx.pending_approval.take() {
                *ctx.quit_confirm = false;
                match key.code {
                    KeyCode::Up => {
                        *ctx.approval_menu_selected = (*ctx.approval_menu_selected + 2) % 3;
                        *ctx.pending_approval = Some(p);
                    }
                    KeyCode::Down => {
                        *ctx.approval_menu_selected = (*ctx.approval_menu_selected + 1) % 3;
                        *ctx.pending_approval = Some(p);
                    }
                    KeyCode::Enter => {
                        let d = match *ctx.approval_menu_selected {
                            0 => ApprovalDecision::AllowOnce,
                            1 => ApprovalDecision::AllowToolForProject,
                            _ => ApprovalDecision::Deny,
                        };
                        let _ = p.reply.send(d);
                    }
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        let _ = p.reply.send(ApprovalDecision::AllowOnce);
                    }
                    KeyCode::Char('p') | KeyCode::Char('P') => {
                        let _ = p.reply.send(ApprovalDecision::AllowToolForProject);
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        let _ = p.reply.send(ApprovalDecision::Deny);
                    }
                    _ => {
                        *ctx.pending_approval = Some(p);
                    }
                }
                return Ok(TuiLoopCtl::Continue);
            }

            if ctx.rev_search.is_some() {
                return Ok(handle_rev_search_key(key, ctx));
            }

            handle_main_key(key, ctx, runtime, messages, agent_type, working_dir_str).await
        }
        _ => Ok(TuiLoopCtl::Ok),
    }
}

fn handle_rev_search_key(key: crossterm::event::KeyEvent, ctx: &mut TuiEventCtx<'_>) -> TuiLoopCtl {
    *ctx.quit_confirm = false;
    let Some(mut rs) = ctx.rev_search.take() else {
        return TuiLoopCtl::Ok;
    };
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => TuiLoopCtl::Break,
        KeyCode::Esc => TuiLoopCtl::Continue,
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let m = rs.matches(ctx.input_history);
            if !m.is_empty() {
                rs.pick = (rs.pick + 1) % m.len();
            }
            *ctx.rev_search = Some(rs);
            TuiLoopCtl::Continue
        }
        KeyCode::BackTab => {
            let m = rs.matches(ctx.input_history);
            if !m.is_empty() {
                rs.pick = (rs.pick + m.len() - 1) % m.len();
            }
            *ctx.rev_search = Some(rs);
            TuiLoopCtl::Continue
        }
        KeyCode::Tab => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                let m = rs.matches(ctx.input_history);
                if !m.is_empty() {
                    rs.pick = (rs.pick + m.len() - 1) % m.len();
                }
            } else {
                let m = rs.matches(ctx.input_history);
                if !m.is_empty() {
                    rs.pick = (rs.pick + 1) % m.len();
                }
            }
            *ctx.rev_search = Some(rs);
            TuiLoopCtl::Continue
        }
        KeyCode::Enter | KeyCode::Char('\n') | KeyCode::Char('\r') => {
            let m = rs.matches(ctx.input_history);
            if !m.is_empty() {
                let pick = rs.pick % m.len();
                if let Some(f) = m.get(pick) {
                    ctx.input.set_from_str(f);
                    *ctx.history_idx = None;
                    reset_slash_state(ctx);
                }
            }
            TuiLoopCtl::Continue
        }
        KeyCode::Backspace => {
            rs.query.backspace();
            rs.pick = 0;
            *ctx.rev_search = Some(rs);
            TuiLoopCtl::Continue
        }
        KeyCode::Left => {
            rs.query.move_left();
            *ctx.rev_search = Some(rs);
            TuiLoopCtl::Continue
        }
        KeyCode::Right => {
            rs.query.move_right();
            *ctx.rev_search = Some(rs);
            TuiLoopCtl::Continue
        }
        KeyCode::Home => {
            rs.query.move_home();
            *ctx.rev_search = Some(rs);
            TuiLoopCtl::Continue
        }
        KeyCode::End => {
            rs.query.move_end();
            *ctx.rev_search = Some(rs);
            TuiLoopCtl::Continue
        }
        KeyCode::Delete => {
            rs.query.delete_forward();
            rs.pick = 0;
            *ctx.rev_search = Some(rs);
            TuiLoopCtl::Continue
        }
        KeyCode::Char(c) => {
            if !c.is_control() {
                rs.query.insert(c);
                rs.pick = 0;
            }
            *ctx.rev_search = Some(rs);
            TuiLoopCtl::Continue
        }
        _ => {
            *ctx.rev_search = Some(rs);
            TuiLoopCtl::Continue
        }
    }
}

async fn handle_main_key(
    key: crossterm::event::KeyEvent,
    ctx: &mut TuiEventCtx<'_>,
    runtime: &Arc<AgentRuntime>,
    messages: &Arc<Mutex<Vec<Message>>>,
    agent_type: &mut anycode_core::AgentType,
    working_dir_str: &str,
) -> anyhow::Result<TuiLoopCtl> {
    if matches!(
        key.code,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL)
    ) {
        if *ctx.quit_confirm {
            return Ok(TuiLoopCtl::Break);
        }
        if ctx.exec_handle.is_some() {
            ctx.turn_coop_cancel.store(true, Ordering::Release);
            return Ok(TuiLoopCtl::Ok);
        }
        *ctx.quit_confirm = true;
        return Ok(TuiLoopCtl::Ok);
    }
    *ctx.quit_confirm = false;

    match key.code {
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if *ctx.executing {
                return Ok(TuiLoopCtl::Continue);
            }
            if ctx.rev_search.is_none() {
                *ctx.rev_search = Some(RevSearchState::default());
            }
            Ok(TuiLoopCtl::Ok)
        }
        // Workspace 滚动：帮助/欢迎文案为 PgUp/PgDn（无 Ctrl）；保留 Ctrl+PgUp/PgDn 以兼容旧习惯。
        KeyCode::PageUp => {
            let avail = ctx.main_avail_cell.get().max(10);
            let max_sc = ctx.workspace_line_count.get().saturating_sub(avail);
            let page = (avail / 2).max(4);
            *ctx.transcript_scroll_up = (*ctx.transcript_scroll_up + page).min(max_sc);
            Ok(TuiLoopCtl::Ok)
        }
        KeyCode::PageDown => {
            let avail = ctx.main_avail_cell.get().max(10);
            let page = (avail / 2).max(4);
            *ctx.transcript_scroll_up = ctx.transcript_scroll_up.saturating_sub(page);
            Ok(TuiLoopCtl::Ok)
        }
        KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if *ctx.executing {
                *ctx.last_turn_error = Some(tr("tui-err-clear-during-task"));
                return Ok(TuiLoopCtl::Continue);
            }
            rebuild_session_messages(runtime, messages, agent_type, working_dir_str).await?;
            reset_transcript_state(ctx);
            ctx.input.clear();
            *ctx.history_idx = None;
            reset_slash_state(ctx);
            Ok(TuiLoopCtl::Ok)
        }
        KeyCode::Char('o') | KeyCode::Char('O')
            if key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            ctrl_o_fold_cycle(ctx.transcript, ctx.tool_folds_expanded);
            *ctx.fold_layout_rev = ctx.fold_layout_rev.wrapping_add(1);
            Ok(TuiLoopCtl::Ok)
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if ctx.rev_search.is_none() {
                ctx.input.clear();
                *ctx.history_idx = None;
                reset_slash_state(ctx);
            }
            Ok(TuiLoopCtl::Ok)
        }
        KeyCode::Char('?') => {
            *ctx.help_open = !*ctx.help_open;
            Ok(TuiLoopCtl::Ok)
        }
        KeyCode::Esc => {
            if *ctx.quit_confirm {
                *ctx.quit_confirm = false;
                return Ok(TuiLoopCtl::Ok);
            }
            if *ctx.help_open {
                *ctx.help_open = false;
                return Ok(TuiLoopCtl::Ok);
            }
            let cands = slash_commands::slash_suggestions_for_first_line(&ctx.input.as_string());
            if !cands.is_empty() && cursor_on_first_line(ctx.input) {
                *ctx.slash_suggest_suppress = true;
                return Ok(TuiLoopCtl::Ok);
            }
            if ctx.input.is_empty() {
                if ctx.last_turn_error.is_some() {
                    *ctx.last_turn_error = None;
                } else {
                    return Ok(TuiLoopCtl::Break);
                }
            }
            // 不按 Esc 清空输入：中文 IME 常用 Esc 处理候选。
            Ok(TuiLoopCtl::Ok)
        }
        KeyCode::Up if key.modifiers.contains(KeyModifiers::ALT) => {
            if ctx.rev_search.is_none() {
                ctx.input.move_line_up();
                *ctx.history_idx = None;
                reset_slash_state(ctx);
            }
            Ok(TuiLoopCtl::Ok)
        }
        KeyCode::Down if key.modifiers.contains(KeyModifiers::ALT) => {
            if ctx.rev_search.is_none() {
                ctx.input.move_line_down();
                *ctx.history_idx = None;
                reset_slash_state(ctx);
            }
            Ok(TuiLoopCtl::Ok)
        }
        KeyCode::Up => {
            if ctx.rev_search.is_none() {
                let cands = slash_suggestions_for_ctx(ctx);
                if !cands.is_empty() && cursor_on_first_line(ctx.input) {
                    let len = cands.len();
                    *ctx.slash_suggest_pick = (*ctx.slash_suggest_pick + len - 1) % len;
                    *ctx.history_idx = None;
                    return Ok(TuiLoopCtl::Ok);
                }
                history_apply_up(ctx.input_history, ctx.history_idx, ctx.input);
                reset_slash_state(ctx);
            }
            Ok(TuiLoopCtl::Ok)
        }
        KeyCode::Down => {
            if ctx.rev_search.is_none() {
                let cands = slash_suggestions_for_ctx(ctx);
                if !cands.is_empty() && cursor_on_first_line(ctx.input) {
                    let len = cands.len();
                    *ctx.slash_suggest_pick = (*ctx.slash_suggest_pick + 1) % len;
                    *ctx.history_idx = None;
                    return Ok(TuiLoopCtl::Ok);
                }
                history_apply_down(ctx.input_history, ctx.history_idx, ctx.input);
                reset_slash_state(ctx);
            }
            Ok(TuiLoopCtl::Ok)
        }
        KeyCode::Left if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if ctx.rev_search.is_none() {
                ctx.input.move_word_left();
                *ctx.history_idx = None;
            }
            Ok(TuiLoopCtl::Ok)
        }
        KeyCode::Right if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if ctx.rev_search.is_none() {
                ctx.input.move_word_right();
                *ctx.history_idx = None;
            }
            Ok(TuiLoopCtl::Ok)
        }
        KeyCode::Left => {
            if ctx.rev_search.is_none() {
                ctx.input.move_left();
            }
            Ok(TuiLoopCtl::Ok)
        }
        KeyCode::Right => {
            if ctx.rev_search.is_none() {
                ctx.input.move_right();
            }
            Ok(TuiLoopCtl::Ok)
        }
        KeyCode::Home => {
            if ctx.rev_search.is_none()
                && ctx.pending_approval.is_none()
                && ctx.pending_user_question.is_none()
            {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    let avail = ctx.main_avail_cell.get().max(10);
                    let max_sc = ctx.workspace_line_count.get().saturating_sub(avail);
                    *ctx.transcript_scroll_up = max_sc;
                } else {
                    ctx.input.move_home();
                }
            }
            Ok(TuiLoopCtl::Ok)
        }
        KeyCode::End => {
            if ctx.rev_search.is_none()
                && ctx.pending_approval.is_none()
                && ctx.pending_user_question.is_none()
            {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    *ctx.transcript_scroll_up = 0;
                } else {
                    ctx.input.move_end();
                }
            }
            Ok(TuiLoopCtl::Ok)
        }
        KeyCode::Delete => {
            if ctx.rev_search.is_none() {
                ctx.input.delete_forward();
                reset_slash_state(ctx);
            }
            Ok(TuiLoopCtl::Ok)
        }
        KeyCode::Backspace if key.modifiers.contains(KeyModifiers::ALT) => {
            if ctx.rev_search.is_none() {
                ctx.input.delete_word_backward();
                *ctx.history_idx = None;
                reset_slash_state(ctx);
            }
            Ok(TuiLoopCtl::Ok)
        }
        KeyCode::Backspace => {
            if ctx.rev_search.is_none() {
                ctx.input.backspace();
                reset_slash_state(ctx);
            }
            Ok(TuiLoopCtl::Ok)
        }
        KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if ctx.rev_search.is_none() {
                ctx.input.delete_word_backward();
                *ctx.history_idx = None;
                reset_slash_state(ctx);
            }
            Ok(TuiLoopCtl::Ok)
        }
        KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if ctx.rev_search.is_none() {
                ctx.input.delete_to_end_of_line();
                *ctx.history_idx = None;
                reset_slash_state(ctx);
            }
            Ok(TuiLoopCtl::Ok)
        }
        KeyCode::BackTab => {
            if ctx.rev_search.is_some() {
                return Ok(TuiLoopCtl::Ok);
            }
            let cands = slash_suggestions_for_ctx(ctx);
            if !cands.is_empty() && cursor_on_first_line(ctx.input) {
                let len = cands.len();
                *ctx.slash_suggest_pick = (*ctx.slash_suggest_pick + len - 1) % len;
            }
            Ok(TuiLoopCtl::Ok)
        }
        KeyCode::Tab => {
            if ctx.rev_search.is_some() {
                return Ok(TuiLoopCtl::Ok);
            }
            let cands = slash_suggestions_for_ctx(ctx);
            if !cands.is_empty() && cursor_on_first_line(ctx.input) {
                apply_slash_pick_to_input(ctx);
                *ctx.slash_suggest_suppress = true;
                return Ok(TuiLoopCtl::Ok);
            }
            for c in "    ".chars() {
                ctx.input.insert(c);
            }
            *ctx.history_idx = None;
            reset_slash_state(ctx);
            Ok(TuiLoopCtl::Ok)
        }
        KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
            if *ctx.help_open
                || ctx.pending_approval.is_some()
                || ctx.pending_user_question.is_some()
            {
                return Ok(TuiLoopCtl::Continue);
            }
            *ctx.last_turn_error = None;
            ctx.input.insert('\n');
            *ctx.history_idx = None;
            reset_slash_state(ctx);
            Ok(TuiLoopCtl::Continue)
        }
        KeyCode::Enter
        | KeyCode::Char('\n')
        | KeyCode::Char('\r')
        | KeyCode::Char('\u{0085}')
        | KeyCode::Char('\u{2028}')
        | KeyCode::Char('\u{2029}') => {
            if *ctx.help_open {
                *ctx.help_open = false;
                return Ok(TuiLoopCtl::Continue);
            }
            if *ctx.executing
                || ctx.pending_approval.is_some()
                || ctx.pending_user_question.is_some()
            {
                return Ok(TuiLoopCtl::Continue);
            }
            if !slash_suggestions_for_ctx(ctx).is_empty() {
                apply_slash_pick_to_input(ctx);
                *ctx.slash_suggest_suppress = true;
            }
            let trimmed_owned = trim_or_default(&ctx.input.as_string()).to_string();
            ctx.input.clear();
            *ctx.history_idx = None;
            reset_slash_state(ctx);
            if trimmed_owned.is_empty() {
                return Ok(TuiLoopCtl::Continue);
            }
            if trimmed_owned == "/exit" {
                return Ok(TuiLoopCtl::Break);
            }
            if ctx.input_history.last().map(|s| s.as_str()) != Some(trimmed_owned.as_str()) {
                ctx.input_history.push(trimmed_owned.clone());
            }
            let trimmed = trimmed_owned;

            if trimmed == "/help" {
                *ctx.help_open = true;
                return Ok(TuiLoopCtl::Continue);
            }
            if trimmed == "/clear" {
                if *ctx.executing {
                    *ctx.last_turn_error = Some(tr("tui-err-clear-during-task"));
                    return Ok(TuiLoopCtl::Continue);
                }
                rebuild_session_messages(runtime, messages, agent_type, working_dir_str).await?;
                reset_transcript_state(ctx);
                return Ok(TuiLoopCtl::Continue);
            }
            if trimmed.starts_with("/compact") {
                if *ctx.executing
                    || ctx.pending_approval.is_some()
                    || ctx.pending_user_question.is_some()
                {
                    *ctx.last_turn_error = Some(tr("tui-err-compact-during-task"));
                    return Ok(TuiLoopCtl::Continue);
                }
                if ctx.exec_handle.is_some() || ctx.compact_handle.is_some() {
                    *ctx.last_turn_error = Some(tr("tui-err-compact-during-task"));
                    return Ok(TuiLoopCtl::Continue);
                }
                let rest = trimmed["/compact".len()..].trim();
                let custom = if rest.is_empty() {
                    None
                } else {
                    Some(rest.to_string())
                };
                let snap = messages.lock().await.clone();
                if snap.len() < 2 {
                    *ctx.last_turn_error = Some(tr("tui-err-compact-empty"));
                    return Ok(TuiLoopCtl::Continue);
                }
                let rt = runtime.clone();
                let at = agent_type.clone();
                let wd = working_dir_str.to_string();
                *ctx.compact_followup = Some(CompactFollowup::ManualSlash);
                *ctx.compact_handle = Some(tokio::spawn(async move {
                    rt.compact_session_messages(&at, &wd, &snap, custom.as_deref(), false, None)
                        .await
                        .map_err(|e| anyhow::anyhow!("{}", e))
                }));
                *ctx.executing = true;
                *ctx.executing_since = Some(Instant::now());
                *ctx.transcript_scroll_up = 0;
                return Ok(TuiLoopCtl::Continue);
            }
            if trimmed == "/agents" {
                let lines: Vec<Line<'static>> = agents_lines()
                    .into_iter()
                    .map(|l| Line::from(Span::styled(l, style_dim())))
                    .collect();
                ctx.transcript.push(TranscriptEntry::Plain(lines));
                *ctx.transcript_gen = ctx.transcript_gen.wrapping_add(1);
                return Ok(TuiLoopCtl::Continue);
            }
            if trimmed == "/tools" {
                let lines: Vec<Line<'static>> = tools_lines()
                    .into_iter()
                    .map(|l| Line::from(Span::styled(l, style_dim())))
                    .collect();
                ctx.transcript.push(TranscriptEntry::Plain(lines));
                *ctx.transcript_gen = ctx.transcript_gen.wrapping_add(1);
                return Ok(TuiLoopCtl::Continue);
            }

            if let Some(id) = parse_agent_slash_command(trimmed.as_str()) {
                if *ctx.executing {
                    *ctx.last_turn_error = Some(tr("tui-err-switch-agent-during-task"));
                    return Ok(TuiLoopCtl::Continue);
                }
                *agent_type = anycode_core::AgentType::new(id.to_string());
                rebuild_session_messages(runtime, messages, agent_type, working_dir_str).await?;
                let mut ha = FluentArgs::new();
                ha.set("id", id);
                let hint = tr_args("tui-agent-switched", &ha);
                ctx.transcript
                    .push(TranscriptEntry::Plain(vec![Line::from(Span::styled(
                        hint,
                        style_dim(),
                    ))]));
                *ctx.transcript_gen = ctx.transcript_gen.wrapping_add(1);
                *ctx.last_turn_error = None;
                return Ok(TuiLoopCtl::Continue);
            }
            if let Some(cmd) = slash_commands::parse(trimmed.as_str()) {
                match cmd {
                    slash_commands::ParsedSlashCommand::Mode(arg) => {
                        if let Some(mode) = arg {
                            if let Some(parsed) = RuntimeMode::parse(&mode) {
                                *agent_type =
                                    anycode_core::AgentType::new(parsed.default_agent().as_str());
                                rebuild_session_messages(
                                    runtime,
                                    messages,
                                    agent_type,
                                    working_dir_str,
                                )
                                .await?;
                                let hint = format!(
                                    "mode -> {} (agent: {})",
                                    parsed.as_str(),
                                    agent_type.as_str()
                                );
                                ctx.transcript.push(TranscriptEntry::Plain(vec![Line::from(
                                    Span::styled(hint, style_dim()),
                                )]));
                            } else {
                                ctx.transcript.push(TranscriptEntry::Plain(vec![Line::from(
                                    Span::styled(format!("unknown mode: {}", mode), style_dim()),
                                )]));
                            }
                        } else {
                            ctx.transcript.push(TranscriptEntry::Plain(vec![Line::from(
                                Span::styled(
                                    format!("current agent: {}", agent_type.as_str()),
                                    style_dim(),
                                ),
                            )]));
                        }
                        *ctx.transcript_gen = ctx.transcript_gen.wrapping_add(1);
                        return Ok(TuiLoopCtl::Continue);
                    }
                    slash_commands::ParsedSlashCommand::Status => {
                        let mut lines: Vec<Line<'static>> = vec![
                            Line::from(Span::styled(
                                format!("workspace: {}", working_dir_str),
                                style_dim(),
                            )),
                            Line::from(Span::styled(
                                format!("agent: {}", agent_type.as_str()),
                                style_dim(),
                            )),
                            Line::from(Span::styled(
                                format!("default_mode: {}", ctx.default_mode),
                                style_dim(),
                            )),
                            Line::from(Span::styled(
                                format!("provider: {}", ctx.llm_provider),
                                style_dim(),
                            )),
                            Line::from(Span::styled(
                                format!("plan: {}", ctx.llm_plan),
                                style_dim(),
                            )),
                            Line::from(Span::styled(
                                format!("model: {}", ctx.llm_model),
                                style_dim(),
                            )),
                            Line::from(Span::styled(
                                format!(
                                    "permission: {} (interactive_approval: {})",
                                    ctx.permission_mode, ctx.require_approval
                                ),
                                style_dim(),
                            )),
                            Line::from(Span::styled(
                                format!("memory_backend: {}", ctx.memory_backend),
                                style_dim(),
                            )),
                        ];
                        if let Some(lab) = ctx.workspace_project_label {
                            lines.push(Line::from(Span::styled(
                                format!("project_label: {lab}"),
                                style_dim(),
                            )));
                        }
                        if let Some(ch) = ctx.workspace_channel_profile {
                            lines.push(Line::from(Span::styled(
                                format!("channel_profile: {ch}"),
                                style_dim(),
                            )));
                        }
                        let appr = if let Some(p) = ctx.pending_approval.as_ref() {
                            format!("approval: pending — {}", p.tool)
                        } else if ctx.pending_user_question.is_some() {
                            "ask_user_question: pending".to_string()
                        } else {
                            "approval: none".to_string()
                        };
                        lines.push(Line::from(Span::styled(appr, style_dim())));
                        ctx.transcript.push(TranscriptEntry::Plain(lines));
                        *ctx.transcript_gen = ctx.transcript_gen.wrapping_add(1);
                        return Ok(TuiLoopCtl::Continue);
                    }
                    slash_commands::ParsedSlashCommand::Context => {
                        let g = messages.lock().await;
                        let n = g.len();
                        drop(g);
                        let win = effective_session_context_window_tokens(
                            ctx.session_cfg,
                            ctx.llm_provider,
                            ctx.llm_model,
                        );
                        let lines = crate::session_transcript_export::format_context_lines(
                            n,
                            win,
                            *ctx.last_max_input_tokens,
                            ctx.last_turn_usage.as_ref(),
                        );
                        let tlines: Vec<Line<'static>> = lines
                            .into_iter()
                            .map(|s| Line::from(Span::styled(s, style_dim())))
                            .collect();
                        ctx.transcript.push(TranscriptEntry::Plain(tlines));
                        *ctx.transcript_gen = ctx.transcript_gen.wrapping_add(1);
                        return Ok(TuiLoopCtl::Continue);
                    }
                    slash_commands::ParsedSlashCommand::Cost => {
                        let g = messages.lock().await;
                        let n = g.len();
                        drop(g);
                        let win = effective_session_context_window_tokens(
                            ctx.session_cfg,
                            ctx.llm_provider,
                            ctx.llm_model,
                        );
                        let lines = crate::session_transcript_export::format_cost_lines(
                            n,
                            win,
                            *ctx.last_max_input_tokens,
                            ctx.last_turn_usage.as_ref(),
                        );
                        let tlines: Vec<Line<'static>> = lines
                            .into_iter()
                            .map(|s| Line::from(Span::styled(s, style_dim())))
                            .collect();
                        ctx.transcript.push(TranscriptEntry::Plain(tlines));
                        *ctx.transcript_gen = ctx.transcript_gen.wrapping_add(1);
                        return Ok(TuiLoopCtl::Continue);
                    }
                    slash_commands::ParsedSlashCommand::Export(arg) => {
                        let g = messages.lock().await;
                        let msgs = g.clone();
                        drop(g);
                        let text =
                            crate::session_transcript_export::messages_to_plain_export(&msgs);
                        let wd = std::path::PathBuf::from(working_dir_str);
                        let path = match arg.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                            Some(p) => {
                                let pb = std::path::PathBuf::from(p);
                                if pb.is_absolute() {
                                    pb
                                } else {
                                    wd.join(pb)
                                }
                            }
                            None => {
                                let id = ctx.session_file_id.simple().to_string();
                                let short: String = id.chars().take(8).collect();
                                wd.join(format!("anycode-export-{short}.txt"))
                            }
                        };
                        match std::fs::write(&path, text.as_bytes()) {
                            Ok(()) => {
                                let mut a = FluentArgs::new();
                                a.set("path", path.display().to_string());
                                ctx.transcript.push(TranscriptEntry::Plain(vec![Line::from(
                                    Span::styled(tr_args("repl-export-done", &a), style_dim()),
                                )]));
                            }
                            Err(e) => {
                                let mut a = FluentArgs::new();
                                a.set("err", e.to_string());
                                ctx.transcript.push(TranscriptEntry::Plain(vec![Line::from(
                                    Span::styled(
                                        tr_args("repl-export-failed-detail", &a),
                                        style_error(),
                                    ),
                                )]));
                            }
                        }
                        *ctx.transcript_gen = ctx.transcript_gen.wrapping_add(1);
                        return Ok(TuiLoopCtl::Continue);
                    }
                    slash_commands::ParsedSlashCommand::Session(arg) => {
                        if *ctx.executing || ctx.compact_handle.is_some() {
                            *ctx.last_turn_error = Some(tr("tui-err-session-during-task"));
                            return Ok(TuiLoopCtl::Continue);
                        }
                        let dir = crate::tui::tui_session_persist::sessions_dir();
                        match arg.as_deref().map(str::trim) {
                            Some("list") => {
                                match crate::tui::tui_session_persist::list_session_index_entries(
                                    &dir,
                                ) {
                                    Ok(mut rows) => {
                                        #[allow(clippy::unnecessary_sort_by)]
                                        rows.sort_by(|a, b| b.mtime.cmp(&a.mtime));
                                        if rows.is_empty() {
                                            ctx.transcript.push(TranscriptEntry::Plain(vec![
                                                Line::from(Span::styled(
                                                    tr("tui-session-list-empty"),
                                                    style_dim(),
                                                )),
                                            ]));
                                        } else {
                                            let mut lines: Vec<Line<'static>> =
                                                vec![Line::from(Span::styled(
                                                    tr("tui-session-list-title"),
                                                    style_dim(),
                                                ))];
                                            for e in rows.iter().take(40) {
                                                let short = e.id.to_string();
                                                let short = if short.len() > 8 {
                                                    format!("{}…", &short[..8])
                                                } else {
                                                    short
                                                };
                                                lines.push(Line::from(Span::styled(
                                                    format!(
                                                        "{}  {}  {}  {}",
                                                        short, e.workspace_root, e.agent, e.model
                                                    ),
                                                    style_dim(),
                                                )));
                                            }
                                            ctx.transcript.push(TranscriptEntry::Plain(lines));
                                        }
                                    }
                                    Err(e) => {
                                        ctx.transcript.push(TranscriptEntry::Plain(vec![
                                            Line::from(Span::styled(
                                                format!("{} {e}", tr("tui-session-list-err")),
                                                style_dim(),
                                            )),
                                        ]));
                                    }
                                }
                                *ctx.transcript_gen = ctx.transcript_gen.wrapping_add(1);
                                return Ok(TuiLoopCtl::Continue);
                            }
                            Some(rest) if !rest.is_empty() => {
                                let id = match Uuid::parse_str(rest) {
                                    Ok(u) => u,
                                    Err(_) => {
                                        ctx.transcript.push(TranscriptEntry::Plain(vec![
                                            Line::from(Span::styled(
                                                tr("tui-session-bad-uuid"),
                                                style_dim(),
                                            )),
                                        ]));
                                        *ctx.transcript_gen = ctx.transcript_gen.wrapping_add(1);
                                        return Ok(TuiLoopCtl::Continue);
                                    }
                                };
                                if crate::tui::tui_session_persist::load_tui_session(id)?.is_none()
                                {
                                    ctx.transcript.push(TranscriptEntry::Plain(vec![Line::from(
                                        Span::styled(tr("tui-resume-not-found"), style_dim()),
                                    )]));
                                    *ctx.transcript_gen = ctx.transcript_gen.wrapping_add(1);
                                    return Ok(TuiLoopCtl::Continue);
                                }
                                return Ok(TuiLoopCtl::ResumeSession(id));
                            }
                            _ => {
                                match crate::tui::tui_session_persist::resolve_session_for_reopen(
                                    working_dir_str,
                                ) {
                                    Ok(id) => return Ok(TuiLoopCtl::ResumeSession(id)),
                                    Err(_) => {
                                        ctx.transcript.push(TranscriptEntry::Plain(vec![
                                            Line::from(Span::styled(
                                                tr("tui-session-resolve-none"),
                                                style_dim(),
                                            )),
                                        ]));
                                        *ctx.transcript_gen = ctx.transcript_gen.wrapping_add(1);
                                        return Ok(TuiLoopCtl::Continue);
                                    }
                                }
                            }
                        }
                    }
                    slash_commands::ParsedSlashCommand::Workflow(arg) => {
                        let label = if arg.as_deref().map(str::trim) == Some("run") {
                            "workflow: run is available in REPL currently".to_string()
                        } else {
                            match workflows::discover_workflow(std::path::Path::new(
                                working_dir_str,
                            )) {
                                Ok(Some((path, workflow))) => {
                                    format!("workflow: {} ({})", workflow.name, path.display())
                                }
                                Ok(None) => "workflow: none".to_string(),
                                Err(e) => format!("workflow error: {}", e),
                            }
                        };
                        ctx.transcript.push(TranscriptEntry::Plain(vec![Line::from(
                            Span::styled(label, style_dim()),
                        )]));
                        *ctx.transcript_gen = ctx.transcript_gen.wrapping_add(1);
                        return Ok(TuiLoopCtl::Continue);
                    }
                    slash_commands::ParsedSlashCommand::Compact(_) => {
                        // handled earlier by /compact block
                    }
                    slash_commands::ParsedSlashCommand::Clear => {
                        // handled earlier by /clear block
                    }
                    slash_commands::ParsedSlashCommand::Paste => {
                        if let Some(raw) = crate::repl_clipboard::read_system_clipboard() {
                            let (clean, truncated) = sanitize_paste(raw);
                            if truncated {
                                let mut a = FluentArgs::new();
                                a.set("n", MAX_PASTE_CHARS as i64);
                                ctx.transcript.push(TranscriptEntry::Plain(vec![Line::from(
                                    Span::styled(
                                        tr_args("tui-err-paste-truncated", &a),
                                        style_dim(),
                                    ),
                                )]));
                            }
                            ctx.input.insert_str(&clean);
                        } else {
                            ctx.transcript.push(TranscriptEntry::Plain(vec![Line::from(
                                Span::styled(tr("repl-paste-clipboard-failed"), style_dim()),
                            )]));
                        }
                        *ctx.transcript_gen = ctx.transcript_gen.wrapping_add(1);
                        return Ok(TuiLoopCtl::Continue);
                    }
                }
            }

            if should_auto_compact_before_send(
                ctx.session_cfg,
                ctx.llm_provider,
                ctx.llm_model,
                *ctx.last_max_input_tokens,
            ) {
                let snap = messages.lock().await.clone();
                if snap.len() >= 2 {
                    if ctx.exec_handle.is_some() || ctx.compact_handle.is_some() {
                        *ctx.last_turn_error = Some(tr("tui-err-compact-during-task"));
                        return Ok(TuiLoopCtl::Continue);
                    }
                    let rt = runtime.clone();
                    let at = agent_type.clone();
                    let wd = working_dir_str.to_string();
                    let trimmed_after_compact = trimmed.clone();
                    *ctx.compact_followup = Some(CompactFollowup::AutoThenUserTurn {
                        trimmed: trimmed_after_compact,
                    });
                    *ctx.compact_handle = Some(tokio::spawn(async move {
                        rt.compact_session_messages(&at, &wd, &snap, None, true, None)
                            .await
                            .map_err(|e| anyhow::anyhow!("{}", e))
                    }));
                    *ctx.executing = true;
                    *ctx.executing_since = Some(Instant::now());
                    return Ok(TuiLoopCtl::Continue);
                }
            }

            *ctx.last_turn_error = None;
            *ctx.executing = true;
            *ctx.executing_since = Some(Instant::now());
            *ctx.exec_handle = Some(
                append_user_line_and_spawn_turn(
                    trimmed.as_str(),
                    ctx.transcript,
                    ctx.transcript_gen,
                    ctx.transcript_scroll_up,
                    ctx.exec_live_tail,
                    ctx.next_tool_fold_id,
                    ctx.exec_prev_len,
                    runtime,
                    agent_type,
                    messages,
                    working_dir_str,
                    ctx.turn_coop_cancel,
                )
                .await,
            );
            Ok(TuiLoopCtl::Ok)
        }
        KeyCode::Char(c) => {
            if ctx.pending_approval.is_some() || ctx.pending_user_question.is_some() {
                return Ok(TuiLoopCtl::Continue);
            }
            if c.is_control() {
                return Ok(TuiLoopCtl::Continue);
            }
            if ctx.last_turn_error.is_some() {
                *ctx.last_turn_error = None;
            }
            *ctx.history_idx = None;
            ctx.input.insert(c);
            reset_slash_state(ctx);
            Ok(TuiLoopCtl::Ok)
        }
        _ => Ok(TuiLoopCtl::Ok),
    }
}
