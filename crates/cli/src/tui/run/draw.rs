//! 单帧 TUI 绘制（ratatui），与事件循环解耦以便阅读与单测。

use crate::i18n::{tr, tr_args};
use crate::md_tui::{
    pad_end_to_display_width, text_display_width, truncate_to_display_width, wrap_string_to_width,
};
use crate::slash_commands;
use crate::tui::approval::PendingApproval;
use crate::tui::chrome::sidebar_help_text;
use crate::tui::input::{prompt_multiline_lines_and_cursor, InputState, RevSearchState};
use crate::tui::pet;
use crate::tui::styles::*;
use crate::tui::transcript::{layout_workspace, TranscriptEntry, WorkspaceLiveLayout};
use crate::tui::util::{
    bottom_align_viewport_lines, top_align_viewport_lines, transcript_first_visible,
    truncate_preview,
};
use crate::tui::PendingUserQuestion;
use anycode_core::AgentType;
use fluent_bundle::FluentArgs;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use std::cell::Cell;
use std::collections::HashSet;

/// Dock 内 Buddy 列宽（窄屏回退：无 Prompt 上 HUD 时使用）。
const DOCK_BUDDY_AREA_WIDTH: u16 = 14;
/// Prompt 上方 HUD 右侧 Buddy 列宽。
const PROMPT_HUD_BUDDY_W: u16 = 14;

/// 脚标最多占用的行数（防止极窄终端无限增高底栏）。
const FOOTER_LINES_CAP: usize = 10;

/// 与 Workspace / Dock 左右边对齐的整行横线。
fn horizontal_rule_line(width: u16) -> Line<'static> {
    let w = width.max(1) as usize;
    Line::from(Span::styled("─".repeat(w), style_dim()))
}

/// 欢迎卡底边，与 Dock 上缘衔接（`╰─╯`），用于非三段式紧凑栈。
fn home_welcome_card_bottom_line(width: u16) -> Line<'static> {
    let w = width.max(4) as usize;
    let inner = w.saturating_sub(2).max(1);
    Line::from(Span::styled(
        format!("╰{}╯", "─".repeat(inner)),
        style_welcome_border(),
    ))
}

/// 欢迎卡：仅标题行 + 版本后缀，不展示 provider/model（避免顶栏重复脚标信息）。
/// `closes_bottom=true` 时自带底边（三段式：顶卡 / 中空 / Dock）。
fn home_welcome_card_lines(width: u16, closes_bottom: bool) -> Vec<Line<'static>> {
    let w = width.max(4) as usize;
    let inner = w.saturating_sub(2).max(1);
    let b = style_welcome_border();
    let mut ver = FluentArgs::new();
    ver.set("version", env!("CARGO_PKG_VERSION"));
    let ver_suffix = tr_args("tui-brand-version-suffix", &ver);

    let mut v = vec![
        Line::from(Span::styled(format!("╭{}╮", "─".repeat(inner)), b)),
        Line::from(vec![
            Span::styled("│", b),
            Span::styled("  ", style_dim()),
            Span::styled("● ", style_brand()),
            Span::styled("anyCode", style_brand()),
            Span::styled(ver_suffix, style_dim()),
        ]),
    ];
    if closes_bottom {
        v.push(Line::from(Span::styled(
            format!("╰{}╯", "─".repeat(inner)),
            b,
        )));
    }
    v
}

/// 横线下方两行：`✶` 活动态 + `⎿` 轮换提示；右侧可选 Buddy 双行。
fn render_prompt_hud_stacked(
    f: &mut Frame<'_>,
    area: Rect,
    buddy_column: bool,
    pet_anim_frame: u64,
    executing: bool,
    pending_approval: bool,
    working_elapsed_secs: Option<u64>,
    hud_tip_slot: usize,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let activity = crate::tui::hud_text::prompt_hud_activity_text(
        pending_approval,
        executing,
        working_elapsed_secs,
    );
    let tip = crate::tui::hud_text::hud_tip_rotated(hud_tip_slot);
    let (text_r, buddy_r) = if buddy_column && area.width > PROMPT_HUD_BUDDY_W.saturating_add(24) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(8), Constraint::Length(PROMPT_HUD_BUDDY_W)])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };
    let hud_text = Text::from(vec![
        Line::from(vec![
            Span::styled("✶ ", style_dim()),
            Span::styled(activity, style_assistant()),
        ]),
        Line::from(vec![
            Span::styled("⎿ ", style_dim()),
            Span::styled(tip, style_dim()),
        ]),
    ]);
    f.render_widget(Paragraph::new(hud_text).wrap(Wrap { trim: true }), text_r);
    if let Some(br) = buddy_r {
        let pet_lines = pet::pet_hud_lines(pet_anim_frame, executing);
        let pet_par = Paragraph::new(Text::from(pet_lines))
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(Wrap { trim: false });
        f.render_widget(pet_par, br);
    }
}

/// 供脚标折行高度预算与绘制共用（与 [`DrawFrameCtx`] 字段对应）。
pub(super) struct FooterLayoutInput<'a> {
    pub permission_mode: &'a str,
    pub require_approval: bool,
    pub llm_provider: &'a str,
    pub llm_model: &'a str,
    pub transcript_scroll_up: usize,
    pub debug: bool,
    pub last_key: Option<&'a str>,
    /// 与 [`crate::app_config::effective_session_context_window_tokens`] 一致。
    pub context_window_tokens: u32,
    pub last_max_input_tokens: u32,
    /// 最近一轮 LLM 调用聚合的 output tokens（与流式 HUD 同源）。
    pub last_output_tokens: u32,
}

fn footer_ctx_fragment(inp: &FooterLayoutInput<'_>) -> String {
    let win = inp.context_window_tokens;
    let mut base = if win == 0 {
        tr("tui-footer-ctx-unknown")
    } else if inp.last_max_input_tokens == 0 {
        let mut a = FluentArgs::new();
        a.set("win", win as i64);
        tr_args("tui-footer-ctx-zero", &a)
    } else {
        let pct = ((inp.last_max_input_tokens as f64 / win as f64) * 100.0).min(100.0);
        let mut a = FluentArgs::new();
        a.set("pct", (pct.round() as i64).max(0));
        a.set("win", win as i64);
        tr_args("tui-footer-ctx-pct", &a)
    };
    if inp.last_output_tokens > 0 {
        let mut a = FluentArgs::new();
        a.set(
            "k",
            crate::tui::hud_text::format_tokens_k_thousands(inp.last_output_tokens),
        );
        base.push_str(&tr_args("tui-footer-out-tokens", &a));
    }
    base
}

