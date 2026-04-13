//! REPL 行内编辑（ratatui）：上方为输出区，底部为输入行；斜杠候选在**输入行下方**。

use crate::i18n::{tr, tr_args};
use crate::md_tui::{
    pad_end_to_display_width, text_display_width, truncate_to_display_width, wrap_string_to_width,
};
use crate::slash_commands;
use crate::tui::input::{
    history_apply_down, history_apply_up, prompt_multiline_lines_and_cursor, InputState,
};
use crate::tui::styles::{style_assistant, style_dim, style_tool, style_warn};
use crate::tui::util::{sanitize_paste, trim_or_default, truncate_preview, MAX_PASTE_CHARS};
use anycode_core::TurnTokenUsage;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use fluent_bundle::FluentArgs;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Paragraph, Widget, Wrap};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// 仅用于绘制：保留尾部若干行，避免 transcript 撑爆屏幕。
#[allow(dead_code)]
const TRANSCRIPT_MAX_DISPLAY_LINES: usize = 256;

pub(crate) struct ReplLineState {
    pub input: InputState,
    pub slash_pick: usize,
    pub slash_suppress: bool,
    pub input_history: Vec<String>,
    pub history_idx: Option<usize>,
    /// 流式 / Inline REPL 底栏顶行：模型 / 审批等（与全屏 TUI 脚标信息对齐）。
    pub dock_status: String,
    /// 任务与 REPL 消息（显示在输入区上方）；与异步任务共享以便 tail 写入时重绘。
    #[allow(dead_code)]
    pub transcript: Arc<Mutex<String>>,
    /// 流式 REPL 主区宽度（ratatui `draw` 回写），供 transcript 排版换行。
    pub stream_viewport_width: u16,
    /// 与全屏 TUI 一致：待处理的工具审批（仅流式 REPL 主循环设置）。
    pub pending_approval: Option<crate::tui::PendingApproval>,
    pub approval_menu_selected: usize,
    /// 流式 REPL：自然语言轮开始执行时起算，供 Prompt HUD 显示耗时（与全屏 TUI `executing_since` 一致）。
    pub executing_since: Option<Instant>,
    /// 回合结束后在 prompt 上方短暂显示 Claude 风格摘要（耗时 + ctx tokens）。
    pub finished_turn_summary: Option<String>,
    pub finished_turn_summary_until: Option<Instant>,
    /// 主区向上滚动的显示行数（从贴底算起，越大越「老」）；仅流式 Inline 使用。
    pub stream_transcript_scroll: usize,
    /// 最近完成的一轮 `execute_turn` 聚合用量（供 `/context` 与 HUD 对齐）。
    pub last_turn_token_usage: Option<TurnTokenUsage>,
    /// 退出时 `ANYCODE_STREAM_EXIT_SCROLLBACK_DUMP=anchor` 用的字节偏移：当前「自然语言轮」写入前 `transcript.len()`（与异步侧 `turn_transcript_anchor` 一致）。
    pub stream_exit_dump_anchor: usize,
}

impl Default for ReplLineState {
    fn default() -> Self {
        Self {
            input: InputState::default(),
            slash_pick: 0,
            slash_suppress: false,
            input_history: Vec::new(),
            history_idx: None,
            dock_status: String::new(),
            transcript: Arc::new(Mutex::new(String::new())),
            stream_viewport_width: 80,
            pending_approval: None,
            approval_menu_selected: 0,
            executing_since: None,
            finished_turn_summary: None,
            finished_turn_summary_until: None,
            stream_transcript_scroll: 0,
            last_turn_token_usage: None,
            stream_exit_dump_anchor: 0,
        }
    }
}

pub(crate) enum ReplCtl {
    Continue,
    Submit(String),
    /// 与全屏 TUI Ctrl+L 一致：清空本会话消息并重建 system 上下文。
    ClearSession,
    Eof,
}

pub(crate) fn reset_slash_state(state: &mut ReplLineState) {
    state.slash_pick = 0;
    state.slash_suppress = false;
}

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

