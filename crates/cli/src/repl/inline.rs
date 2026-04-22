//! 流式 REPL transcript 清洗、折行与样式（ratatui 主区与单测共用）。
//!
//! 状态与底栏见 [`crate::repl::line_state`]、[`crate::repl::dock_render`]；键盘见 [`crate::repl::stream_events`]。

#![allow(dead_code)] // `repl_stream_transcript_bottom_padded` 等主要由本文件内单测使用。

use crate::i18n::tr;
use crate::md_render::wrap_string_to_width;
use crate::term::styles::{
    style_assistant, style_assistant_prose, style_brand, style_dim, style_error, style_user,
};
use ratatui::style::Style;
#[cfg(test)]
use ratatui::text::{Line, Span, Text};

/// 将 `body` 按 `wrap_width` 折成显示行列表（与 [`crate::md_render::wrap_string_to_width`] 一致，供行预算与上滚裁剪）。
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

pub(crate) fn tail_for_display(raw: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    if lines.len() <= max_lines {
        return raw.to_string();
    }
    lines[lines.len().saturating_sub(max_lines)..].join("\n")
}

/// 流式 Inline：按 `wrap_width` 折行后，只保留末尾 `row_budget` 条**显示行**（超长时等价于旧内容上移滚出视口），
/// 不足时在上方补空行使正文贴在输入区上沿（stick-to-bottom）。
///
/// 注意：若仅按 `\n` 逻辑行计数而不折行，长行（如整段 JSON）在 `Paragraph` 中会占多行却仍算 1 行，导致不「上滚」且被裁切。
pub(crate) fn repl_stream_transcript_bottom_padded(
    raw: &str,
    row_budget: u16,
    wrap_width: u16,
    scroll_up: usize,
) -> String {
    let rows = row_budget.max(1) as usize;
    let w = wrap_width.max(8) as usize;
    let logical_max = crate::repl::line_state::TRANSCRIPT_MAX_DISPLAY_LINES.max(rows);
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

/// 流式 Inline 主区按行上色：错误醒目、用户行高亮、说明性表格变淡（对齐 Claude Code 式层次，避免整页灰字）。
///
/// 预折行与 `md_render::wrap_string_to_width` 一致；渲染时用 `Paragraph::wrap(Wrap { trim: false })`（ratatui `WordWrapper`），
/// 避免无 wrap 时 `LineTruncator` 把超长行甩到下一列与底栏 `─` 叠成「满屏横线」。
#[cfg(test)]
pub(crate) fn stream_transcript_plain_to_styled_text(body: &str) -> Text<'static> {
    let lines: Vec<Line<'static>> = body
        .lines()
        .map(|line| {
            let t = line.trim_start();
            let style = stream_transcript_line_style(t, line);
            Line::from(vec![Span::styled(line.to_string(), style)])
        })
        .collect();
    Text::from(lines)
}

pub(crate) fn stream_transcript_line_style(trimmed: &str, full_line: &str) -> Style {
    // 错误消息 - 红色加粗
    if trimmed.starts_with("Turn failed:") || trimmed.starts_with("Turn join error:") {
        return style_error();
    }

    // 用户消息 - 橙色
    if trimmed.starts_with('❯') {
        return style_user();
    }

    // 斜杠命令帮助 - 灰色
    if looks_like_slash_help_catalog_row(trimmed) {
        return style_dim();
    }

    // 命令示例 - 灰色
    if trimmed.contains("anycode run") && (trimmed.contains("-C") || trimmed.contains("--agent")) {
        return style_dim();
    }

    // 标题和命令标题 - 紫色品牌色
    if trimmed.starts_with("Commands:") || trimmed.starts_with("命令：") {
        return style_brand();
    }

    // 会话状态消息 - 紫色品牌色
    if trimmed.contains("Session restored")
        || trimmed.contains("已恢复会话")
        || trimmed.contains("Switched Agent")
        || trimmed.contains("已切换 Agent")
    {
        return style_brand();
    }

    // 标准错误输出 - 灰色
    if full_line.contains("stderr") || full_line.contains("标准错误") {
        return style_dim();
    }

    // 工具执行状态 - 紫色助手色
    if trimmed.starts_with('✅')
        || trimmed.starts_with('❌')
        || trimmed.contains("🤖")
        || trimmed.contains("🔧")
    {
        return style_assistant();
    }

    // 等待状态 - 灰色
    if trimmed.starts_with('📝') || trimmed.starts_with('⏳') {
        return style_dim();
    }

    // 输出标题 - 紫色助手色
    if trimmed == "Output:" || trimmed.starts_with("输出：") {
        return style_assistant();
    }

    // Markdown 标题样式识别
    if trimmed.starts_with('#') {
        return Style::default()
            .fg(ratatui::style::Color::Rgb(255, 140, 66)) // 橙色 H1
            .add_modifier(ratatui::style::Modifier::BOLD);
    }

    // 代码块相关行
    if trimmed.starts_with('`') || trimmed.starts_with("```") {
        return Style::default()
            .fg(ratatui::style::Color::Yellow)
            .add_modifier(ratatui::style::Modifier::DIM);
    }

    // 列表项
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("• ") {
        return style_assistant_prose();
    }

    // 引用块
    if trimmed.starts_with('>') {
        return Style::default().fg(ratatui::style::Color::Rgb(170, 160, 185)); // 灰紫色
    }

    // 默认助手文本
    style_assistant_prose()
}

