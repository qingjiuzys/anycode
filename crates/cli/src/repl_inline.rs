//! REPL 行内编辑（ratatui）：上方为输出区，底部为输入行；斜杠候选在**输入行上方**（对齐 Claude Code）。

use crate::i18n::{tr, tr_args};
use crate::md_tui::text_display_width;
use crate::slash_commands;
use crate::tui::input::{
    history_apply_down, history_apply_up, prompt_multiline_lines_and_cursor, InputState,
};
use crate::tui::styles::style_dim;
use crate::tui::util::{sanitize_paste, trim_or_default, truncate_preview, MAX_PASTE_CHARS};
use crossterm::cursor::Hide;
use crossterm::event::{DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use fluent_bundle::FluentArgs;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Terminal;
use std::io::{stdout, Stdout};
use std::sync::{Arc, Mutex};

/// 仅用于绘制：保留尾部若干行，避免 transcript 撑爆屏幕。
const TRANSCRIPT_MAX_DISPLAY_LINES: usize = 256;

pub(crate) struct ReplLineState {
    pub input: InputState,
    pub slash_pick: usize,
    pub slash_suppress: bool,
    pub input_history: Vec<String>,
    pub history_idx: Option<usize>,
    /// 任务与 REPL 消息（显示在输入区上方）；与异步任务共享以便 tail 写入时重绘。
    pub transcript: Arc<Mutex<String>>,
}

impl Default for ReplLineState {
    fn default() -> Self {
        Self {
            input: InputState::default(),
            slash_pick: 0,
            slash_suppress: false,
            input_history: Vec::new(),
            history_idx: None,
            transcript: Arc::new(Mutex::new(String::new())),
        }
    }
}

pub(crate) enum ReplCtl {
    Continue,
    Submit(String),
    Eof,
}

fn reset_slash_state(state: &mut ReplLineState) {
    state.slash_pick = 0;
    state.slash_suppress = false;
}

fn cursor_on_first_line(input: &InputState) -> bool {
    !input.chars[..input.cursor].iter().any(|&c| c == '\n')
}

fn slash_suggestions_for_ctx(state: &ReplLineState) -> Vec<slash_commands::SlashSuggestionItem> {
    if state.slash_suppress {
        return Vec::new();
    }
    slash_commands::slash_suggestions_for_first_line(&state.input.as_string())
}

fn apply_slash_pick_to_input(state: &mut ReplLineState) {
    let cands = slash_commands::slash_suggestions_for_first_line(&state.input.as_string());
    if cands.is_empty() {
        return;
    }
    let len = cands.len();
    let pick = state.slash_pick % len;
    let new_first = cands[pick].replacement.clone();
    let new_buf = slash_commands::replace_first_line(&state.input.as_string(), &new_first);
    state.input.set_from_str(&new_buf);
    state.slash_pick = 0;
    state.history_idx = None;
}

fn tail_for_display(raw: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    if lines.len() <= max_lines {
        return raw.to_string();
    }
    lines[lines.len().saturating_sub(max_lines)..].join("\n")
}

pub(crate) struct ReplTerminalGuard {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    in_alt: bool,
}

impl ReplTerminalGuard {
    pub(crate) fn new() -> anyhow::Result<Self> {
        enable_raw_mode()?;
        let mut out = stdout();
        execute!(out, EnterAlternateScreen, Hide, EnableBracketedPaste)?;
        let backend = CrosstermBackend::new(out);
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;
        Ok(Self {
            terminal,
            in_alt: true,
        })
    }

    pub(crate) fn draw(&mut self, state: &ReplLineState) -> anyhow::Result<()> {
        self.terminal.draw(|f| {
            let area = f.size();
            let slash_candidates = slash_suggestions_for_ctx(state);
            let input_inner_w = area.width.max(8);

            let slash_ghost = if state.slash_suppress {
                None
            } else {
                slash_commands::slash_ghost_suffix(&state.input.as_string(), state.input.cursor)
            };
            let (pl, cur) =
                prompt_multiline_lines_and_cursor(&state.input, input_inner_w, slash_ghost);
            let input_line_count = pl.len().max(1) as u16;
            let sugg_h = if slash_candidates.is_empty() {
                0u16
            } else {
                let len = slash_candidates.len();
                let pick = state.slash_pick % len;
                const MAX_SHOW: usize = 8;
                let start = if len <= MAX_SHOW {
                    0usize
                } else {
                    pick.saturating_sub(MAX_SHOW / 2)
                        .min(len.saturating_sub(MAX_SHOW))
                };
                let end = (start + MAX_SHOW).min(len);
                let mut h = (end - start) as u16;
                if len > MAX_SHOW {
                    h = h.saturating_add(1);
                }
                h = h.saturating_add(1);
                h.min(12)
            };

            let bottom_h = (sugg_h.saturating_add(input_line_count.saturating_add(1)))
                .max(3)
                .min(area.height.saturating_sub(1));
            let top_bot = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(bottom_h)])
                .split(area);
            let top_cell = top_bot[0];
            let bottom_outer = top_bot[1];

            let transcript_guard = state.transcript.lock().unwrap_or_else(|e| e.into_inner());
            let tail = tail_for_display(
                transcript_guard.as_str(),
                TRANSCRIPT_MAX_DISPLAY_LINES.max(top_cell.height as usize * 2),
            );
            let top_par = Paragraph::new(Text::raw(tail))
                .style(style_dim())
                .wrap(Wrap { trim: false });
            f.render_widget(top_par, top_cell);

            let sugg_input = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(sugg_h), Constraint::Min(1)])
                .split(bottom_outer);
            let sugg_rect = sugg_input[0];
            let input_rect = sugg_input[1];

            if !slash_candidates.is_empty() {
                let len = slash_candidates.len();
                let pick = state.slash_pick % len;
                const MAX_SHOW: usize = 8;
                let start = if len <= MAX_SHOW {
                    0usize
                } else {
                    pick.saturating_sub(MAX_SHOW / 2)
                        .min(len.saturating_sub(MAX_SHOW))
                };
                let end = (start + MAX_SHOW).min(len);
                let mut sugg_lines: Vec<Line> = Vec::new();
                for idx in start..end {
                    let item = &slash_candidates[idx];
                    let is_sel = idx == pick;
                    let cmd_w = text_display_width(item.display.as_str()).max(6).min(14);
                    let desc_max = (sugg_rect.width as usize)
                        .saturating_sub(4 + cmd_w + 2)
                        .max(6);
                    let desc = truncate_preview(&item.description, desc_max);
                    let cmd_style = if is_sel {
                        Style::default()
                            .bg(Color::Blue)
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        style_dim()
                    };
                    sugg_lines.push(Line::from(vec![
                        Span::styled(if is_sel { "▸ " } else { "  " }, style_dim()),
                        Span::styled(item.display.as_str(), cmd_style),
                        Span::styled(format!("  {desc}"), style_dim()),
                    ]));
                }
                if len > MAX_SHOW {
                    let mut a = FluentArgs::new();
                    a.set("s", (start + 1) as i64);
                    a.set("e", end as i64);
                    a.set("n", len as i64);
                    sugg_lines.push(Line::from(Span::styled(
                        tr_args("tui-slash-range", &a),
                        style_dim(),
                    )));
                }
                sugg_lines.push(Line::from(Span::styled(tr("tui-slash-nav"), style_dim())));
                let sugg_par = Paragraph::new(Text::from(sugg_lines)).wrap(Wrap { trim: false });
                f.render_widget(sugg_par, sugg_rect);
            }

            let mut prompt_hw_cursor: Option<(usize, usize)> = None;
            let lines_before = 0usize;
            if let Some((li, ox)) = cur {
                prompt_hw_cursor = Some((lines_before + li, usize::from(ox)));
            }
            let input_par = Paragraph::new(Text::from(pl)).wrap(Wrap { trim: false });
            f.render_widget(input_par, input_rect);

            if let Some((gli, ox)) = prompt_hw_cursor {
                if input_rect.height > 0 {
                    let ya = input_rect.y.saturating_add(gli as u16);
                    let y_end = input_rect.y + input_rect.height;
                    if ya < y_end {
                        let max_x = input_rect
                            .x
                            .saturating_add(input_rect.width.saturating_sub(1));
                        let xa = input_rect.x.saturating_add(ox as u16).min(max_x);
                        f.set_cursor(xa, ya);
                    }
                }
            }
        })?;
        Ok(())
    }

    pub(crate) fn suspend_for_output(&mut self) -> anyhow::Result<()> {
        disable_raw_mode()?;
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen)?;
        self.terminal.show_cursor()?;
        let mut out = stdout();
        execute!(out, DisableBracketedPaste)?;
        self.in_alt = false;
        Ok(())
    }

    pub(crate) fn resume_after_output(&mut self) -> anyhow::Result<()> {
        enable_raw_mode()?;
        execute!(self.terminal.backend_mut(), EnterAlternateScreen)?;
        self.terminal.hide_cursor()?;
        let mut out = stdout();
        execute!(out, EnableBracketedPaste)?;
        self.in_alt = true;
        Ok(())
    }
}