fn cursor_on_first_line(input: &InputState) -> bool {
    !input.chars[..input.cursor].contains(&'\n')
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

/// 将 `body` 按 `wrap_width` 折成与 ratatui `Paragraph`+`Wrap` 一致的**显示行**列表（逐逻辑行 `wrap_string_to_width`）。
#[allow(dead_code)]
fn transcript_wrapped_rows(body: &str, wrap_width: usize) -> Vec<String> {
    let w = wrap_width.max(8);
    let mut out = Vec::new();
    for line in body.lines() {
        out.extend(wrap_string_to_width(line, w));
    }
    out
}

#[inline]
fn is_unicode_horizontal_rule_char(c: char) -> bool {
    matches!(c, '─' | '━' | '═') || matches!(c, '\u{2500}'..='\u{2503}' | '\u{2550}'..='\u{2551}')
}

/// 连续盒线横笔或 3+ 个 ASCII `-`（Markdown `---` 规则线）压成单个空格，避免与 JSON/正文粘成「满屏横线」。
fn collapse_rule_like_runs(line: &str) -> String {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Run {
        None,
        UnicodeBar,
        AsciiDash,
    }

    let mut out = String::with_capacity(line.len());
    let mut run = Run::None;
    let mut ascii_len = 0usize;

    let flush = |out: &mut String, run: &mut Run, ascii_len: &mut usize| {
        match *run {
            Run::None => {}
            Run::UnicodeBar => out.push(' '),
            Run::AsciiDash => {
                if *ascii_len >= 3 {
                    out.push(' ');
                } else {
                    for _ in 0..*ascii_len {
                        out.push('-');
                    }
                }
                *ascii_len = 0;
            }
        }
        *run = Run::None;
    };

    for ch in line.chars() {
        let is_bar = is_unicode_horizontal_rule_char(ch);
        let is_dash = ch == '-' || ch == '－'; // U+FF0D 全角连字符
        if is_bar {
            if run != Run::UnicodeBar {
                flush(&mut out, &mut run, &mut ascii_len);
                run = Run::UnicodeBar;
            }
        } else if is_dash {
            if run != Run::AsciiDash {
                flush(&mut out, &mut run, &mut ascii_len);
                run = Run::AsciiDash;
                ascii_len = 0;
            }
            ascii_len += 1;
        } else {
            flush(&mut out, &mut run, &mut ascii_len);
            out.push(ch);
        }
    }
    flush(&mut out, &mut run, &mut ascii_len);
    out
}

/// 整行仅为装饰横线 / 表格线元字符（无字母数字）时视为噪音行。
fn is_rule_or_table_garnish_line(t: &str) -> bool {
    !t.is_empty()
        && t.chars().all(|c| {
            c.is_whitespace()
                || is_unicode_horizontal_rule_char(c)
                || matches!(
                    c,
                    '-' | '_'
                        | '|'
                        | '┌'
                        | '┐'
                        | '└'
                        | '┘'
                        | '│'
                        | '├'
                        | '┤'
                        | '┬'
                        | '┴'
                        | '┼'
                )
        })
}

/// `ReplSink::line` 可能直接写入 `Turn failed: LLM error: google… body=[{`（不经 `build_stream_turn_plain`），
/// 必须在 **tail 裁剪之前** 对整段 transcript 处理，否则视口只截到 JSON 续行会永远无法匹配首行。
pub(crate) fn scrub_stream_transcript_llm_raw_dumps(s: &str) -> String {
    fn line_starts_dump(line: &str) -> bool {
        line.contains("google request failed after retries")
            || line.contains("generativelanguage.googleapis.com/v1beta/openai")
            || line.contains("generativelanguage.googleapis.com/v1/openai")
            || ((line.contains("Turn failed:") || line.contains("LLM error:"))
                && line.contains("google")
                && (line.contains("body=") || line.contains("status=400")))
    }

    fn line_exits_dump(line: &str) -> bool {
        let t = line.trim_start();
        if t.starts_with("❯ ") {
            return true;
        }
        if let Some(rest) = t.strip_prefix('>') {
            let r = rest.trim_start();
            if r.is_empty() {
                return false;
            }
            let first = r.chars().next().unwrap();
            if !matches!(first, '{' | '[' | '"' | '}' | ']') {
                return true;
            }
        }
        line.contains("Google 生成式语言接口")
            || line.contains("Google Generative Language API rejected")
            || (line.contains("Turn failed:") && !line.contains("LLM error:"))
    }

    let mut out: Vec<String> = Vec::new();
    let mut skipping = false;
    let mut pending_replacement = true;

    for line in s.lines() {
        if !skipping {
            if line_starts_dump(line) {
                skipping = true;
                if pending_replacement {
                    let geo = line.contains("User location is not supported")
                        || s.contains("User location is not supported");
                    if geo {
                        out.push(tr("repl-stream-error-google-geo"));
                    } else {
                        out.push(tr("repl-stream-error-assistant-blob-short"));
                        out.push(tr("repl-stream-error-stderr-hint"));
                    }
                    pending_replacement = false;
                }
                continue;
            }
            pending_replacement = true;
            out.push(line.to_string());
        } else if line_exits_dump(line) {
            skipping = false;
            pending_replacement = true;
            out.push(line.to_string());
        }
    }

    let mut joined = out.join("\n");
    if s.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

/// 流式 Inline 主区展示前清理：去掉纯装饰横线/表格线行，并把正文中连续盒线字符压成空格，
/// 避免 Markdown 误解析出的 `─` 与底栏横线叠成满屏「断线」。
pub(crate) fn sanitize_stream_transcript_visual_noise(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    let mut rows: Vec<String> = Vec::new();
    for raw in s.lines() {
        // `\t` 在部分终端/ratatui 里会拉成宽列，JSON 缩进看起来像「错位/假横线」。
        let raw = raw.replace('\t', " ");
        let collapsed = collapse_rule_like_runs(&raw);
        let t = collapsed.trim();
        if !t.is_empty() && is_rule_or_table_garnish_line(t) {
            continue;
        }
        rows.push(collapsed);
    }
    let mut joined = rows.join("\n");
    if s.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

/// 流式 Inline：按 `wrap_width` 折行后，只保留末尾 `row_budget` 条**显示行**（超长时等价于旧内容上移滚出视口），
/// 不足时在上方补空行使正文贴在输入区上沿（stick-to-bottom）。
///
/// 注意：若仅按 `\n` 逻辑行计数而不折行，长行（如整段 JSON）在 `Paragraph` 中会占多行却仍算 1 行，导致不「上滚」且被裁切。
#[allow(dead_code)]
pub(crate) fn repl_stream_transcript_bottom_padded(
    raw: &str,
    row_budget: u16,
    wrap_width: u16,
    scroll_up: usize,
) -> String {
    let rows = row_budget.max(1) as usize;
    let w = wrap_width.max(8) as usize;
    let logical_max = TRANSCRIPT_MAX_DISPLAY_LINES.max(rows);
    let scrubbed = scrub_stream_transcript_llm_raw_dumps(raw);
    let body = tail_for_display(&scrubbed, logical_max);
    if body.is_empty() {
        return String::new();
    }
    let body = sanitize_stream_transcript_visual_noise(&body);
    if body.trim().is_empty() {
        return String::new();
    }
    let wrapped = transcript_wrapped_rows(&body, w);
    if wrapped.is_empty() {
        return String::new();
    }
    let n = wrapped.len();
    let max_start = n.saturating_sub(rows);
    let scroll_up = scroll_up.min(max_start);
    let start = max_start.saturating_sub(scroll_up);
    let end = (start + rows).min(n);
    let slice = &wrapped[start..end];
    let pad = rows.saturating_sub(slice.len());
    if pad == 0 {
        slice.join("\n")
    } else {
        format!("{}{}", "\n".repeat(pad), slice.join("\n"))
    }
}

#[allow(dead_code)]
fn tail_for_display(raw: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    if lines.len() <= max_lines {
        return raw.to_string();
    }
    lines[lines.len().saturating_sub(max_lines)..].join("\n")
}

/// 底栏布局参数（流式 Inline：**上横线 → 输入 → 斜杠/审批 → 脚标（含 ✶）→ 下横线**，脚标夹在两条 `─` 之间）。
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ReplDockLayout;

impl ReplDockLayout {
    fn max_slash_show(self) -> usize {
        let _ = self;
        5
    }

    fn slash_height_cap(self) -> u16 {
        let _ = self;
        7
    }

    fn approval_total_cap(self) -> u16 {
        let _ = self;
        14
    }

    fn approval_preview_wrap_cap(self) -> usize {
        let _ = self;
        5
    }

    fn min_dock_rows(self) -> u16 {
        let _ = self;
        // 上横线 + 至少一行输入 + 下横线
        3
    }

    /// 输入框**正上方**整行横线（紧贴 `>` 输入区上沿；HUD 画在此行之上）。
    fn prompt_rule_top_rows(self) -> u16 {
        let _ = self;
        1
    }

    /// 输入框**正下方**整行横线（斜杠候选 / 脚标在此行之下）。
    fn prompt_rule_bottom_rows(self) -> u16 {
        let _ = self;
        1
    }
}

fn repl_stream_approval_block_h(width: u16, state: &ReplLineState, layout: ReplDockLayout) -> u16 {
    let Some(p) = state.pending_approval.as_ref() else {
        return 0;
    };
    let w = width.max(8) as usize;
    let pv = p.input_preview.as_str();
    let preview_rows = if text_display_width(pv) <= w {
        1u16
    } else {
        wrap_string_to_width(pv, w.max(8))
            .len()
            .min(layout.approval_preview_wrap_cap()) as u16
    };
    // 标题 + 工具行 + 三选项 + 提示
    (5u16 + preview_rows).min(layout.approval_total_cap())
}

#[derive(Clone, Copy, Debug)]
struct ReplDockNatural {
    /// 与全屏 TUI Prompt HUD 对齐的 `✶`/`⎿` 两行（或收缩为 0～1 行）。
    hud_h: u16,
    rule_top_h: u16,
    input_h: u16,
    rule_bottom_h: u16,
    approval_h: u16,
    sugg_h: u16,
    status_h: u16,
}

impl ReplDockNatural {
    fn sum(self) -> u16 {
        self.hud_h
            .saturating_add(self.rule_top_h)
            .saturating_add(self.input_h)
            .saturating_add(self.approval_h)
            .saturating_add(self.sugg_h)
            .saturating_add(self.status_h)
            .saturating_add(self.rule_bottom_h)
    }
}

fn repl_dock_compute_natural(
    area_width: u16,
    state: &ReplLineState,
    layout: ReplDockLayout,
) -> ReplDockNatural {
    // 活动态 HUD 不再单独占一行（见 [`stream_dock_activity_prefix`] 并入脚标）。
    let hud_h = 0u16;
    let status_h = if state.dock_status.is_empty() {
        0u16
    } else {
        1u16
    };
    let approval_h = repl_stream_approval_block_h(area_width, state, layout);
    let slash_candidates = slash_suggestions_for_ctx(state);
    let input_inner_w = area_width.max(8);
    let slash_ghost = if state.slash_suppress {
        None
    } else {
        slash_commands::slash_ghost_suffix(&state.input.as_string(), state.input.cursor)
    };
    let (pl, _) = prompt_multiline_lines_and_cursor(&state.input, input_inner_w, slash_ghost);
    let input_line_count = pl.len().max(1) as u16;
    let sugg_h = if approval_h > 0 {
        0u16
    } else if slash_candidates.is_empty() {
        0u16
    } else {
        let len = slash_candidates.len();
        let pick = state.slash_pick % len;
        let max_show = layout.max_slash_show();
        let start = if len <= max_show {
            0usize
        } else {
            pick.saturating_sub(max_show / 2)
                .min(len.saturating_sub(max_show))
        };
        let end = (start + max_show).min(len);
        let mut h = (end - start) as u16;
        if len > max_show {
            h = h.saturating_add(1);
        }
        h = h.saturating_add(1);
        h.min(layout.slash_height_cap())
    };
    let input_h = input_line_count.max(1);
    let rule_top_h = layout.prompt_rule_top_rows();
    let rule_bottom_h = layout.prompt_rule_bottom_rows();
    ReplDockNatural {
        hud_h,
        rule_top_h,
        input_h,
        rule_bottom_h,
        approval_h,
        sugg_h,
        status_h,
    }
}

fn repl_dock_block_sum(hud: u16, rt: u16, i: u16, a: u16, g: u16, s: u16, rb: u16) -> u16 {
    hud.saturating_add(rt)
        .saturating_add(i)
        .saturating_add(a)
        .saturating_add(g)
        .saturating_add(s)
        .saturating_add(rb)
}

/// 将自然高度压进或铺满 `target_h`，保证 **各块之和等于 `target_h`**，避免矮终端下 `Layout` 约束溢出叠字。
fn repl_dock_fit_into(target_h: u16, mut n: ReplDockNatural) -> ReplDockNatural {
    let target_h = target_h.max(1);
    let mut hud = n.hud_h;
    let mut rt = n.rule_top_h;
    let mut i = n.input_h.max(1);
    let mut rb = n.rule_bottom_h;
    let mut a = n.approval_h;
    let mut g = n.sugg_h;
    let mut s = n.status_h;

    while repl_dock_block_sum(hud, rt, i, a, g, s, rb) > target_h {
        if g > 0 {
            g -= 1;
            continue;
        }
        if hud > 0 {
            hud -= 1;
            continue;
        }
        if s > 0 {
            s = 0;
            continue;
        }
        if a > 0 {
            a -= 1;
            continue;
        }
        if i > 1 {
            i -= 1;
            continue;
        }
        // 极矮终端：最后才去掉输入上下的横线。
        if rb > 0 {
            rb -= 1;
            continue;
        }
        if rt > 0 {
            rt -= 1;
            continue;
        }
        break;
    }

    let spare = target_h.saturating_sub(repl_dock_block_sum(hud, rt, i, a, g, s, rb));
    if spare > 0 {
        i = i.saturating_add(spare);
    }

    n.hud_h = hud;
    n.rule_top_h = rt;
    n.input_h = i.max(1);
    n.rule_bottom_h = rb;
    n.approval_h = a;
    n.sugg_h = g;
    n.status_h = s;
    n
}

/// 底部 dock（斜杠候选 + 多行输入）高度，与全屏 REPL / 流式 dock 共用同一套布局规则。
///
/// **流式 Inline**：返回值 = dock 内层总高度（含输入上下各一行 `─`，无外层 `Block`）。
pub(crate) fn repl_dock_height(area: Rect, state: &ReplLineState, layout: ReplDockLayout) -> u16 {
    let avail = area.height.saturating_sub(1);
    let nat = repl_dock_compute_natural(area.width.max(1), state, layout);
    let max_inner = avail.max(1);
    let target_inner = nat.sum().max(layout.min_dock_rows()).min(max_inner).max(1);
    let fitted = repl_dock_fit_into(target_inner, nat);
    let inner_h = fitted.sum().min(max_inner).max(1);
    inner_h.min(avail).max(layout.min_dock_rows())
}

/// 流式 dock 曾用的独立 ✶ 行（已改为脚标前辍）；保留以便日后可选恢复。
#[allow(dead_code)]
fn render_stream_hud_to_buffer(buf: &mut Buffer, area: Rect, state: &ReplLineState, hud_h: u16) {
    if hud_h == 0 || area.height == 0 {
        return;
    }
    let pending = state.pending_approval.is_some();
    let exec = state.executing_since.is_some();
    let secs = state.executing_since.map(|t| t.elapsed().as_secs());
    let summary_line = if let (Some(text), Some(until)) = (
        state.finished_turn_summary.as_ref(),
        state.finished_turn_summary_until,
    ) {
        if Instant::now() < until {
            Some(text.as_str())
        } else {
            None
        }
    } else {
        None
    };
    // 审批 > 执行中 > 回合摘要 > 空闲脚标
    let activity = if pending || exec {
        crate::tui::hud_text::prompt_hud_activity_text(pending, exec, secs)
    } else if let Some(s) = summary_line {
        s.to_string()
    } else {
        crate::tui::hud_text::prompt_hud_activity_text(false, false, None)
    };
    let line = Line::from(vec![
        Span::styled("✶ ", style_dim()),
        Span::styled(activity, style_assistant()),
    ]);
    Paragraph::new(Text::from(line)).render(area, buf);
}

/// 将底部 dock 渲染进 `buf`（`buf.area` 应与 `dock_area` 一致，一般为 `Rect::new(0,0,w,bottom_h)`）。
/// 返回相对于 `dock_area` 左上角的光标 `(x,y)`。
pub(crate) fn render_repl_dock_to_buffer(
    buf: &mut Buffer,
    dock_area: Rect,
    state: &ReplLineState,
    layout: ReplDockLayout,
) -> Option<(u16, u16)> {
    let slash_candidates = slash_suggestions_for_ctx(state);
    let input_inner_w = dock_area.width.max(8);
    let slash_ghost = if state.slash_suppress {
        None
    } else {
        slash_commands::slash_ghost_suffix(&state.input.as_string(), state.input.cursor)
    };
    let (pl, cur) = prompt_multiline_lines_and_cursor(&state.input, input_inner_w, slash_ghost);

    let nat = repl_dock_compute_natural(dock_area.width.max(1), state, layout);
    let dock_h = dock_area.height.max(1);
    let fitted = repl_dock_fit_into(dock_h, nat);
    debug_assert_eq!(fitted.sum(), dock_h, "dock layout must fill buffer height");

    let hud_h = fitted.hud_h;
    let approval_h = fitted.approval_h;
    let status_h = fitted.status_h;
    let rule_top_h = fitted.rule_top_h;
    let input_h = fitted.input_h;
    let rule_bottom_h = fitted.rule_bottom_h;
    let sugg_h = fitted.sugg_h;

    let mut constraints: Vec<Constraint> = Vec::new();
    if hud_h > 0 {
        constraints.push(Constraint::Length(hud_h));
    }
    if rule_top_h > 0 {
        constraints.push(Constraint::Length(rule_top_h));
    }
    constraints.push(Constraint::Length(input_h));
    if approval_h > 0 {
        constraints.push(Constraint::Length(approval_h));
    }
    constraints.push(Constraint::Length(sugg_h));
    if status_h > 0 {
        constraints.push(Constraint::Length(status_h));
    }
    if rule_bottom_h > 0 {
        constraints.push(Constraint::Length(rule_bottom_h));
    }
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(dock_area);

    let mut ci = 0usize;
    let hud_rect_opt = if hud_h > 0 {
        let r = chunks[ci];
        ci += 1;
        Some(r)
    } else {
        None
    };
    let rule_top_rect = if rule_top_h > 0 {
        let r = chunks[ci];
        ci += 1;
        Some(r)
    } else {
        None
    };
    let input_rect = chunks[ci];
    ci += 1;
    let approval_rect_opt = if approval_h > 0 {
        let r = chunks[ci];
        ci += 1;
        Some(r)
    } else {
        None
    };
    let sugg_rect = chunks[ci];
    ci += 1;
    let status_rect_opt = if status_h > 0 {
        let r = chunks[ci];
        ci += 1;
        Some(r)
    } else {
        None
    };
    let rule_bottom_rect = if rule_bottom_h > 0 {
        let r = chunks[ci];
        Some(r)
    } else {
        None
    };

    if let Some(hr) = hud_rect_opt {
        render_stream_hud_to_buffer(buf, hr, state, hud_h);
    }

    if let Some(rr) = rule_top_rect {
        let rule_w = dock_area.width.max(1) as usize;
        let rule_txt = "─".repeat(rule_w.min(512));
        let rule_lines: Vec<Line> = (0..rule_top_h)
            .map(|_| Line::from(Span::styled(rule_txt.as_str(), style_dim())))
            .collect();
        Paragraph::new(Text::from(rule_lines)).render(rr, buf);
    }

    let mut prompt_hw_cursor: Option<(usize, usize)> = None;
    let lines_before = 0usize;
    if let Some((li, ox)) = cur {
        prompt_hw_cursor = Some((lines_before + li, usize::from(ox)));
    }
    Paragraph::new(Text::from(pl))
        .wrap(Wrap { trim: false })
        .render(input_rect, buf);

    if let (Some(apr), Some(p)) = (approval_rect_opt, state.pending_approval.as_ref()) {
        let preview_w = input_inner_w as usize;
        let pv = p.input_preview.as_str();
        let mut input_lines: Vec<Line> = vec![
            Line::from(Span::styled(
                tr("tui-approval-question"),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled("⏺ ", style_tool()),
                Span::styled(
                    format!("{} ", p.tool),
                    style_warn().add_modifier(Modifier::BOLD),
                ),
                Span::styled(tr("tui-approval-pending"), style_dim()),
            ]),
        ];
        if text_display_width(pv) <= preview_w {
            input_lines.push(Line::from(Span::styled(pv, style_dim())));
        } else {
            for row in wrap_string_to_width(pv, preview_w.max(8)) {
                input_lines.push(Line::from(Span::styled(row, style_dim())));
            }
        }
        let pick = state.approval_menu_selected % 3;
        let opt_once = tr("tui-approval-opt-once");
        let opt_proj = tr("tui-approval-opt-project");
        let opt_deny = tr("tui-approval-opt-deny");
        for (i, label) in [opt_once, opt_proj, opt_deny].into_iter().enumerate() {
            let prefix = if i == pick { "❯ " } else { "  " };
            let st = if i == pick {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                style_dim()
            };
            input_lines.push(Line::from(vec![
                Span::styled(prefix, st),
                Span::styled(label, st),
            ]));
        }
        input_lines.push(Line::from(vec![Span::styled(
            tr("tui-approval-hint-arrows"),
            style_dim(),
        )]));
        Paragraph::new(Text::from(input_lines))
            .wrap(Wrap { trim: false })
            .render(apr, buf);
    }

    if !slash_candidates.is_empty() {
        let len = slash_candidates.len();
        let pick = state.slash_pick % len;
        let max_show = layout.max_slash_show();
        let start = if len <= max_show {
            0usize
        } else {
            pick.saturating_sub(max_show / 2)
                .min(len.saturating_sub(max_show))
        };
        let end = (start + max_show).min(len);
        let line_w = sugg_rect.width as usize;
        let max_cmd_w =
            slash_commands::slash_menu_cmd_column_width(&slash_candidates, start, end, line_w);
        let mut sugg_lines: Vec<Line> = Vec::new();
        for idx in start..end {
            let item = &slash_candidates[idx];
            let is_sel = idx == pick;
            let d = item.display.as_str();
            let raw = if text_display_width(d) > max_cmd_w {
                truncate_to_display_width(d, max_cmd_w)
            } else {
                d.to_string()
            };
            let cmd_cell = pad_end_to_display_width(&raw, max_cmd_w);
            let desc_max = line_w.saturating_sub(2 + max_cmd_w + 2).max(8);
            let desc = truncate_to_display_width(item.description.trim(), desc_max);
            let cmd_style = if is_sel {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                style_dim()
            };
            sugg_lines.push(Line::from(vec![
                Span::styled(if is_sel { "› " } else { "  " }, style_dim()),
                Span::styled(cmd_cell, cmd_style),
                Span::styled(format!("  {desc}"), style_dim()),
            ]));
        }
        if len > max_show {
            let mut a = FluentArgs::new();
            a.set("s", (start + 1) as i64);
            a.set("e", end as i64);
            a.set("n", len as i64);
            sugg_lines.push(Line::from(Span::styled(
                tr_args("repl-slash-range", &a),
                style_dim(),
            )));
        }
        sugg_lines.push(Line::from(Span::styled(tr("repl-slash-nav"), style_dim())));
        Paragraph::new(Text::from(sugg_lines)).render(sugg_rect, buf);
    }

    if let Some(sr) = status_rect_opt {
        let status_w = (sr.width as usize).max(4);
        let pre = stream_dock_activity_prefix(state);
        let merged = if pre.is_empty() {
            state.dock_status.clone()
        } else {
            format!("{}{}", pre, state.dock_status.as_str())
        };
        let line = truncate_preview(merged.as_str(), status_w);
        Paragraph::new(Text::from(Span::styled(line, style_dim())))
            .wrap(Wrap { trim: false })
            .render(sr, buf);
    }

    if let Some(rr) = rule_bottom_rect {
        let rule_w = dock_area.width.max(1) as usize;
        let rule_txt = "─".repeat(rule_w.min(512));
        let rule_lines: Vec<Line> = (0..rule_bottom_h)
            .map(|_| Line::from(Span::styled(rule_txt.as_str(), style_dim())))
            .collect();
        Paragraph::new(Text::from(rule_lines)).render(rr, buf);
    }

    if let Some((gli, ox)) = prompt_hw_cursor {
        if input_rect.height > 0 {
            let ya = input_rect.y.saturating_add(gli as u16);
            let y_end = input_rect.y + input_rect.height;
            if ya < y_end {
                let max_x = input_rect
                    .x
                    .saturating_add(input_rect.width.saturating_sub(1));
                let xa = input_rect.x.saturating_add(ox as u16).min(max_x);
                return Some((xa, ya));
            }
        }
    }
    None
}

/// `✶ thinking…` / 待审批 / 回合摘要 等，拼在脚标 `dock_status` 前（秒级刷新由每帧重绘保证）。
pub(crate) fn stream_dock_activity_prefix(state: &ReplLineState) -> String {
    if state.pending_approval.is_some() {
        return format!(
            "✶ {} · ",
            crate::tui::hud_text::prompt_hud_activity_text(true, false, None)
        );
    }
    if state.executing_since.is_some() {
        let secs = state.executing_since.map(|t| t.elapsed().as_secs());
        return format!(
            "✶ {} · ",
            crate::tui::hud_text::prompt_hud_activity_text(false, true, secs)
        );
    }
    if let (Some(text), Some(until)) = (
        state.finished_turn_summary.as_ref(),
        state.finished_turn_summary_until,
    ) {
        if Instant::now() < until {
            return format!("✶ {} · ", text);
        }
    }
    String::new()
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
                    let cands = slash_suggestions_for_ctx(state);
                    if !cands.is_empty() && cursor_on_first_line(&state.input) {
                        state.slash_suppress = true;
                        return Ok(ReplCtl::Continue);
                    }
                    // 不按 Esc 清空整行：中文 IME 常用 Esc 关闭候选，清空会破坏输入。
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
                KeyCode::PageUp => {
                    state.stream_transcript_scroll =
                        state.stream_transcript_scroll.saturating_add(8);
                    Ok(ReplCtl::Continue)
                }
                KeyCode::PageDown => {
                    state.stream_transcript_scroll =
                        state.stream_transcript_scroll.saturating_sub(8);
                    Ok(ReplCtl::Continue)
                }
                KeyCode::Home if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    state.stream_transcript_scroll = usize::MAX;
                    Ok(ReplCtl::Continue)
                }
                KeyCode::End if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    state.stream_transcript_scroll = 0;
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
                    // 无 `/` 补全时不再插空格，避免占用 Tab（中文 IME 常用 Tab 切候选）。
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
                            eprintln!("{}", tr_args("tui-err-paste-truncated", &a));
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
    use crate::tui::ApprovalDecision;

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

#[cfg(test)]
mod stream_transcript_tests {
    use super::{
        repl_dock_compute_natural, repl_stream_transcript_bottom_padded,
        sanitize_stream_transcript_visual_noise, scrub_stream_transcript_llm_raw_dumps,
        stream_dock_activity_prefix, stream_repl_accept_key_event, ReplDockLayout, ReplLineState,
    };
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

    #[test]
    fn bottom_pad_inserts_leading_newlines_to_fill_row_budget() {
        let s = repl_stream_transcript_bottom_padded("a\nb", 5, 80, 0);
        assert_eq!(s.lines().count(), 5);
        assert!(s.ends_with("a\nb"));
    }

    #[test]
    fn empty_transcript_stays_empty() {
        assert!(repl_stream_transcript_bottom_padded("", 4, 80, 0).is_empty());
    }

    #[test]
    fn long_logical_line_keeps_last_wrapped_rows_like_scroll() {
        let line: String = "x".repeat(30);
        let s = repl_stream_transcript_bottom_padded(&line, 3, 10, 0);
        let rows: Vec<&str> = s.lines().collect();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], "xxxxxxxxxx");
        assert_eq!(rows[1], "xxxxxxxxxx");
        assert_eq!(rows[2], "xxxxxxxxxx");
    }

    #[test]
    fn scroll_up_shows_older_wrapped_rows() {
        let line: String = (b'a'..=b'z').cycle().take(50).map(|b| b as char).collect();
        let bottom = repl_stream_transcript_bottom_padded(&line, 3, 10, 0);
        let older = repl_stream_transcript_bottom_padded(&line, 3, 10, 2);
        assert_ne!(
            bottom, older,
            "scroll_up>0 should show earlier wrapped rows than stick-to-bottom"
        );
        assert!(older.starts_with("abcdefghij"));
        assert!(bottom.contains("opqrst"));
    }

    #[test]
    fn suppresses_enter_repeat_key_event() {
        let mut enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        enter.kind = KeyEventKind::Repeat;
        assert!(
            !stream_repl_accept_key_event(&enter),
            "repeat enter should be ignored to avoid duplicate submit"
        );

        let mut char_enter = KeyEvent::new(KeyCode::Char('\n'), KeyModifiers::NONE);
        char_enter.kind = KeyEventKind::Repeat;
        assert!(!stream_repl_accept_key_event(&char_enter));
    }

    #[test]
    fn keeps_non_enter_repeat_key_event() {
        let mut up = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        up.kind = KeyEventKind::Repeat;
        assert!(
            stream_repl_accept_key_event(&up),
            "repeat non-enter keys should keep working for navigation"
        );
    }

    #[test]
    fn stream_dock_prompt_sandwiched_by_two_rule_rows() {
        let st = ReplLineState::default();
        let nat = repl_dock_compute_natural(80, &st, ReplDockLayout);
        assert_eq!(nat.rule_top_h, 1);
        assert_eq!(nat.rule_bottom_h, 1);
    }

    #[test]
    fn stream_dock_merges_activity_into_status_not_extra_hud_row() {
        let mut st = ReplLineState::default();
        st.dock_status = "google · model · agent · on".into();
        st.executing_since = Some(std::time::Instant::now());
        let nat = repl_dock_compute_natural(80, &st, ReplDockLayout);
        assert_eq!(nat.hud_h, 0);
        assert_eq!(nat.rule_top_h, 1);
        assert_eq!(nat.status_h, 1);
        assert!(
            stream_dock_activity_prefix(&st).starts_with("✶ "),
            "thinking should appear as status prefix"
        );
    }

    #[test]
    fn sanitize_drops_pure_rule_lines() {
        let s = "hello\n────────────────────────\nworld";
        let o = sanitize_stream_transcript_visual_noise(s);
        assert!(!o.contains('─'));
        assert!(o.contains("hello"));
        assert!(o.contains("world"));
    }

    #[test]
    fn sanitize_collapses_mixed_bar_runs_inside_line() {
        let s = "Turn failed: x\n────  \"error\": {────";
        let o = sanitize_stream_transcript_visual_noise(s);
        assert!(!o.contains('─'), "got: {o:?}");
        assert!(o.contains("\"error\":"));
        assert!(o.contains("Turn failed:"));
    }

    #[test]
    fn sanitize_collapses_ascii_markdown_rule_dashes() {
        let s = "err\n---\n\"code\": 400";
        let o = sanitize_stream_transcript_visual_noise(s);
        assert!(
            !o.contains("---"),
            "thematic-break dashes should collapse, got: {o:?}"
        );
        assert!(o.contains("\"code\":"));
    }

    #[test]
    fn scrub_drops_google_llm_dump_and_inserts_geo_message() {
        let raw = concat!(
            "[> Turn failed: LLM error: google request failed after retries ",
            "status=400 body=[{\n",
            "  \"error\": {\n",
            "    \"code\": 400,\n",
            "    \"message\": \"User location is not supported for the API use.\",\n",
            "    \"status\": \"FAILED_PRECONDITION\"\n",
            "  }\n",
            "}]\n",
            "❯ next\n",
        );
        let o = scrub_stream_transcript_llm_raw_dumps(raw);
        assert!(
            !o.contains("body=[{"),
            "raw HTTP dump should be removed, got: {o:?}"
        );
        assert!(
            !o.contains("FAILED_PRECONDITION"),
            "JSON tail should be removed, got: {o:?}"
        );
        assert!(o.contains("❯ next"));
        assert!(
            o.contains("User location is not supported"),
            "expected localized geo hint, got: {o:?}"
        );
    }

    #[test]
    fn scrub_runs_before_tail_so_header_outside_window_still_strips_json() {
        let filler: String = (0..260).map(|i| format!("pad {i}\n")).collect::<String>();
        let tail = concat!(
            "[> Turn failed: LLM error: google request failed body=[{\n",
            "  \"error\": { \"code\": 400 }\n",
            "}]\n",
        );
        let raw = format!("{filler}{tail}");
        let scrubbed = scrub_stream_transcript_llm_raw_dumps(&raw);
        assert!(!scrubbed.contains("body=[{"));
        let logical_max = 256usize;
        let body = super::tail_for_display(&scrubbed, logical_max);
        assert!(
            !body.contains("\"code\": 400"),
            "tail view should not show JSON after full scrub, got: {body:?}"
        );
    }
}
