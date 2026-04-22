//! 流式 REPL 键盘与审批/选题事件（与 `stream_ratatui` UI 线程配合）。

use crate::i18n::tr_args;
use crate::repl::line_state::{
    reset_slash_state, stream_transcript_page_step, ReplCtl, ReplLineState,
};
use crate::term::input::{history_apply_down, history_apply_up};
use crate::term::util::{sanitize_paste, trim_or_default, MAX_PASTE_CHARS};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use fluent_bundle::FluentArgs;

/// 流式 REPL 主循环是否处理该键盘事件。
/// 为改善 macOS/中文 IME 与各类终端组合，**不做** Release 过滤；若个别终端出现重复键再收紧。
pub(crate) fn stream_repl_accept_key_event(key: &KeyEvent) -> bool {
    // 部分终端在按住 Enter 时会发 Repeat；过滤回车类 Repeat，避免重复 submit/重复状态提示。
    if key.kind == KeyEventKind::Repeat {
        return !matches!(
            key.code,
            KeyCode::Enter | KeyCode::Char('\n') | KeyCode::Char('\r')
        );
    }
    true
}
pub(crate) fn handle_event(ev: Event, state: &mut ReplLineState) -> anyhow::Result<ReplCtl> {
    match ev {
        Event::Paste(text) => {
            let (clean, truncated) = sanitize_paste(text);
            if truncated {
                let mut a = FluentArgs::new();
                a.set("n", MAX_PASTE_CHARS as i64);
                eprintln!("{}", tr_args("term-err-paste-truncated", &a));
            }
            state.input.insert_str(&clean);
            state.history_idx = None;
            reset_slash_state(state);
            Ok(ReplCtl::Continue)
        }
        Event::Key(key) => {
            // Kitty / 增强键盘协议会发 Release；若当作普通键处理会导致重复或状态错乱。
            if key.kind == KeyEventKind::Release {
                return Ok(ReplCtl::Continue);
            }
            if !stream_repl_accept_key_event(&key) {
                return Ok(ReplCtl::Continue);
            }
            match key.code {
                KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    Ok(ReplCtl::ClearSession)
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    state.input.clear();
                    state.history_idx = None;
                    reset_slash_state(state);
                    Ok(ReplCtl::Continue)
                }
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if state.executing_since.is_some() {
                        return Ok(ReplCtl::CooperativeCancelTurn);
                    }
                    if state.input.is_empty() {
                        return Ok(ReplCtl::Eof);
                    }
                    state.input.clear();
                    state.history_idx = None;
                    reset_slash_state(state);
                    Ok(ReplCtl::Continue)
                }
                KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if state.input.is_empty() {
                        Ok(ReplCtl::Eof)
                    } else {
                        state.input.delete_forward();
                        reset_slash_state(state);
                        Ok(ReplCtl::Continue)
                    }
                }
                KeyCode::Esc => {
                    // 不按 Esc 清空整行：中文 IME 常用 Esc 关闭候选，清空会破坏输入。
                    Ok(ReplCtl::Continue)
                }
                KeyCode::Up => {
                    history_apply_up(
                        &state.input_history,
                        &mut state.history_idx,
                        &mut state.input,
                    );
                    reset_slash_state(state);
                    Ok(ReplCtl::Continue)
                }
                KeyCode::Down => {
                    history_apply_down(
                        &state.input_history,
                        &mut state.history_idx,
                        &mut state.input,
                    );
                    reset_slash_state(state);
                    Ok(ReplCtl::Continue)
                }
                KeyCode::PageUp => {
                    state.stream_repl_auto_scroll_follow = false;
                    let step = stream_transcript_page_step(state);
                    state.stream_transcript_scroll =
                        state.stream_transcript_scroll.saturating_add(step);
                    Ok(ReplCtl::Continue)
                }
                KeyCode::PageDown => {
                    let step = stream_transcript_page_step(state);
                    state.stream_transcript_scroll =
                        state.stream_transcript_scroll.saturating_sub(step);
                    if state.stream_transcript_scroll == 0 {
                        state.stream_repl_auto_scroll_follow = true;
                    }
                    Ok(ReplCtl::Continue)
                }
                KeyCode::Home if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    state.stream_repl_auto_scroll_follow = false;
                    state.stream_transcript_scroll = usize::MAX;
                    Ok(ReplCtl::Continue)
                }
                KeyCode::End if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    state.stream_transcript_scroll = 0;
                    state.stream_repl_auto_scroll_follow = true;
                    Ok(ReplCtl::Continue)
                }
                KeyCode::Left => {
                    state.input.move_left();
                    Ok(ReplCtl::Continue)
                }
                KeyCode::Right => {
                    state.input.move_right();
                    Ok(ReplCtl::Continue)
                }
                KeyCode::Home => {
                    state.input.move_home();
                    Ok(ReplCtl::Continue)
                }
                KeyCode::End => {
                    state.input.move_end();
                    Ok(ReplCtl::Continue)
                }
                KeyCode::Delete => {
                    state.input.delete_forward();
                    reset_slash_state(state);
                    Ok(ReplCtl::Continue)
                }
                KeyCode::Backspace => {
                    state.input.backspace();
                    reset_slash_state(state);
                    Ok(ReplCtl::Continue)
                }
                KeyCode::BackTab => Ok(ReplCtl::Continue),
                KeyCode::Tab => {
                    // 流式 REPL 不做 `/` 补全；不占 Tab（中文 IME 常用 Tab 切候选）。
                    Ok(ReplCtl::Continue)
                }
                // 不依赖终端 bracketed-paste：直连系统剪贴板（raw TTY 下 Cmd+V 常到不了 Event::Paste）
                KeyCode::Char(c)
                    if (c == 'v' || c == 'V')
                        && ((key.modifiers.contains(KeyModifiers::CONTROL)
                            && key.modifiers.contains(KeyModifiers::SHIFT))
                            || key.modifiers.contains(KeyModifiers::SUPER)) =>
                {
                    if let Some(raw) = crate::repl_clipboard::read_system_clipboard() {
                        let (clean, truncated) = sanitize_paste(raw);
                        if truncated {
                            let mut a = FluentArgs::new();
                            a.set("n", MAX_PASTE_CHARS as i64);
                            eprintln!("{}", tr_args("term-err-paste-truncated", &a));
                        }
                        state.input.insert_str(&clean);
                        state.history_idx = None;
                        reset_slash_state(state);
                    }
                    Ok(ReplCtl::Continue)
                }
                KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
                    state.input.insert('\n');
                    state.history_idx = None;
                    reset_slash_state(state);
                    Ok(ReplCtl::Continue)
                }
                KeyCode::Enter
                | KeyCode::Char('\n')
                | KeyCode::Char('\r')
                | KeyCode::Char('\u{0085}')
                | KeyCode::Char('\u{2028}')
                | KeyCode::Char('\u{2029}') => {
                    let trimmed_owned = trim_or_default(&state.input.as_string()).to_string();
                    state.input.clear();
                    state.history_idx = None;
                    reset_slash_state(state);
                    if trimmed_owned.is_empty() {
                        return Ok(ReplCtl::Continue);
                    }
                    if state.input_history.last().map(|s| s.as_str())
                        != Some(trimmed_owned.as_str())
                    {
                        state.input_history.push(trimmed_owned.clone());
                    }
                    Ok(ReplCtl::Submit(trimmed_owned))
                }
                KeyCode::Char(c) => {
                    if c.is_control() {
                        return Ok(ReplCtl::Continue);
                    }
                    state.history_idx = None;
                    state.input.insert(c);
                    reset_slash_state(state);
                    Ok(ReplCtl::Continue)
                }
                _ => Ok(ReplCtl::Continue),
            }
        }
        _ => Ok(ReplCtl::Continue),
    }
}

