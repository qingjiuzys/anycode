//! crossterm 事件分发（与 ratatui 绘制、exec 轮询解耦）。

use super::exec_completion::{append_user_line_and_spawn_turn, CompactFollowup};
use crate::app_config::{should_auto_compact_before_send, SessionConfig};
use crate::builtin_agents::parse_agent_slash_command;
use crate::i18n::{tr, tr_args};
use crate::slash_commands;
use crate::tui::approval::{ApprovalDecision, PendingApproval};
use crate::tui::chrome::{agents_lines, tools_lines};
use crate::tui::input::{history_apply_down, history_apply_up, InputState, RevSearchState};
use crate::tui::styles::*;
use crate::tui::transcript::{ctrl_o_fold_cycle, TranscriptEntry};
use crate::tui::util::{sanitize_paste, trim_or_default, MAX_PASTE_CHARS};
use anycode_agent::AgentRuntime;
use anycode_core::{Artifact, Message, RuntimeMode, Usage};
use anycode_tools::workflows;
use crossterm::event::{Event, KeyCode, KeyModifiers, MouseEventKind};
use fluent_bundle::FluentArgs;
use ratatui::text::{Line, Span};
use std::cell::Cell;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
/// 主 `loop` 内对 `continue` / `break` 的显式结果。
pub(super) enum TuiLoopCtl {
    Ok,
    Continue,
    Break,
}

fn reset_slash_state(ctx: &mut TuiEventCtx<'_>) {
    *ctx.slash_suggest_pick = 0;
    *ctx.slash_suggest_suppress = false;
}

fn cursor_on_first_line(input: &InputState) -> bool {
    !input.chars[..input.cursor].iter().any(|&c| c == '\n')
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
    /// 审批菜单高亮：`0` 允许一次 · `1` 允许（项目） · `2` 拒绝（与 Claude 式 ↑↓ 一致）。
    pub approval_menu_selected: &'a mut usize,
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
    pub exec_handle: &'a mut Option<JoinHandle<anyhow::Result<(String, Vec<Artifact>, u32)>>>,
    pub exec_prev_len: &'a mut usize,
    pub last_max_input_tokens: &'a mut u32,
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
            if ctx.pending_approval.is_some() {
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
            *ctx.last_key = Some(format!("{:?} {:?}", key.code, key.modifiers));

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
            let fresh = runtime
                .build_session_messages(agent_type, working_dir_str)
                .await?;
            {
                let mut g = messages.lock().await;
                *g = fresh;
            }
            ctx.transcript.clear();
            *ctx.transcript_gen = ctx.transcript_gen.wrapping_add(1);
            *ctx.transcript_scroll_up = 0;
            ctx.tool_folds_expanded.clear();
            *ctx.next_tool_fold_id = 0;
            *ctx.exec_live_tail = None;
            *ctx.fold_layout_rev = ctx.fold_layout_rev.wrapping_add(1);
            ctx.input.clear();
            *ctx.history_idx = None;
            *ctx.last_turn_error = None;
            *ctx.help_open = false;
            *ctx.rev_search = None;
            reset_slash_state(ctx);
            *ctx.executing_since = None;
            *ctx.last_max_input_tokens = 0;
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
            } else if !ctx.input.is_empty() {
                ctx.input.clear();
                *ctx.history_idx = None;
                reset_slash_state(ctx);
            } else if ctx.last_turn_error.is_some() {
                *ctx.last_turn_error = None;
            } else {
                return Ok(TuiLoopCtl::Break);
            }
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
            if ctx.rev_search.is_none() {
                ctx.input.move_home();
            }
            Ok(TuiLoopCtl::Ok)
        }
        KeyCode::End => {
            if ctx.rev_search.is_none() {
                ctx.input.move_end();
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
            if *ctx.help_open || ctx.pending_approval.is_some() {
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
            if *ctx.executing || ctx.pending_approval.is_some() {
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
                let fresh = runtime
                    .build_session_messages(agent_type, working_dir_str)
                    .await?;
                {
                    let mut g = messages.lock().await;
                    *g = fresh;
                }
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
                return Ok(TuiLoopCtl::Continue);
            }
            if trimmed.starts_with("/compact") {
                if *ctx.executing || ctx.pending_approval.is_some() {
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
                let fresh = runtime
                    .build_session_messages(agent_type, working_dir_str)
                    .await?;
                {
                    let mut g = messages.lock().await;
                    *g = fresh;
                }
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
                                let fresh = runtime
                                    .build_session_messages(agent_type, working_dir_str)
                                    .await?;
                                {
                                    let mut g = messages.lock().await;
                                    *g = fresh;
                                }
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
                        } else {
                            "approval: none".to_string()
                        };
                        lines.push(Line::from(Span::styled(appr, style_dim())));
                        ctx.transcript.push(TranscriptEntry::Plain(lines));
                        *ctx.transcript_gen = ctx.transcript_gen.wrapping_add(1);
                        return Ok(TuiLoopCtl::Continue);
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
                    slash_commands::ParsedSlashCommand::Compact => {
                        // handled earlier by /compact block
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
                )
                .await,
            );
            Ok(TuiLoopCtl::Ok)
        }
        KeyCode::Char(c) => {
            if ctx.pending_approval.is_some() {
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