fn looks_like_slash_help_catalog_row(s: &str) -> bool {
    if !s.starts_with('/') {
        return false;
    }
    s.contains("  ") && (s.contains("local") || s.contains("runtime") || s.contains("prompt"))
}

#[cfg(test)]
mod stream_transcript_tests {
    #![allow(dead_code)] // 部分断言仍引用仅测试可见的旧 tail 辅助函数
    use super::{
        repl_stream_transcript_bottom_padded, sanitize_stream_transcript_visual_noise,
        scrub_stream_transcript_llm_raw_dumps,
    };
    use crate::repl::dock_render::{
        repl_dock_compute_natural, stream_dock_activity_prefix, ReplDockLayout,
    };
    use crate::repl::line_state::{ReplCtl, ReplLineState};
    use crate::repl::stream_events::{handle_event, stream_repl_accept_key_event};
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

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
    fn ctrl_c_while_executing_is_cooperative_cancel_not_eof() {
        let mut st = ReplLineState::default();
        st.executing_since = Some(std::time::Instant::now());
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let ctl = handle_event(Event::Key(key), &mut st).unwrap();
        assert!(matches!(ctl, ReplCtl::CooperativeCancelTurn));
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
    fn stream_dock_prompt_has_fixed_top_and_bottom_rule_rows() {
        let st = ReplLineState::default();
        let nat = repl_dock_compute_natural(80, &st, ReplDockLayout);
        assert_eq!(nat.rule_top_h, 1);
        assert_eq!(nat.rule_bottom_h, 1);
    }

    #[test]
    fn stream_dock_hud_one_row_executing_two_rows_when_pending_approval() {
        let mut st = ReplLineState::default();
        st.dock_status = "google · model · agent · on".into();
        st.dock_footer_left = "ctx — · ? help".into();
        st.executing_since = Some(std::time::Instant::now());
        let nat = repl_dock_compute_natural(80, &st, ReplDockLayout);
        assert_eq!(nat.hud_h, 1, "pure execute: single ✶ row, no ⎿ tips");
        assert_eq!(nat.rule_top_h, 1, "prompt 上横线固定，与 HUD 并存");
        assert_eq!(nat.rule_bottom_h, 1);
        assert_eq!(nat.status_h, 1);
        assert!(
            stream_dock_activity_prefix(&st).is_empty(),
            "thinking lives in HUD row, not status prefix"
        );

        let (reply_tx, _rx) = tokio::sync::oneshot::channel();
        st.pending_approval = Some(crate::term::approval::PendingApproval {
            tool: "bash".into(),
            input_preview: "{}".into(),
            reply: reply_tx,
        });
        let nat2 = repl_dock_compute_natural(80, &st, ReplDockLayout);
        assert_eq!(nat2.hud_h, 2, "approval menu: ✶ + ⎿ like fullscreen TUI");
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
    fn styled_transcript_marks_error_and_user_lines() {
        use ratatui::style::Color;
        let body = "❯ hi\nTurn failed: x\nplain\n";
        let t = super::stream_transcript_plain_to_styled_text(body);
        let lines: Vec<_> = t.lines.iter().collect();
        assert_eq!(lines.len(), 3);
        let user_fg = if std::env::var_os("NO_COLOR").is_some() {
            Some(Color::Reset)
        } else {
            Some(Color::Rgb(255, 140, 66))
        };
        assert_eq!(lines[0].spans[0].style.fg, user_fg);
        let err_fg = if std::env::var_os("NO_COLOR").is_some() {
            Some(Color::Reset)
        } else {
            Some(Color::Red)
        };
        assert_eq!(lines[1].spans[0].style.fg, err_fg);
        let plain_fg = if std::env::var_os("NO_COLOR").is_some() {
            Some(Color::Reset)
        } else {
            Some(Color::White)
        };
        assert_eq!(lines[2].spans[0].style.fg, plain_fg);
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
        let body = crate::repl::inline::tail_for_display(&scrubbed, logical_max);
        assert!(
            !body.contains("\"code\": 400"),
            "tail view should not show JSON after full scrub, got: {body:?}"
        );
    }
}

/// 键盘与审批/选题条：`ratatui` 主循环无 TTY E2E，这里用事件原子测覆盖关键路径。
#[cfg(test)]
mod stream_repl_keyboard_tests {
    use crate::repl::dock_render::{repl_dock_height, ReplDockLayout};
    use crate::repl::line_state::{ReplCtl, ReplLineState};
    use crate::repl::stream_events::{
        apply_stream_approval_key, apply_stream_user_question_key, handle_event,
    };
    use crate::term::{ApprovalDecision, PendingApproval, PendingUserQuestion};
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
    use ratatui::layout::Rect;

    #[test]
    fn key_release_does_not_clear_on_ctrl_u() {
        let mut st = ReplLineState::default();
        st.input.set_from_str("abc");
        let mut k = KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL);
        k.kind = KeyEventKind::Release;
        handle_event(Event::Key(k), &mut st).unwrap();
        assert_eq!(st.input.as_string(), "abc");
    }

    #[test]
    fn ctrl_u_clears_input_and_resets_slash() {
        let mut st = ReplLineState::default();
        st.input.set_from_str("/h");
        st.slash_pick = 3;
        handle_event(
            Event::Key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL)),
            &mut st,
        )
        .unwrap();
        assert!(st.input.as_string().is_empty());
        assert_eq!(st.slash_pick, 0);
        assert!(!st.slash_suppress);
    }

    #[test]
    fn page_up_down_adjusts_transcript_scroll() {
        let mut st = ReplLineState::default();
        st.stream_transcript_viewport_h = 20;
        handle_event(
            Event::Key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE)),
            &mut st,
        )
        .unwrap();
        assert_eq!(st.stream_transcript_scroll, 20);
        handle_event(
            Event::Key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE)),
            &mut st,
        )
        .unwrap();
        assert_eq!(st.stream_transcript_scroll, 0);
    }

    #[test]
    fn ctrl_home_end_jump_transcript_scroll() {
        let mut st = ReplLineState::default();
        handle_event(
            Event::Key(KeyEvent::new(KeyCode::Home, KeyModifiers::CONTROL)),
            &mut st,
        )
        .unwrap();
        assert_eq!(st.stream_transcript_scroll, usize::MAX);
        handle_event(
            Event::Key(KeyEvent::new(KeyCode::End, KeyModifiers::CONTROL)),
            &mut st,
        )
        .unwrap();
        assert_eq!(st.stream_transcript_scroll, 0);
    }

    #[test]
    fn shift_enter_inserts_newline_without_submit() {
        let mut st = ReplLineState::default();
        st.input.set_from_str("a");
        handle_event(
            Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT)),
            &mut st,
        )
        .unwrap();
        assert_eq!(st.input.as_string(), "a\n");
    }

    #[test]
    fn empty_enter_yields_continue_not_submit() {
        let mut st = ReplLineState::default();
        st.input.set_from_str("   ");
        let ctl = handle_event(
            Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            &mut st,
        )
        .unwrap();
        assert!(matches!(ctl, ReplCtl::Continue));
        assert!(st.input.as_string().is_empty());
    }

    #[test]
    fn slash_tab_does_not_complete_in_stream_repl() {
        let mut st = ReplLineState::default();
        st.input.set_from_str("/he");
        handle_event(
            Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)),
            &mut st,
        )
        .unwrap();
        assert_eq!(st.input.as_string(), "/he");
        assert!(!st.slash_suppress);
    }

    #[test]
    fn slash_down_with_empty_history_keeps_input() {
        let mut st = ReplLineState::default();
        st.input.set_from_str("/");
        handle_event(
            Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
            &mut st,
        )
        .unwrap();
        assert_eq!(st.slash_pick, 0);
        assert_eq!(st.input.as_string(), "/");
    }

    #[test]
    fn paste_sanitized_inserts_and_resets_slash() {
        let mut st = ReplLineState::default();
        st.slash_pick = 2;
        handle_event(Event::Paste("hello".into()), &mut st).unwrap();
        assert_eq!(st.input.as_string(), "hello");
        assert_eq!(st.slash_pick, 0);
    }

    #[test]
    fn repl_dock_height_respects_terminal_rows() {
        let st = ReplLineState::default();
        let area = Rect::new(0, 0, 80, 12);
        let h = repl_dock_height(area, &st, ReplDockLayout);
        assert!(h <= area.height);
        assert!(h >= 1);
    }

    #[tokio::test]
    async fn approval_y_sends_allow_once() {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let mut st = ReplLineState::default();
        st.pending_approval = Some(PendingApproval {
            tool: "bash".into(),
            input_preview: "{}".into(),
            reply: tx,
        });
        assert!(apply_stream_approval_key(
            &mut st,
            KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE)
        ));
        assert!(st.pending_approval.is_none());
        assert!(matches!(rx.await.unwrap(), ApprovalDecision::AllowOnce));
    }

    #[tokio::test]
    async fn approval_enter_respects_menu_index() {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let mut st = ReplLineState::default();
        st.approval_menu_selected = 2;
        st.pending_approval = Some(PendingApproval {
            tool: "bash".into(),
            input_preview: "{}".into(),
            reply: tx,
        });
        assert!(apply_stream_approval_key(
            &mut st,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)
        ));
        assert!(matches!(rx.await.unwrap(), ApprovalDecision::Deny));
    }

    #[tokio::test]
    async fn approval_unknown_key_keeps_pending() {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let mut st = ReplLineState::default();
        st.pending_approval = Some(PendingApproval {
            tool: "bash".into(),
            input_preview: "{}".into(),
            reply: tx,
        });
        assert!(apply_stream_approval_key(
            &mut st,
            KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)
        ));
        assert!(st.pending_approval.is_some());
        std::mem::drop(rx);
    }

    #[tokio::test]
    async fn user_question_down_enter_second_option() {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let mut st = ReplLineState::default();
        st.pending_user_question = Some(PendingUserQuestion {
            header: "h".into(),
            question: "q".into(),
            option_labels: vec!["first".into(), "second".into()],
            option_descriptions: vec![String::new(), String::new()],
            multi_select: false,
            reply: tx,
        });
        assert!(apply_stream_user_question_key(
            &mut st,
            KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)
        ));
        assert_eq!(st.user_question_menu_selected, 1);
        assert!(apply_stream_user_question_key(
            &mut st,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)
        ));
        assert_eq!(rx.await.unwrap().unwrap(), vec!["second"]);
    }

    #[tokio::test]
    async fn user_question_esc_cancels() {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let mut st = ReplLineState::default();
        st.pending_user_question = Some(PendingUserQuestion {
            header: "h".into(),
            question: "q".into(),
            option_labels: vec!["only".into()],
            option_descriptions: vec![String::new()],
            multi_select: false,
            reply: tx,
        });
        assert!(apply_stream_user_question_key(
            &mut st,
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)
        ));
        assert!(rx.await.unwrap().is_err());
    }

    #[test]
    fn resize_yields_continue() {
        let mut st = ReplLineState::default();
        let ctl = handle_event(Event::Resize(100, 40), &mut st).unwrap();
        assert!(matches!(ctl, ReplCtl::Continue));
    }

    #[test]
    fn ctrl_l_requests_clear_session() {
        let mut st = ReplLineState::default();
        st.input.set_from_str("noise");
        let ctl = handle_event(
            Event::Key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::CONTROL)),
            &mut st,
        )
        .unwrap();
        assert!(matches!(ctl, ReplCtl::ClearSession));
        assert_eq!(st.input.as_string(), "noise");
    }

    #[test]
    fn ctrl_c_empty_input_eof() {
        let mut st = ReplLineState::default();
        let ctl = handle_event(
            Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            &mut st,
        )
        .unwrap();
        assert!(matches!(ctl, ReplCtl::Eof));
    }

    #[test]
    fn ctrl_c_nonempty_clears_input() {
        let mut st = ReplLineState::default();
        st.input.set_from_str("x");
        let ctl = handle_event(
            Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            &mut st,
        )
        .unwrap();
        assert!(matches!(ctl, ReplCtl::Continue));
        assert!(st.input.as_string().is_empty());
    }

    #[test]
    fn ctrl_d_empty_eof() {
        let mut st = ReplLineState::default();
        let ctl = handle_event(
            Event::Key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL)),
            &mut st,
        )
        .unwrap();
        assert!(matches!(ctl, ReplCtl::Eof));
    }

    #[test]
    fn ctrl_d_deletes_forward() {
        let mut st = ReplLineState::default();
        st.input.set_from_str("ab");
        st.input.move_home();
        handle_event(
            Event::Key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL)),
            &mut st,
        )
        .unwrap();
        assert_eq!(st.input.as_string(), "b");
    }

    #[test]
    fn esc_does_not_toggle_slash_suppress_in_stream_repl() {
        let mut st = ReplLineState::default();
        st.input.set_from_str("/he");
        handle_event(
            Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            &mut st,
        )
        .unwrap();
        assert!(!st.slash_suppress);
    }

    #[test]
    fn esc_does_not_suppress_when_cursor_not_on_first_line() {
        let mut st = ReplLineState::default();
        st.input.set_from_str("/he\n");
        assert_eq!(st.input.as_string(), "/he\n");
        handle_event(
            Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            &mut st,
        )
        .unwrap();
        assert!(!st.slash_suppress);
    }

    #[test]
    fn slash_arrow_keys_use_history_not_pick_in_stream_repl() {
        let mut st = ReplLineState::default();
        st.input.set_from_str("/");
        handle_event(
            Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
            &mut st,
        )
        .unwrap();
        assert_eq!(st.slash_pick, 0);
        handle_event(
            Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
            &mut st,
        )
        .unwrap();
        assert_eq!(st.slash_pick, 0);
    }

    #[test]
    fn slash_backtab_is_no_op_in_stream_repl() {
        let mut st = ReplLineState::default();
        st.input.set_from_str("/");
        st.slash_pick = 2;
        handle_event(
            Event::Key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT)),
            &mut st,
        )
        .unwrap();
        assert_eq!(st.slash_pick, 2);
    }

    #[test]
    fn tab_without_slash_is_no_op() {
        let mut st = ReplLineState::default();
        st.input.set_from_str("plain");
        handle_event(
            Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)),
            &mut st,
        )
        .unwrap();
        assert_eq!(st.input.as_string(), "plain");
    }

    #[test]
    fn enter_submits_partial_slash_literal() {
        let mut st = ReplLineState::default();
        st.input.set_from_str("/he");
        let ctl = handle_event(
            Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            &mut st,
        )
        .unwrap();
        let ReplCtl::Submit(s) = ctl else {
            panic!("expected Submit after /he + Enter");
        };
        assert_eq!(s, "/he");
    }

    #[test]
    fn repeat_enter_does_not_submit() {
        let mut st = ReplLineState::default();
        st.input.set_from_str("x");
        let mut k = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        k.kind = KeyEventKind::Repeat;
        let ctl = handle_event(Event::Key(k), &mut st).unwrap();
        assert!(matches!(ctl, ReplCtl::Continue));
        assert_eq!(st.input.as_string(), "x");
    }

    #[test]
    fn control_char_insert_ignored() {
        let mut st = ReplLineState::default();
        st.input.set_from_str("a");
        handle_event(
            Event::Key(KeyEvent::new(KeyCode::Char('\t'), KeyModifiers::NONE)),
            &mut st,
        )
        .unwrap();
        assert_eq!(st.input.as_string(), "a");
    }

    #[test]
    fn history_restored_on_up_after_submit() {
        let mut st = ReplLineState::default();
        st.input.set_from_str("line-a");
        let ctl = handle_event(
            Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            &mut st,
        )
        .unwrap();
        assert!(matches!(ctl, ReplCtl::Submit(_)));
        assert!(st.input.as_string().is_empty());
        handle_event(
            Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
            &mut st,
        )
        .unwrap();
        assert_eq!(st.input.as_string(), "line-a");
    }

    #[test]
    fn history_dedupes_consecutive_identical_submit() {
        let mut st = ReplLineState::default();
        for _ in 0..2 {
            st.input.set_from_str("same");
            let _ = handle_event(
                Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
                &mut st,
            )
            .unwrap();
        }
        assert_eq!(st.input_history.len(), 1);
    }

    #[test]
    fn left_right_home_end_move_cursor() {
        let mut st = ReplLineState::default();
        st.input.set_from_str("abc");
        st.input.move_home();
        handle_event(
            Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
            &mut st,
        )
        .unwrap();
        handle_event(
            Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
            &mut st,
        )
        .unwrap();
        handle_event(
            Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
            &mut st,
        )
        .unwrap();
        assert_eq!(st.input.cursor, 1);
        handle_event(
            Event::Key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)),
            &mut st,
        )
        .unwrap();
        assert_eq!(st.input.cursor, 0);
        handle_event(
            Event::Key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE)),
            &mut st,
        )
        .unwrap();
        assert_eq!(st.input.cursor, 3);
    }

    #[test]
    fn backspace_delete_reset_slash_state() {
        let mut st = ReplLineState::default();
        st.input.set_from_str("/x");
        st.slash_pick = 2;
        handle_event(
            Event::Key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)),
            &mut st,
        )
        .unwrap();
        assert_eq!(st.slash_pick, 0);
    }

    #[test]
    fn repl_dock_with_pending_approval_still_bounded() {
        let (tx, _rx) = tokio::sync::oneshot::channel();
        let mut st = ReplLineState::default();
        st.input.set_from_str("/");
        st.pending_approval = Some(PendingApproval {
            tool: "bash".into(),
            input_preview: "{}".into(),
            reply: tx,
        });
        let area = Rect::new(0, 0, 80, 24);
        let h = repl_dock_height(area, &st, ReplDockLayout);
        assert!(h <= area.height);
    }

    #[test]
    fn apply_stream_approval_key_false_without_pending() {
        let mut st = ReplLineState::default();
        assert!(!apply_stream_approval_key(
            &mut st,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)
        ));
    }

    #[tokio::test]
    async fn approval_p_sends_allow_tool_for_project() {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let mut st = ReplLineState::default();
        st.pending_approval = Some(PendingApproval {
            tool: "bash".into(),
            input_preview: "{}".into(),
            reply: tx,
        });
        assert!(apply_stream_approval_key(
            &mut st,
            KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE)
        ));
        assert!(matches!(
            rx.await.unwrap(),
            ApprovalDecision::AllowToolForProject
        ));
    }

    #[tokio::test]
    async fn approval_n_sends_deny() {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let mut st = ReplLineState::default();
        st.pending_approval = Some(PendingApproval {
            tool: "bash".into(),
            input_preview: "{}".into(),
            reply: tx,
        });
        assert!(apply_stream_approval_key(
            &mut st,
            KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE)
        ));
        assert!(matches!(rx.await.unwrap(), ApprovalDecision::Deny));
    }

    #[tokio::test]
    async fn approval_down_cycles_menu_without_send() {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let mut st = ReplLineState::default();
        st.approval_menu_selected = 0;
        st.pending_approval = Some(PendingApproval {
            tool: "bash".into(),
            input_preview: "{}".into(),
            reply: tx,
        });
        assert!(apply_stream_approval_key(
            &mut st,
            KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)
        ));
        assert_eq!(st.approval_menu_selected, 1);
        assert!(st.pending_approval.is_some());
        std::mem::drop(rx);
    }

    #[tokio::test]
    async fn user_question_up_wraps_from_zero() {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let mut st = ReplLineState::default();
        st.user_question_menu_selected = 0;
        st.pending_user_question = Some(PendingUserQuestion {
            header: String::new(),
            question: String::new(),
            option_labels: vec!["a".into(), "b".into()],
            option_descriptions: vec![String::new(), String::new()],
            multi_select: false,
            reply: tx,
        });
        assert!(apply_stream_user_question_key(
            &mut st,
            KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)
        ));
        assert_eq!(st.user_question_menu_selected, 1);
        assert!(st.pending_user_question.is_some());
        std::mem::drop(rx);
    }
}