impl Drop for ReplTerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        if self.in_alt {
            let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        }
        let _ = self.terminal.show_cursor();
        let mut out = stdout();
        let _ = execute!(out, DisableBracketedPaste);
    }
}

pub(crate) fn handle_event(ev: Event, state: &mut ReplLineState) -> anyhow::Result<ReplCtl> {
    match ev {
        Event::Resize(_, _) => Ok(ReplCtl::Continue),
        Event::Paste(text) => {
            let (clean, truncated) = sanitize_paste(text);
            if truncated {
                let mut a = FluentArgs::new();
                a.set("n", MAX_PASTE_CHARS as i64);
                eprintln!("{}", tr_args("tui-err-paste-truncated", &a));
            }
            state.input.insert_str(&clean);
            state.history_idx = None;
            reset_slash_state(state);
            Ok(ReplCtl::Continue)
        }
        Event::Key(key) => match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
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
                if !state.input.is_empty() {
                    state.input.clear();
                    state.history_idx = None;
                    reset_slash_state(state);
                }
                Ok(ReplCtl::Continue)
            }
            KeyCode::Up => {
                let cands = slash_suggestions_for_ctx(state);
                if !cands.is_empty() && cursor_on_first_line(&state.input) {
                    let len = cands.len();
                    state.slash_pick = (state.slash_pick + len - 1) % len;
                    state.history_idx = None;
                    return Ok(ReplCtl::Continue);
                }
                history_apply_up(
                    &state.input_history,
                    &mut state.history_idx,
                    &mut state.input,
                );
                reset_slash_state(state);
                Ok(ReplCtl::Continue)
            }
            KeyCode::Down => {
                let cands = slash_suggestions_for_ctx(state);
                if !cands.is_empty() && cursor_on_first_line(&state.input) {
                    let len = cands.len();
                    state.slash_pick = (state.slash_pick + 1) % len;
                    state.history_idx = None;
                    return Ok(ReplCtl::Continue);
                }
                history_apply_down(
                    &state.input_history,
                    &mut state.history_idx,
                    &mut state.input,
                );
                reset_slash_state(state);
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
            KeyCode::BackTab => {
                let cands = slash_suggestions_for_ctx(state);
                if !cands.is_empty() && cursor_on_first_line(&state.input) {
                    let len = cands.len();
                    state.slash_pick = (state.slash_pick + len - 1) % len;
                }
                Ok(ReplCtl::Continue)
            }
            KeyCode::Tab => {
                let cands = slash_suggestions_for_ctx(state);
                if !cands.is_empty() && cursor_on_first_line(&state.input) {
                    apply_slash_pick_to_input(state);
                    state.slash_suppress = true;
                    return Ok(ReplCtl::Continue);
                }
                for _ in 0..4 {
                    state.input.insert(' ');
                }
                state.history_idx = None;
                reset_slash_state(state);
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
                if !slash_suggestions_for_ctx(state).is_empty() {
                    apply_slash_pick_to_input(state);
                    state.slash_suppress = true;
                }
                let trimmed_owned = trim_or_default(&state.input.as_string()).to_string();
                state.input.clear();
                state.history_idx = None;
                reset_slash_state(state);
                if trimmed_owned.is_empty() {
                    return Ok(ReplCtl::Continue);
                }
                if state.input_history.last().map(|s| s.as_str()) != Some(trimmed_owned.as_str()) {
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
        },
        _ => Ok(ReplCtl::Continue),
    }
}