/// 流式 REPL 审批条：在 [`handle_event`] 之前消费方向键与确认（与 `tasks_repl` 原逻辑一致）。
pub(crate) fn apply_stream_approval_key(state: &mut ReplLineState, key: KeyEvent) -> bool {
    use crate::term::ApprovalDecision;

    let Some(p) = state.pending_approval.take() else {
        return false;
    };
    match key.code {
        KeyCode::Up => {
            state.approval_menu_selected = (state.approval_menu_selected + 2) % 3;
            state.pending_approval = Some(p);
        }
        KeyCode::Down => {
            state.approval_menu_selected = (state.approval_menu_selected + 1) % 3;
            state.pending_approval = Some(p);
        }
        KeyCode::Enter => {
            let d = match state.approval_menu_selected % 3 {
                0 => ApprovalDecision::AllowOnce,
                1 => ApprovalDecision::AllowToolForProject,
                _ => ApprovalDecision::Deny,
            };
            let _ = p.reply.send(d);
            state.approval_menu_selected = 0;
        }
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            let _ = p.reply.send(ApprovalDecision::AllowOnce);
            state.approval_menu_selected = 0;
        }
        KeyCode::Char('p') | KeyCode::Char('P') => {
            let _ = p.reply.send(ApprovalDecision::AllowToolForProject);
            state.approval_menu_selected = 0;
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            let _ = p.reply.send(ApprovalDecision::Deny);
            state.approval_menu_selected = 0;
        }
        _ => {
            state.pending_approval = Some(p);
        }
    }
    true
}

/// 流式 REPL 选题条：在 [`handle_event`] 之前消费方向键与确认（与审批一致）。
pub(crate) fn apply_stream_user_question_key(state: &mut ReplLineState, key: KeyEvent) -> bool {
    let Some(p) = state.pending_user_question.take() else {
        return false;
    };
    let n = p.option_labels.len().max(1);
    match key.code {
        KeyCode::Up => {
            state.user_question_menu_selected = (state.user_question_menu_selected + n - 1) % n;
            state.pending_user_question = Some(p);
        }
        KeyCode::Down => {
            state.user_question_menu_selected = (state.user_question_menu_selected + 1) % n;
            state.pending_user_question = Some(p);
        }
        KeyCode::Enter => {
            let i = state.user_question_menu_selected % n;
            let label = p.option_labels.get(i).cloned().unwrap_or_default();
            let _ = p.reply.send(Ok(vec![label]));
            state.user_question_menu_selected = 0;
        }
        KeyCode::Esc => {
            let _ = p.reply.send(Err(()));
            state.user_question_menu_selected = 0;
        }
        _ => {
            state.pending_user_question = Some(p);
        }
    }
    true
}