fn spans_flat_string(spans: &[Span<'static>]) -> String {
    spans.iter().map(|s| s.content.to_string()).collect()
}

fn footer_left_spans(inp: &FooterLayoutInput<'_>) -> Vec<Span<'static>> {
    let pipe = Style::default().fg(Color::DarkGray);
    let val = Style::default().fg(Color::White);
    let ctx = footer_ctx_fragment(inp);
    let appr = if inp.require_approval {
        tr("tui-approval-on-short")
    } else {
        tr("tui-approval-off-short")
    };
    let appr_st = if inp.require_approval {
        style_assistant()
    } else {
        style_dim()
    };

    let mut left: Vec<Span<'static>> = vec![
        Span::styled(ctx, style_dim()),
        Span::styled(" · ", pipe),
        Span::styled(tr("tui-hdr-permission"), style_dim()),
        Span::styled(String::from(inp.permission_mode), val),
        Span::styled(" · ", pipe),
        Span::styled(tr("tui-hdr-approval"), style_dim()),
        Span::styled(appr, appr_st),
        Span::styled(" · ", pipe),
        Span::styled(tr("tui-footer-help-hint"), style_dim()),
    ];
    if inp.transcript_scroll_up > 0 {
        let mut a = FluentArgs::new();
        a.set("n", inp.transcript_scroll_up as i64);
        left.push(Span::styled(" · ", pipe));
        left.push(Span::styled(
            tr_args("tui-footer-scroll-hint", &a),
            style_dim(),
        ));
    }
    if inp.debug {
        if let Some(k) = inp.last_key {
            left.push(Span::styled(" · ", pipe));
            left.push(Span::styled(
                format!("{}{}", tr("tui-hdr-key-prefix"), k),
                style_dim(),
            ));
        }
    }
    left
}

fn footer_right_spans(inp: &FooterLayoutInput<'_>) -> Vec<Span<'static>> {
    let val = Style::default().fg(Color::White);
    let right_txt = format!("{} · {}", inp.llm_provider.trim(), inp.llm_model.trim());
    vec![Span::styled(right_txt, val)]
}

/// 左 token/权限/?；右对齐 提供商·模型。
fn footer_span_rows_combined(width: usize, inp: &FooterLayoutInput<'_>) -> Vec<Span<'static>> {
    let left = footer_left_spans(inp);
    let right = footer_right_spans(inp);
    let lw = text_display_width(&spans_flat_string(&left));
    let rw = text_display_width(&spans_flat_string(&right));
    let gap = width.saturating_sub(lw + rw).min(200);
    let gap = gap.max(1);
    let mut out = left;
    out.push(Span::styled(" ".repeat(gap), Style::default()));
    out.extend(right);
    out
}

/// 将一段 `Span` 序列按显示宽度折成多行（单段过长时按字符切分）。
fn wrap_span_sequence(spans: Vec<Span<'static>>, max_w: usize) -> Vec<Line<'static>> {
    let max_w = max_w.max(1);
    let mut out: Vec<Line<'static>> = vec![];
    let mut cur: Vec<Span<'static>> = vec![];
    let mut cur_w = 0usize;

    for sp in spans {
        let st = sp.style;
        let content = sp.content.to_string();
        let sw = text_display_width(&content);
        if sw > max_w {
            if !cur.is_empty() {
                out.push(Line::from(std::mem::take(&mut cur)));
                cur_w = 0;
            }
            for piece in wrap_string_to_width(&content, max_w) {
                out.push(Line::from(Span::styled(piece, st)));
            }
            continue;
        }
        if cur_w + sw > max_w && !cur.is_empty() {
            out.push(Line::from(std::mem::take(&mut cur)));
            cur_w = 0;
        }
        cur_w += sw;
        cur.push(Span::styled(content, st));
    }
    if !cur.is_empty() {
        out.push(Line::from(cur));
    }
    if out.is_empty() {
        out.push(Line::from(""));
    }
    out
}

fn build_tui_footer_lines(width: u16, inp: &FooterLayoutInput<'_>) -> Vec<Line<'static>> {
    let w = width.max(1) as usize;
    let spans = footer_span_rows_combined(w, inp);
    let mut lines = wrap_span_sequence(spans, w);
    if lines.len() > FOOTER_LINES_CAP {
        lines.truncate(FOOTER_LINES_CAP);
    }
    lines
}

/// 脚标折行后的行数（至少 1，与 `build_tui_footer_lines` 一致），供 `loop_inner` 预留底栏高度。
pub(super) fn footer_wrapped_line_count(width: u16, inp: &FooterLayoutInput<'_>) -> u16 {
    let n = build_tui_footer_lines(width, inp).len().max(1);
    n.min(FOOTER_LINES_CAP) as u16
}

/// 空会话顶栏：品牌色 + 版本，下行 dim 显示 provider·model（无大块 Workspace）。
fn compact_brand_banner_lines(
    frame_w: usize,
    llm_provider: &str,
    llm_model: &str,
) -> Vec<Line<'static>> {
    let fw = frame_w.max(12);
    let mut ver = FluentArgs::new();
    ver.set("version", env!("CARGO_PKG_VERSION"));
    let line1 = Line::from(vec![
        Span::styled("  ", style_dim()),
        Span::styled("● ", style_brand()),
        Span::styled("anyCode", style_brand()),
        Span::styled(tr_args("tui-brand-version-suffix", &ver), style_dim()),
    ]);
    let sub = format!("  {} · {}", llm_provider.trim(), llm_model.trim());
    let sub = truncate_preview(&sub, fw.saturating_sub(1));
    vec![line1, Line::from(Span::styled(sub, style_dim()))]
}

pub(super) struct DrawFrameCtx<'a> {
    pub bottom_h: u16,
    /// 脚标折行后的行数（由 `loop_inner` 调用 `footer_wrapped_line_count` 与绘制保持一致）。
    pub footer_line_count: u16,
    /// 宽度足够时在 Dock 内、Prompt 右侧显示 Buddy。
    pub show_buddy: bool,
    /// 宠物动画帧 `0..4`（`executing` 时由外层用时间驱动）。
    pub pet_anim_frame: u64,
    /// 保留供调用方与后续 HUD；当前帧绘制未使用。
    #[allow(dead_code)]
    pub agent_type: &'a AgentType,
    pub permission_mode: &'a str,
    pub require_approval: bool,
    pub llm_provider: &'a str,
    pub llm_model: &'a str,
    pub debug: bool,
    pub last_key: Option<&'a str>,
    pub pending_approval: Option<&'a PendingApproval>,
    pub pending_user_question: Option<&'a PendingUserQuestion>,
    /// `0..3`：与 `event` 中审批菜单一致。
    pub approval_menu_selected: usize,
    pub user_question_menu_selected: usize,
    pub executing: bool,
    /// `executing` 为 true 时，自 `executing_since` 起经过的整秒数（用于顶栏，避免子秒刷新）。
    pub working_elapsed_secs: Option<u64>,
    pub help_open: bool,
    pub transcript: &'a [TranscriptEntry],
    pub transcript_scroll_up: usize,
    pub rev_search: Option<&'a RevSearchState>,
    /// 首行 `/` 补全时，候选列表中高亮项（与 `slash_suggestions_for_first_line` 配合）。
    pub slash_suggest_pick: usize,
    /// 采纳补全后隐藏列表，直至用户再次编辑。
    pub slash_suggest_suppress: bool,
    pub input: &'a InputState,
    pub input_history: &'a [String],
    pub workspace_cache_lines: &'a mut Vec<Line<'static>>,
    pub workspace_cache_gen: &'a mut u64,
    pub workspace_cache_w: &'a mut usize,
    pub workspace_cache_fold_rev: &'a mut u64,
    pub workspace_cache_executing: &'a mut bool,
    pub workspace_cache_working_secs: &'a mut Option<u64>,
    pub workspace_cache_pulse_frame: &'a mut u64,
    pub transcript_gen: u64,
    pub fold_layout_rev: u64,
    pub expanded_tool_folds: &'a HashSet<u64>,
    pub main_avail_cell: &'a Cell<usize>,
    pub workspace_line_count: &'a Cell<usize>,
    /// 底栏 status line（`statusLine.command` 或内置行）。
    pub status_line_show: bool,
    pub status_line_text: &'a str,
    pub status_line_padding: u16,
    /// 首次 Ctrl+C 后：Dock 内提示再按一次退出，并展示 `anycode --resume <id>`。
    pub quit_confirm_pending: bool,
    /// `true`：DEC 备用屏；`false`：主缓冲（与 shell 回滚混排）。主缓冲空会话时顶对齐品牌，减少上方空白。
    pub used_alternate_screen: bool,
    pub tui_resume_session_id: &'a str,
    /// `0..6`（与 [`crate::tui::hud_text::HUD_TIP_COUNT`] 一致），Prompt 上 `⎿` 行轮换提示。
    pub hud_tip_slot: usize,
    pub last_max_input_tokens: u32,
    pub last_output_tokens: u32,
    pub context_window_tokens: u32,
}

pub(super) fn draw_tui_frame(f: &mut Frame<'_>, ctx: DrawFrameCtx<'_>) {
    let DrawFrameCtx {
        bottom_h,
        footer_line_count,
        show_buddy,
        pet_anim_frame,
        agent_type: _,
        permission_mode,
        require_approval,
        llm_provider,
        llm_model,
        debug,
        last_key,
        pending_approval,
        pending_user_question,
        approval_menu_selected,
        user_question_menu_selected,
        executing,
        working_elapsed_secs,
        help_open,
        transcript,
        transcript_scroll_up,
        rev_search,
        slash_suggest_pick,
        slash_suggest_suppress,
        input,
        input_history,
        workspace_cache_lines,
        workspace_cache_gen,
        workspace_cache_w,
        workspace_cache_fold_rev,
        workspace_cache_executing,
        workspace_cache_working_secs,
        workspace_cache_pulse_frame,
        transcript_gen,
        fold_layout_rev,
        expanded_tool_folds,
        main_avail_cell,
        workspace_line_count,
        status_line_show,
        status_line_text,
        status_line_padding,
        quit_confirm_pending,
        used_alternate_screen,
        tui_resume_session_id,
        hud_tip_slot,
        last_max_input_tokens,
        last_output_tokens,
        context_window_tokens,
    } = ctx;

    // 以 Frame 实际区域为准（ratatui 0.24 为 `size()`），避免 `terminal.size()` 与缓冲不一致时整屏错位。
    let raw = f.size();
    let size = Rect::new(0, 0, raw.width, raw.height);
    let bottom_h = bottom_h.min(size.height.saturating_sub(1)).max(1);

    let prompt_hud_active =
        pending_approval.is_some() || pending_user_question.is_some() || executing;
    let hud_rows_effective: u16 = if prompt_hud_active { 2 } else { 0 };

    let slash_candidates = if pending_approval.is_none()
        && pending_user_question.is_none()
        && rev_search.is_none()
        && !slash_suggest_suppress
    {
        slash_commands::slash_suggestions_for_first_line(&input.as_string())
    } else {
        Vec::new()
    };

    let frame_w = size.width.max(1) as usize;

    // --- Workspace 缓存宽度与主区同宽（全幅） ---
    if *workspace_cache_gen != transcript_gen
        || *workspace_cache_w != frame_w
        || *workspace_cache_fold_rev != fold_layout_rev
        || *workspace_cache_executing != executing
        || *workspace_cache_working_secs != working_elapsed_secs
        || *workspace_cache_pulse_frame != pet_anim_frame
    {
        *workspace_cache_lines = layout_workspace(
            transcript,
            frame_w,
            expanded_tool_folds,
            WorkspaceLiveLayout {
                executing,
                working_elapsed_secs,
                pulse_frame: pet_anim_frame,
                ..Default::default()
            },
        );
        *workspace_cache_gen = transcript_gen;
        *workspace_cache_w = frame_w;
        *workspace_cache_fold_rev = fold_layout_rev;
        *workspace_cache_executing = executing;
        *workspace_cache_working_secs = working_elapsed_secs;
        *workspace_cache_pulse_frame = pet_anim_frame;
    }

    let empty_home_compact = !used_alternate_screen && transcript.is_empty() && !help_open;
    let empty_home_use_card =
        empty_home_compact && size.width >= 16 && 2u16.saturating_add(bottom_h) <= size.height;
    // 顶栏闭合卡 + Min(1) 留白 + 底栏（上卡 / 中空 / 下 Dock）；闭合卡现为 3 行（顶边 / 标题 / 底边）。
    let claude_three_band =
        empty_home_use_card && 3u16.saturating_add(bottom_h).saturating_add(1) <= size.height;

    let brand_banner = if transcript.is_empty() && !help_open {
        if empty_home_use_card {
            Some(home_welcome_card_lines(size.width, claude_three_band))
        } else {
            Some(compact_brand_banner_lines(frame_w, llm_provider, llm_model))
        }
    } else {
        None
    };

    if transcript.is_empty() && !help_open {
        workspace_line_count.set(brand_banner.as_ref().map(|l| l.len()).unwrap_or(2).max(1));
    } else if transcript.is_empty() && help_open {
        workspace_line_count.set(24);
    } else {
        workspace_line_count.set(workspace_cache_lines.len());
    }

    // 主区高度 = 屏高 − 底栏。主缓冲空会话且够高：三段式「顶栏闭合卡 / Min(1) 留白 / 底栏」对齐 Claude Code；否则沿用紧凑栈或满高主区。
    let bh_brand = if empty_home_compact {
        brand_banner
            .as_ref()
            .map(|l| l.len() as u16)
            .unwrap_or(2)
            .max(1)
    } else {
        0
    };
    let need_top_stack = bh_brand.saturating_add(bottom_h);
    let empty_home_tight_top = empty_home_compact && need_top_stack <= size.height;

    let cap_workspace = size.height.saturating_sub(bottom_h).max(1);

    let mut welcome_only_rect: Option<Rect> = None;

    let (mid, bottom) = if claude_three_band {
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(bh_brand),
                Constraint::Min(1),
                Constraint::Length(bottom_h),
            ])
            .split(size);
        welcome_only_rect = Some(outer[0]);
        main_avail_cell.set(outer[1].height as usize);
        (outer[1], outer[2])
    } else if empty_home_compact {
        let bh = bh_brand;
        let need = need_top_stack;
        if need > size.height {
            let workspace_h = size.height.saturating_sub(bottom_h).max(1);
            let outer = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(workspace_h),
                    Constraint::Length(bottom_h),
                ])
                .split(size);
            main_avail_cell.set(workspace_h as usize);
            (outer[0], outer[1])
        } else {
            let root = Rect::new(0, 0, size.width, need);
            let outer = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(bh), Constraint::Length(bottom_h)])
                .split(root);
            main_avail_cell.set(bh as usize);
            (outer[0], outer[1])
        }
    } else {
        let workspace_h = cap_workspace;
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(workspace_h),
                Constraint::Length(bottom_h),
            ])
            .split(size);
        main_avail_cell.set(workspace_h as usize);
        (outer[0], outer[1])
    };

    // --- 主区：对话流 / 帮助 / 顶栏品牌 ---

    if help_open {
        let help_block = Block::default()
            .borders(Borders::ALL)
            .border_style(style_dim())
            .title(Span::styled(tr("tui-help-panel-title"), style_brand()));
        main_avail_cell.set((help_block.inner(mid).height as usize).max(1));
        let help = Paragraph::new(sidebar_help_text())
            .block(help_block)
            .wrap(Wrap { trim: true });
        f.render_widget(help, mid);
    } else if !transcript.is_empty() {
        let main_rect = mid;
        let available_height = main_rect.height as usize;
        main_avail_cell.set(available_height.max(1));
        let avail_h = available_height.max(1);
        let start =
            transcript_first_visible(workspace_cache_lines.len(), avail_h, transcript_scroll_up);
        let end = (start + avail_h).min(workspace_cache_lines.len());
        let slice = workspace_cache_lines[start..end].to_vec();
        let body = if !used_alternate_screen && transcript_scroll_up == 0 {
            Text::from(top_align_viewport_lines(slice, avail_h))
        } else {
            Text::from(bottom_align_viewport_lines(
                slice,
                avail_h,
                transcript_scroll_up,
            ))
        };
        f.render_widget(Paragraph::new(body).wrap(Wrap { trim: false }), main_rect);
    } else if let Some(wrect) = welcome_only_rect {
        let lines = brand_banner.as_ref().expect("brand lines");
        let h = wrect.height as usize;
        let avail_h = h.max(1);
        let st = transcript_first_visible(lines.len(), avail_h, transcript_scroll_up);
        let en = (st + avail_h).min(lines.len());
        let slice = lines[st..en].to_vec();
        let body = top_align_viewport_lines(slice, avail_h);
        f.render_widget(
            Paragraph::new(Text::from(body)).wrap(Wrap { trim: false }),
            wrect,
        );
    } else {
        let main_rect = mid;
        let available_height = main_rect.height as usize;
        main_avail_cell.set(available_height.max(1));
        let avail_h = available_height.max(1);
        let lines = brand_banner.as_ref().expect("brand lines");
        let st = transcript_first_visible(lines.len(), avail_h, transcript_scroll_up);
        let en = (st + avail_h).min(lines.len());
        let slice = lines[st..en].to_vec();
        let body = if empty_home_tight_top {
            slice
        } else if used_alternate_screen || transcript_scroll_up > 0 {
            bottom_align_viewport_lines(slice, avail_h, transcript_scroll_up)
        } else {
            top_align_viewport_lines(slice, avail_h)
        };
        f.render_widget(
            Paragraph::new(Text::from(body)).wrap(Wrap { trim: false }),
            main_rect,
        );
    }

    // --- 底栏：分界横线 + 可选自定义 status + Prompt 上 HUD（执行/审批时 2 行）+ Prompt 上下横线 + 折行脚标 ---
    let status_rows: u16 = if status_line_show { 1 } else { 0 };
    let fh = footer_line_count.max(1);
    // 空闲且无 status/HUD：合并「顶横线」与「Prompt 上横线」，避免两条 ─ 紧贴；多出的 1 行给 Dock（与 loop_inner 的 bottom_h 一致）。
    let compact_bottom_chrome = status_rows == 0 && hud_rows_effective == 0;
    let bottom_split = if compact_bottom_chrome {
        let fixed_non_dock = 2u16.saturating_add(fh);
        // 与 loop_inner 的 dock_need 一致：不再强行 min(4)，避免 Prompt 与脚标之间垫空行。
        let dock_rows = bottom_h.saturating_sub(fixed_non_dock).max(1);
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(dock_rows),
                Constraint::Length(1),
                Constraint::Length(fh),
            ])
            .split(bottom)
    } else {
        let fixed_without_dock: u16 = 1u16
            .saturating_add(status_rows)
            .saturating_add(hud_rows_effective)
            .saturating_add(1)
            .saturating_add(1)
            .saturating_add(fh);
        let dock_rows = bottom_h.saturating_sub(fixed_without_dock).max(1);
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(status_rows),
                Constraint::Length(hud_rows_effective),
                Constraint::Length(1),
                Constraint::Length(dock_rows),
                Constraint::Length(1),
                Constraint::Length(fh),
            ])
            .split(bottom)
    };

    if empty_home_use_card && empty_home_tight_top && !claude_three_band {
        f.render_widget(
            Paragraph::new(home_welcome_card_bottom_line(bottom_split[0].width)),
            bottom_split[0],
        );
    } else {
        f.render_widget(
            Paragraph::new(horizontal_rule_line(bottom_split[0].width)),
            bottom_split[0],
        );
    }

    let show_buddy_here = show_buddy
        && executing
        && pending_approval.is_none()
        && pending_user_question.is_none()
        && rev_search.is_none()
        && slash_candidates.is_empty();
    // 紧凑底栏无 HUD 行：buddy 只能进 Dock 侧栏，不能进 Prompt HUD。
    let buddy_in_prompt_hud = if compact_bottom_chrome {
        false
    } else {
        show_buddy_here && bottom_split[2].width > PROMPT_HUD_BUDDY_W.saturating_add(24)
    };

    if !compact_bottom_chrome {
        if status_line_show {
            let pad = status_line_padding.min(48) as usize;
            let mut padded = String::new();
            padded.extend(std::iter::repeat_n(' ', pad));
            padded.push_str(status_line_text);
            let w = bottom_split[1].width.max(1) as usize;
            let disp = truncate_preview(&padded, w);
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(disp, style_dim())))
                    .wrap(Wrap { trim: true }),
                bottom_split[1],
            );
        }

        render_prompt_hud_stacked(
            f,
            bottom_split[2],
            buddy_in_prompt_hud,
            pet_anim_frame,
            executing,
            pending_approval.is_some() || pending_user_question.is_some(),
            working_elapsed_secs,
            hud_tip_slot,
        );

        f.render_widget(
            Paragraph::new(horizontal_rule_line(bottom_split[3].width)),
            bottom_split[3],
        );
    }

    let (dock_rect, footer_rect) = if compact_bottom_chrome {
        (bottom_split[1], bottom_split[3])
    } else {
        (bottom_split[4], bottom_split[6])
    };

    let (prompt_cell, buddy_cell) = if show_buddy_here
        && !buddy_in_prompt_hud
        && dock_rect.width >= DOCK_BUDDY_AREA_WIDTH.saturating_add(14)
    {
        let prompt_w = dock_rect.width.saturating_sub(DOCK_BUDDY_AREA_WIDTH);
        let buddy_x = dock_rect.x.saturating_add(prompt_w);
        (
            Rect {
                x: dock_rect.x,
                y: dock_rect.y,
                width: prompt_w,
                height: dock_rect.height,
            },
            Some(Rect {
                x: buddy_x,
                y: dock_rect.y,
                width: DOCK_BUDDY_AREA_WIDTH,
                height: dock_rect.height,
            }),
        )
    } else {
        (dock_rect, None)
    };

    let input_inner_w = prompt_cell.width.max(1);

    let mut input_lines: Vec<Line> = vec![];
    // 主输入区：`(Paragraph 内行下标, 行内显示宽度列)`，供 `Frame::set_cursor` 把硬件光标放到 Prompt 内（IME 预编辑跟随）。
    let mut prompt_hw_cursor: Option<(usize, u16)> = None;

    if let Some(q) = pending_user_question {
        input_lines.push(Line::from(Span::styled(
            tr("ask-user-title"),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )));
        let hdr = q.header.trim();
        if !hdr.is_empty() {
            input_lines.push(Line::from(Span::styled(hdr, style_dim())));
        }
        let qq = q.question.trim();
        if !qq.is_empty() {
            let preview_w = input_inner_w as usize;
            if text_display_width(qq) <= preview_w {
                input_lines.push(Line::from(Span::styled(qq, style_dim())));
            } else {
                for row in wrap_string_to_width(qq, preview_w.max(8)) {
                    input_lines.push(Line::from(Span::styled(row, style_dim())));
                }
            }
        }
        let n = q.option_labels.len().max(1);
        let pick = user_question_menu_selected % n;
        for (i, label) in q.option_labels.iter().enumerate() {
            let prefix = if i == pick { "❯ " } else { "  " };
            let st = if i == pick {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                style_dim()
            };
            let desc = q
                .option_descriptions
                .get(i)
                .map(|s| s.as_str())
                .unwrap_or("")
                .trim();
            let line = if desc.is_empty() {
                Line::from(vec![
                    Span::styled(prefix, st),
                    Span::styled(label.as_str(), st),
                ])
            } else {
                Line::from(vec![
                    Span::styled(prefix, st),
                    Span::styled(format!("{label} "), st),
                    Span::styled(desc, style_dim()),
                ])
            };
            input_lines.push(line);
        }
        input_lines.push(Line::from(vec![Span::styled(
            tr("ask-user-hint-arrows"),
            style_dim(),
        )]));
    } else if let Some(p) = pending_approval {
        input_lines.push(Line::from(Span::styled(
            tr("tui-approval-question"),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )));
        input_lines.push(Line::from(vec![
            Span::styled("⏺ ", style_tool()),
            Span::styled(
                format!("{} ", p.tool),
                style_warn().add_modifier(Modifier::BOLD),
            ),
            Span::styled(tr("tui-approval-pending"), style_dim()),
        ]));
        let preview_w = input_inner_w as usize;
        let pv = p.input_preview.as_str();
        if text_display_width(pv) <= preview_w {
            input_lines.push(Line::from(Span::styled(pv, style_dim())));
        } else {
            for row in wrap_string_to_width(pv, preview_w.max(8)) {
                input_lines.push(Line::from(Span::styled(row, style_dim())));
            }
        }
        let pick = approval_menu_selected % 3;
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
    } else if quit_confirm_pending {
        input_lines.push(Line::from(Span::styled(
            tr("tui-exit-press-again"),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
        input_lines.push(Line::from(vec![
            Span::styled(tr("tui-exit-resume-lead"), style_dim()),
            Span::styled(
                format!(" anycode --resume {tui_resume_session_id}"),
                style_dim(),
            ),
        ]));
    } else if let Some(rs) = rev_search {
        let m = rs.matches(input_history);
        let pick = if m.is_empty() {
            0usize
        } else {
            rs.pick % m.len()
        };
        let preview_full = m
            .get(pick)
            .cloned()
            .unwrap_or_else(|| tr("tui-revsearch-no-match"));
        let rq = &rs.query;
        let before: String = rq.chars[..rq.cursor].iter().collect();
        let after: String = rq.chars[rq.cursor..].iter().collect();
        let count_hint = if m.len() > 1 {
            format!(" ({}/{})", pick + 1, m.len())
        } else {
            String::new()
        };
        input_lines.push(Line::from(vec![
            Span::styled(tr("tui-revsearch-prefix"), style_warn()),
            Span::styled(before, Style::default().fg(Color::White)),
            Span::styled("▌", Style::default().fg(Color::Cyan)),
            Span::styled(after, Style::default().fg(Color::White)),
        ]));
        let preview_w = input_inner_w.saturating_sub(2) as usize;
        for (pi, row) in wrap_string_to_width(&preview_full, preview_w.max(8))
            .into_iter()
            .enumerate()
        {
            let pfx = if pi == 0 { "→ " } else { "  " };
            input_lines.push(Line::from(vec![
                Span::styled(pfx, style_dim()),
                Span::styled(row, style_dim()),
            ]));
        }
        if !count_hint.is_empty() {
            input_lines.push(Line::from(Span::styled(
                format!("  {count_hint}"),
                style_warn(),
            )));
        }
        if m.len() > 1 {
            input_lines.push(Line::from(Span::styled(
                tr("tui-revsearch-nav"),
                style_dim(),
            )));
        }
    } else {
        let lines_before_prompt = input_lines.len();
        let slash_ghost = if slash_suggest_suppress {
            None
        } else {
            slash_commands::slash_ghost_suffix(&input.as_string(), input.cursor)
        };
        let (pl, cur) = prompt_multiline_lines_and_cursor(input, input_inner_w, slash_ghost);
        for line in pl {
            input_lines.push(line);
        }
        if let Some((li, ox)) = cur {
            prompt_hw_cursor = Some((lines_before_prompt + li, ox));
        }
        if !slash_candidates.is_empty() {
            let len = slash_candidates.len();
            let pick = slash_suggest_pick % len;
            const MAX_SHOW: usize = 8;
            // 候选较多时：选中项尽量落在可见窗口中部。
            let start = if len <= MAX_SHOW {
                0usize
            } else {
                pick.saturating_sub(MAX_SHOW / 2)
                    .min(len.saturating_sub(MAX_SHOW))
            };
            let end = (start + MAX_SHOW).min(len);
            let line_w = input_inner_w as usize;
            let max_cmd_w =
                slash_commands::slash_menu_cmd_column_width(&slash_candidates, start, end, line_w);
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
                let pfx_s = if is_sel { "› " } else { "  " };
                let cmd_st2 = if is_sel {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    style_dim()
                };
                input_lines.push(Line::from(vec![
                    Span::styled(pfx_s, style_dim()),
                    Span::styled(cmd_cell, cmd_st2),
                    Span::styled(format!("  {desc}"), style_dim()),
                ]));
            }
            if len > MAX_SHOW {
                let mut a = FluentArgs::new();
                a.set("s", (start + 1) as i64);
                a.set("e", end as i64);
                a.set("n", len as i64);
                input_lines.push(Line::from(Span::styled(
                    tr_args("tui-slash-range", &a),
                    style_dim(),
                )));
            }
            input_lines.push(Line::from(Span::styled(tr("tui-slash-nav"), style_dim())));
        }
    }

    let input_par = Paragraph::new(Text::from(input_lines)).wrap(Wrap { trim: false });
    f.render_widget(input_par, prompt_cell);
    let rule_dn_idx = if compact_bottom_chrome { 2 } else { 5 };
    f.render_widget(
        Paragraph::new(horizontal_rule_line(bottom_split[rule_dn_idx].width)),
        bottom_split[rule_dn_idx],
    );

    let finp = FooterLayoutInput {
        permission_mode,
        require_approval,
        llm_provider,
        llm_model,
        transcript_scroll_up,
        debug,
        last_key,
        context_window_tokens,
        last_max_input_tokens,
        last_output_tokens,
    };
    let footer_lines = build_tui_footer_lines(footer_rect.width, &finp);
    f.render_widget(
        Paragraph::new(Text::from(footer_lines)).wrap(Wrap { trim: false }),
        footer_rect,
    );

    if let Some(bc) = buddy_cell {
        let pet_lines = pet::pet_panel_lines(pet_anim_frame, executing);
        let pet_par = Paragraph::new(Text::from(pet_lines))
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(Wrap { trim: false });
        f.render_widget(pet_par, bc);
    }

    if let Some((gli, ox)) = prompt_hw_cursor {
        if prompt_cell.height > 0 {
            let ya = prompt_cell.y.saturating_add(gli as u16);
            let y_end = prompt_cell.y + prompt_cell.height;
            if ya < y_end {
                let max_x = prompt_cell
                    .x
                    .saturating_add(prompt_cell.width.saturating_sub(1));
                let xa = prompt_cell.x.saturating_add(ox).min(max_x);
                f.set_cursor(xa, ya);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::approval::PendingApproval;
    use crate::tui::input::InputState;
    use crate::tui::transcript::TranscriptEntry;
    use crate::tui::util::{bottom_align_viewport_lines, top_align_viewport_lines};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use std::cell::Cell;
    use std::collections::HashSet;
    use tokio::sync::oneshot;

    fn buffer_to_string(term: &Terminal<TestBackend>) -> String {
        let buf = term.backend().buffer();
        let mut out = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                out.push_str(&buf.get(x, y).symbol);
            }
            out.push('\n');
        }
        out
    }

    fn first_buffer_line_containing(term: &Terminal<TestBackend>, needle: &str) -> usize {
        buffer_to_string(term)
            .lines()
            .position(|line| line.contains(needle))
            .unwrap_or_else(|| panic!("buffer missing {needle:?}"))
    }

    fn test_footer_h(width: u16) -> u16 {
        let finp = FooterLayoutInput {
            permission_mode: "default",
            require_approval: true,
            llm_provider: "mock",
            llm_model: "mock",
            transcript_scroll_up: 0,
            debug: false,
            last_key: None,
            context_window_tokens: 128_000,
            last_max_input_tokens: 0,
            last_output_tokens: 0,
        };
        footer_wrapped_line_count(width, &finp)
    }

    fn test_bottom_h(status_rows: u16, dock_need: u16, width: u16, pending_approval: bool) -> u16 {
        let fh = test_footer_h(width);
        let hud_rows = if pending_approval { 2 } else { 0 };
        let mut h = 1 + status_rows + hud_rows + 1 + dock_need + 1 + fh;
        if status_rows == 0 && hud_rows == 0 {
            h = h.saturating_sub(1);
        }
        h
    }

    fn draw_short_plain_row(used_alternate_screen: bool, bottom_h: u16) -> usize {
        let backend = TestBackend::new(80, 24);
        let mut term = Terminal::new(backend).unwrap();
        let mut cache_lines: Vec<Line<'static>> = vec![];
        let mut cache_gen: u64 = 0;
        let mut cache_w: usize = 0;
        let mut cache_fold_rev: u64 = 0;
        let mut cache_exec: bool = false;
        let mut cache_secs: Option<u64> = None;
        let mut cache_pulse: u64 = 0;
        let expanded: HashSet<u64> = HashSet::new();
        let agent_type = AgentType::new("general-purpose");
        let main_avail_cell = Cell::new(0usize);
        let workspace_line_count = Cell::new(0usize);
        let input = InputState::default();
        let input_history: Vec<String> = vec![];
        let fh = test_footer_h(80);
        let t1 = vec![TranscriptEntry::Plain(vec![Line::from("ShortPlainUx")])];
        term.draw(|f| {
            draw_tui_frame(
                f,
                DrawFrameCtx {
                    bottom_h,
                    footer_line_count: fh,
                    show_buddy: false,
                    pet_anim_frame: 0,
                    agent_type: &agent_type,
                    permission_mode: "default",
                    require_approval: true,
                    llm_provider: "mock",
                    llm_model: "mock",
                    debug: false,
                    last_key: None,
                    pending_approval: None,
                    pending_user_question: None,
                    approval_menu_selected: 0,
                    user_question_menu_selected: 0,
                    executing: false,
                    working_elapsed_secs: None,
                    help_open: false,
                    transcript: &t1,
                    transcript_scroll_up: 0,
                    rev_search: None,
                    slash_suggest_pick: 0,
                    slash_suggest_suppress: true,
                    input: &input,
                    input_history: &input_history,
                    workspace_cache_lines: &mut cache_lines,
                    workspace_cache_gen: &mut cache_gen,
                    workspace_cache_w: &mut cache_w,
                    workspace_cache_fold_rev: &mut cache_fold_rev,
                    workspace_cache_executing: &mut cache_exec,
                    workspace_cache_working_secs: &mut cache_secs,
                    workspace_cache_pulse_frame: &mut cache_pulse,
                    transcript_gen: 1,
                    fold_layout_rev: 1,
                    expanded_tool_folds: &expanded,
                    main_avail_cell: &main_avail_cell,
                    workspace_line_count: &workspace_line_count,
                    status_line_show: false,
                    status_line_text: " ",
                    status_line_padding: 0,
                    quit_confirm_pending: false,
                    used_alternate_screen,
                    tui_resume_session_id: "",
                    hud_tip_slot: 0,
                    last_max_input_tokens: 0,
                    last_output_tokens: 0,
                    context_window_tokens: 128_000,
                },
            );
        })
        .unwrap();
        first_buffer_line_containing(&term, "ShortPlainUx")
    }

    #[test]
    fn short_transcript_main_buffer_stays_higher_than_alt_screen() {
        let bottom_main = test_bottom_h(0, 1, 80, false);
        let bottom_alt = test_bottom_h(0, 4, 80, false);
        let row_main = draw_short_plain_row(false, bottom_main);
        let row_alt = draw_short_plain_row(true, bottom_alt);
        assert!(
            row_main < row_alt,
            "bounded main-buffer workspace should place content higher (main={row_main} alt={row_alt})"
        );
    }

    #[test]
    fn renders_assistant_incremental_text_changes() {
        let backend = TestBackend::new(80, 24);
        let mut term = Terminal::new(backend).unwrap();

        let mut cache_lines: Vec<Line<'static>> = vec![];
        let mut cache_gen: u64 = 0;
        let mut cache_w: usize = 0;
        let mut cache_fold_rev: u64 = 0;
        let mut cache_exec: bool = false;
        let mut cache_secs: Option<u64> = None;
        let mut cache_pulse: u64 = 0;
        let expanded: HashSet<u64> = HashSet::new();

        let agent_type = AgentType::new("general-purpose");
        let main_avail_cell = Cell::new(0usize);
        let workspace_line_count = Cell::new(0usize);

        let mut input = InputState::default();
        input.set_from_str("");
        let input_history: Vec<String> = vec![];
        let fh = test_footer_h(80);
        let bottom_h = test_bottom_h(0, 4, 80, false);

        let t1 = vec![TranscriptEntry::AssistantMarkdown("Hello".to_string())];
        term.draw(|f| {
            draw_tui_frame(
                f,
                DrawFrameCtx {
                    bottom_h,
                    footer_line_count: fh,
                    show_buddy: false,
                    pet_anim_frame: 0,
                    agent_type: &agent_type,
                    permission_mode: "default",
                    require_approval: true,
                    llm_provider: "mock",
                    llm_model: "mock",
                    debug: false,
                    last_key: None,
                    pending_approval: None,
                    pending_user_question: None,
                    approval_menu_selected: 0,
                    user_question_menu_selected: 0,
                    executing: false,
                    working_elapsed_secs: None,
                    help_open: false,
                    transcript: &t1,
                    transcript_scroll_up: 0,
                    rev_search: None,
                    slash_suggest_pick: 0,
                    slash_suggest_suppress: true,
                    input: &input,
                    input_history: &input_history,
                    workspace_cache_lines: &mut cache_lines,
                    workspace_cache_gen: &mut cache_gen,
                    workspace_cache_w: &mut cache_w,
                    workspace_cache_fold_rev: &mut cache_fold_rev,
                    workspace_cache_executing: &mut cache_exec,
                    workspace_cache_working_secs: &mut cache_secs,
                    workspace_cache_pulse_frame: &mut cache_pulse,
                    transcript_gen: 1,
                    fold_layout_rev: 1,
                    expanded_tool_folds: &expanded,
                    main_avail_cell: &main_avail_cell,
                    workspace_line_count: &workspace_line_count,
                    status_line_show: false,
                    status_line_text: " ",
                    status_line_padding: 0,
                    quit_confirm_pending: false,
                    used_alternate_screen: true,
                    tui_resume_session_id: "",
                    hud_tip_slot: 0,
                    last_max_input_tokens: 0,
                    last_output_tokens: 0,
                    context_window_tokens: 128_000,
                },
            );
        })
        .unwrap();
        let s1 = buffer_to_string(&term);
        assert!(s1.contains("Hello"));

        let t2 = vec![TranscriptEntry::AssistantMarkdown(
            "Hello world".to_string(),
        )];
        term.draw(|f| {
            draw_tui_frame(
                f,
                DrawFrameCtx {
                    bottom_h,
                    footer_line_count: fh,
                    show_buddy: false,
                    pet_anim_frame: 0,
                    agent_type: &agent_type,
                    permission_mode: "default",
                    require_approval: true,
                    llm_provider: "mock",
                    llm_model: "mock",
                    debug: false,
                    last_key: None,
                    pending_approval: None,
                    pending_user_question: None,
                    approval_menu_selected: 0,
                    user_question_menu_selected: 0,
                    executing: false,
                    working_elapsed_secs: None,
                    help_open: false,
                    transcript: &t2,
                    transcript_scroll_up: 0,
                    rev_search: None,
                    slash_suggest_pick: 0,
                    slash_suggest_suppress: true,
                    input: &input,
                    input_history: &input_history,
                    workspace_cache_lines: &mut cache_lines,
                    workspace_cache_gen: &mut cache_gen,
                    workspace_cache_w: &mut cache_w,
                    workspace_cache_fold_rev: &mut cache_fold_rev,
                    workspace_cache_executing: &mut cache_exec,
                    workspace_cache_working_secs: &mut cache_secs,
                    workspace_cache_pulse_frame: &mut cache_pulse,
                    transcript_gen: 2,
                    fold_layout_rev: 1,
                    expanded_tool_folds: &expanded,
                    main_avail_cell: &main_avail_cell,
                    workspace_line_count: &workspace_line_count,
                    status_line_show: false,
                    status_line_text: " ",
                    status_line_padding: 0,
                    quit_confirm_pending: false,
                    used_alternate_screen: true,
                    tui_resume_session_id: "",
                    hud_tip_slot: 0,
                    last_max_input_tokens: 0,
                    last_output_tokens: 0,
                    context_window_tokens: 128_000,
                },
            );
        })
        .unwrap();
        let s2 = buffer_to_string(&term);
        assert!(s2.contains("Hello world"));
    }

    /// 启用 `status_line_show` 时多一行自定义 status，`bottom_h` 须 +1；见
    /// `status_line_forgot_plus_one_collapses_rule_and_status`。
    #[test]
    fn status_line_row_renders_marker_above_dock() {
        let backend = TestBackend::new(80, 24);
        let mut term = Terminal::new(backend).unwrap();

        let mut cache_lines: Vec<Line<'static>> = vec![];
        let mut cache_gen: u64 = 0;
        let mut cache_w: usize = 0;
        let mut cache_fold_rev: u64 = 0;
        let mut cache_exec: bool = false;
        let mut cache_secs: Option<u64> = None;
        let mut cache_pulse: u64 = 0;
        let expanded: HashSet<u64> = HashSet::new();

        let agent_type = AgentType::new("general-purpose");
        let main_avail_cell = Cell::new(0usize);
        let workspace_line_count = Cell::new(0usize);

        let mut input = InputState::default();
        input.set_from_str("");
        let input_history: Vec<String> = vec![];

        let marker = "§AC_STATUSLINE_UX§";
        let fh = test_footer_h(80);
        let bottom_h = test_bottom_h(1, 4, 80, false);
        let t1 = vec![TranscriptEntry::AssistantMarkdown("Hi".to_string())];
        term.draw(|f| {
            draw_tui_frame(
                f,
                DrawFrameCtx {
                    bottom_h,
                    footer_line_count: fh,
                    show_buddy: false,
                    pet_anim_frame: 0,
                    agent_type: &agent_type,
                    permission_mode: "default",
                    require_approval: true,
                    llm_provider: "mock",
                    llm_model: "mock",
                    debug: false,
                    last_key: None,
                    pending_approval: None,
                    pending_user_question: None,
                    approval_menu_selected: 0,
                    user_question_menu_selected: 0,
                    executing: false,
                    working_elapsed_secs: None,
                    help_open: false,
                    transcript: &t1,
                    transcript_scroll_up: 0,
                    rev_search: None,
                    slash_suggest_pick: 0,
                    slash_suggest_suppress: true,
                    input: &input,
                    input_history: &input_history,
                    workspace_cache_lines: &mut cache_lines,
                    workspace_cache_gen: &mut cache_gen,
                    workspace_cache_w: &mut cache_w,
                    workspace_cache_fold_rev: &mut cache_fold_rev,
                    workspace_cache_executing: &mut cache_exec,
                    workspace_cache_working_secs: &mut cache_secs,
                    workspace_cache_pulse_frame: &mut cache_pulse,
                    transcript_gen: 1,
                    fold_layout_rev: 1,
                    expanded_tool_folds: &expanded,
                    main_avail_cell: &main_avail_cell,
                    workspace_line_count: &workspace_line_count,
                    status_line_show: true,
                    status_line_text: marker,
                    status_line_padding: 2,
                    quit_confirm_pending: false,
                    used_alternate_screen: true,
                    tui_resume_session_id: "",
                    hud_tip_slot: 0,
                    last_max_input_tokens: 0,
                    last_output_tokens: 0,
                    context_window_tokens: 128_000,
                },
            );
        })
        .unwrap();
        let buf = buffer_to_string(&term);
        assert!(
            buf.contains(marker),
            "expected dim status line text in buffer (check bottom_h vs loop_inner status row)"
        );
    }

    /// 与历史底栏分段一致（Dock `Min(6)`），用于「少 1 行则脚标被压扁」回归；Dock 行数随 `bottom_h` 与 loop 预算变化。
    fn split_bottom_with_status_line(bottom_h: u16, hud_rows: u16, footer_h: u16) -> Vec<Rect> {
        let bottom = Rect::new(0, 0, 80, bottom_h);
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(hud_rows),
                Constraint::Length(1),
                Constraint::Min(6),
                Constraint::Length(1),
                Constraint::Length(footer_h),
            ])
            .split(bottom)
            .to_vec()
    }

    /// 启用自定义 status 时底栏至少 **14** 行（含 HUD 2 行 + 脚标 2）；少 1 行时 ratatui 往往先压脚标。
    #[test]
    fn status_line_forgot_plus_one_collapses_rule_and_status() {
        let thirteen = split_bottom_with_status_line(13, 2, 2);
        let fourteen = split_bottom_with_status_line(14, 2, 2);
        assert_eq!(
            thirteen.iter().map(|r| r.height).sum::<u16>(),
            13,
            "chunks fill bottom area"
        );
        assert_eq!(fourteen.iter().map(|r| r.height).sum::<u16>(), 14);
        assert_eq!(fourteen[4].height, 6);
        assert_eq!(fourteen[6].height, 2);
        assert!(
            thirteen[6].height < 2,
            "forgot +1: footer squeezed when bottom_h too small for status + HUD + dual rules"
        );
    }

    #[test]
    fn approval_hint_is_three_way_no_session_option() {
        let backend = TestBackend::new(80, 24);
        let mut term = Terminal::new(backend).unwrap();

        let mut cache_lines: Vec<Line<'static>> = vec![];
        let mut cache_gen: u64 = 0;
        let mut cache_w: usize = 0;
        let mut cache_fold_rev: u64 = 0;
        let mut cache_exec: bool = false;
        let mut cache_secs: Option<u64> = None;
        let mut cache_pulse: u64 = 0;
        let expanded: HashSet<u64> = HashSet::new();

        let agent_type = AgentType::new("general-purpose");
        let main_avail_cell = Cell::new(0usize);
        let workspace_line_count = Cell::new(0usize);

        let input = InputState::default();
        let input_history: Vec<String> = vec![];
        let transcript: Vec<TranscriptEntry> =
            vec![TranscriptEntry::Plain(vec![Line::from("workspace")])];

        let (tx, _rx) = oneshot::channel();
        let pending = PendingApproval {
            tool: "Bash".to_string(),
            input_preview: r#"{"command":"echo hi"}"#.to_string(),
            reply: tx,
        };

        let fh = test_footer_h(80);
        let bottom_h = test_bottom_h(0, 17, 80, true);

        term.draw(|f| {
            draw_tui_frame(
                f,
                DrawFrameCtx {
                    bottom_h,
                    footer_line_count: fh,
                    show_buddy: false,
                    pet_anim_frame: 0,
                    agent_type: &agent_type,
                    permission_mode: "default",
                    require_approval: true,
                    llm_provider: "mock",
                    llm_model: "mock",
                    debug: false,
                    last_key: None,
                    pending_approval: Some(&pending),
                    pending_user_question: None,
                    approval_menu_selected: 1,
                    user_question_menu_selected: 0,
                    executing: false,
                    working_elapsed_secs: None,
                    help_open: false,
                    transcript: &transcript,
                    transcript_scroll_up: 0,
                    rev_search: None,
                    slash_suggest_pick: 0,
                    slash_suggest_suppress: true,
                    input: &input,
                    input_history: &input_history,
                    workspace_cache_lines: &mut cache_lines,
                    workspace_cache_gen: &mut cache_gen,
                    workspace_cache_w: &mut cache_w,
                    workspace_cache_fold_rev: &mut cache_fold_rev,
                    workspace_cache_executing: &mut cache_exec,
                    workspace_cache_working_secs: &mut cache_secs,
                    workspace_cache_pulse_frame: &mut cache_pulse,
                    transcript_gen: 1,
                    fold_layout_rev: 1,
                    expanded_tool_folds: &expanded,
                    main_avail_cell: &main_avail_cell,
                    workspace_line_count: &workspace_line_count,
                    status_line_show: false,
                    status_line_text: " ",
                    status_line_padding: 0,
                    quit_confirm_pending: false,
                    used_alternate_screen: true,
                    tui_resume_session_id: "",
                    hud_tip_slot: 0,
                    last_max_input_tokens: 0,
                    last_output_tokens: 0,
                    context_window_tokens: 128_000,
                },
            );
        })
        .unwrap();
        let s = buffer_to_string(&term);
        assert!(s.contains("Bash"));
        assert!(s.contains("echo hi"));
        assert!(!s.contains("会话"));
        assert!(!s.contains("session"));
    }

    #[test]
    fn short_transcript_viewport_is_bottom_aligned_with_top_padding() {
        let lines = vec![Line::from("only")];
        let out = bottom_align_viewport_lines(lines, 5, 0);
        assert_eq!(out.len(), 5);
        assert_eq!(
            out[4].spans.first().map(|s| s.content.as_ref()),
            Some("only")
        );
    }

    #[test]
    fn empty_home_main_buffer_is_top_aligned_with_bottom_padding() {
        let lines = vec![Line::from("brand")];
        let out = top_align_viewport_lines(lines, 5);
        assert_eq!(out.len(), 5);
        assert_eq!(
            out[0].spans.first().map(|s| s.content.as_ref()),
            Some("brand")
        );
    }

    /// 集成断言：主缓冲空会话顶对齐，`anyCode` 行号应早于备用屏底对齐（无需人工盯终端）。
    #[test]
    fn empty_home_main_buffer_renders_brand_higher_than_alternate_screen() {
        fn draw_empty_home_row(used_alternate_screen: bool) -> usize {
            let backend = TestBackend::new(80, 24);
            let mut term = Terminal::new(backend).unwrap();
            let mut cache_lines: Vec<Line<'static>> = vec![];
            let mut cache_gen: u64 = 0;
            let mut cache_w: usize = 0;
            let mut cache_fold_rev: u64 = 0;
            let mut cache_exec: bool = false;
            let mut cache_secs: Option<u64> = None;
            let mut cache_pulse: u64 = 0;
            let expanded: HashSet<u64> = HashSet::new();
            let agent_type = AgentType::new("general-purpose");
            let main_avail_cell = Cell::new(0usize);
            let workspace_line_count = Cell::new(0usize);
            let mut input = InputState::default();
            input.set_from_str("");
            let input_history: Vec<String> = vec![];
            let transcript: Vec<TranscriptEntry> = vec![];
            let fh = test_footer_h(80);
            let bottom_h = test_bottom_h(0, 4, 80, false);

            term.draw(|f| {
                draw_tui_frame(
                    f,
                    DrawFrameCtx {
                        bottom_h,
                        footer_line_count: fh,
                        show_buddy: false,
                        pet_anim_frame: 0,
                        agent_type: &agent_type,
                        permission_mode: "default",
                        require_approval: true,
                        llm_provider: "mock",
                        llm_model: "mock",
                        debug: false,
                        last_key: None,
                        pending_approval: None,
                        pending_user_question: None,
                        approval_menu_selected: 0,
                        user_question_menu_selected: 0,
                        executing: false,
                        working_elapsed_secs: None,
                        help_open: false,
                        transcript: &transcript,
                        transcript_scroll_up: 0,
                        rev_search: None,
                        slash_suggest_pick: 0,
                        slash_suggest_suppress: true,
                        input: &input,
                        input_history: &input_history,
                        workspace_cache_lines: &mut cache_lines,
                        workspace_cache_gen: &mut cache_gen,
                        workspace_cache_w: &mut cache_w,
                        workspace_cache_fold_rev: &mut cache_fold_rev,
                        workspace_cache_executing: &mut cache_exec,
                        workspace_cache_working_secs: &mut cache_secs,
                        workspace_cache_pulse_frame: &mut cache_pulse,
                        transcript_gen: 0,
                        fold_layout_rev: 0,
                        expanded_tool_folds: &expanded,
                        main_avail_cell: &main_avail_cell,
                        workspace_line_count: &workspace_line_count,
                        status_line_show: false,
                        status_line_text: " ",
                        status_line_padding: 0,
                        quit_confirm_pending: false,
                        used_alternate_screen,
                        tui_resume_session_id: "",
                        hud_tip_slot: 0,
                        last_max_input_tokens: 0,
                        last_output_tokens: 0,
                        context_window_tokens: 128_000,
                    },
                );
            })
            .unwrap();
            first_buffer_line_containing(&term, "anyCode")
        }

        let row_main = draw_empty_home_row(false);
        let row_alt = draw_empty_home_row(true);
        assert!(
            row_main <= 1,
            "main-buffer welcome card keeps title near top (main={row_main})"
        );
        assert!(
            row_main < row_alt,
            "expected main-buffer brand above alt-screen brand (main={row_main} alt={row_alt})"
        );
    }
}
