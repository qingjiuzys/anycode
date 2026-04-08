//! 单帧 TUI 绘制（ratatui），与事件循环解耦以便阅读与单测。

use crate::i18n::{tr, tr_args};
use crate::md_tui::{text_display_width, wrap_ratatui_line, wrap_string_to_width};
use crate::slash_commands;
use crate::tui::approval::PendingApproval;
use crate::tui::chrome::{sidebar_help_text, welcome_lines};
use crate::tui::input::{
    format_cwd_header, prompt_multiline_lines_and_cursor, InputState, RevSearchState,
};
use crate::tui::pet;
use crate::tui::styles::*;
use crate::tui::transcript::{layout_workspace, TranscriptEntry, WorkspaceLiveLayout};
use crate::tui::util::{transcript_first_visible, truncate_preview};
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

/// Dock 内 Buddy 列固定宽度。用手写 `Rect` 切分，避免 `Layout::Min + Length` 重绘时列分配抖动（输入变长时 Buddy 像「后退」）。
const DOCK_BUDDY_AREA_WIDTH: u16 = 14;

/// 与 Workspace / Dock 左右边对齐的整行横线（Claude Code 式分隔）。
fn horizontal_rule_line(width: u16) -> Line<'static> {
    let w = width.max(1) as usize;
    Line::from(Span::styled("─".repeat(w), style_dim()))
}

/// Workspace 右侧滚动条（与 `transcript_scroll_up` / `transcript_first_visible` 一致）。
fn workspace_scrollbar_text(
    total_lines: usize,
    visible: usize,
    scroll_up: usize,
    track_h: usize,
) -> Text<'static> {
    let track_h = track_h.max(1);
    let dim = Style::default().fg(Color::DarkGray);
    let thumb = Style::default().fg(Color::Gray);
    let mut lines: Vec<Line<'static>> = Vec::with_capacity(track_h);
    if total_lines == 0 || visible == 0 {
        for _ in 0..track_h {
            lines.push(Line::from(Span::styled("│", dim)));
        }
        return Text::from(lines);
    }
    if total_lines <= visible {
        for _ in 0..track_h {
            lines.push(Line::from(Span::styled("│", dim)));
        }
        return Text::from(lines);
    }
    let avail = visible.max(1);
    let max_start = total_lines.saturating_sub(avail);
    let start = transcript_first_visible(total_lines, avail, scroll_up);
    let thumb_h = (avail * avail / total_lines).max(1).min(avail);
    let thumb_start = if max_start == 0 {
        0usize
    } else {
        start
            .saturating_mul(avail.saturating_sub(thumb_h))
            .saturating_div(max_start)
    };
    for row in 0..track_h {
        let ch = if row >= thumb_start && row < thumb_start.saturating_add(thumb_h) {
            '█'
        } else {
            '▒'
        };
        lines.push(Line::from(Span::styled(ch.to_string(), thumb)));
    }
    Text::from(lines)
}

pub(super) struct DrawFrameCtx<'a> {
    pub size: Rect,
    pub bottom_h: u16,
    /// 宽度足够时在底栏 Dock 内、Prompt 右侧显示 Buddy。
    pub show_buddy: bool,
    /// 宠物动画帧 `0..4`（`executing` 时由外层用时间驱动）。
    pub pet_anim_frame: u64,
    pub working_dir_str: &'a str,
    pub agent_type: &'a AgentType,
    pub permission_mode: &'a str,
    pub require_approval: bool,
    pub llm_provider: &'a str,
    pub llm_plan: &'a str,
    pub llm_model: &'a str,
    pub debug: bool,
    pub last_key: Option<&'a str>,
    pub pending_approval: Option<&'a PendingApproval>,
    /// `0..3`：与 `event` 中审批菜单一致。
    pub approval_menu_selected: usize,
    pub executing: bool,
    /// `executing` 为 true 时，自 `executing_since` 起经过的整秒数（用于顶栏，避免子秒刷新）。
    pub working_elapsed_secs: Option<u64>,
    pub help_open: bool,
    pub transcript: &'a [TranscriptEntry],
    pub transcript_scroll_up: usize,
    pub rev_search: Option<&'a RevSearchState>,
    /// 首行 `/` 补全时，候选列表中高亮项（与 `slash_suggestions_for_first_line` 配合）。
    pub slash_suggest_pick: usize,
    /// 采纳补全后隐藏列表（对齐 Claude `clearSuggestions`）。
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
}

pub(super) fn draw_tui_frame(f: &mut Frame<'_>, ctx: DrawFrameCtx<'_>) {
    let DrawFrameCtx {
        size,
        bottom_h,
        show_buddy,
        pet_anim_frame,
        working_dir_str,
        agent_type,
        permission_mode,
        require_approval,
        llm_provider,
        llm_plan,
        llm_model,
        debug,
        last_key,
        pending_approval,
        approval_menu_selected,
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
    } = ctx;

    let slash_candidates =
        if pending_approval.is_none() && rev_search.is_none() && !slash_suggest_suppress {
            slash_commands::slash_suggestions_for_first_line(&input.as_string())
        } else {
            Vec::new()
        };

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(1),
            Constraint::Length(bottom_h),
        ])
        .split(size);

    // --- 顶栏：品牌 + 路径；下行「状态｜模型｜Agent」；再下行配置两行（弱分隔、少颜色） ---
    let status_text: String = if pending_approval.is_some() {
        tr("tui-status-await-approval")
    } else if executing {
        match working_elapsed_secs {
            Some(s) => {
                let mut a = FluentArgs::new();
                a.set("s", s);
                tr_args("tui-status-working-secs", &a)
            }
            None => tr("tui-status-working"),
        }
    } else {
        tr("tui-status-idle")
    };
    let header_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(style_dim());
    let header_area = header_block.inner(outer[0]);
    f.render_widget(header_block, outer[0]);

    let header_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(2),
        ])
        .split(header_area);

    let pipe = Style::default().fg(Color::DarkGray);
    let val = Style::default().fg(Color::White);

    let cwd_budget = (header_split[0].width as usize).saturating_sub(10);
    let cwd_disp = format_cwd_header(working_dir_str, cwd_budget.max(8));

    let line_brand = Line::from(vec![
        Span::styled("anyCode", style_brand()),
        Span::styled("  ", Style::default()),
        Span::styled(cwd_disp, style_dim()),
    ]);
    f.render_widget(
        Paragraph::new(line_brand).wrap(Wrap { trim: true }),
        header_split[0],
    );

    let line_runtime = Line::from(vec![
        Span::styled(tr("tui-hdr-status"), style_dim()),
        Span::styled(status_text, style_assistant()),
        Span::styled(" │ ", pipe),
        Span::styled(tr("tui-hdr-model"), style_dim()),
        Span::styled(truncate_preview(llm_model, 36), val),
        Span::styled(" │ ", pipe),
        Span::styled(tr("tui-hdr-agent"), style_dim()),
        Span::styled(agent_type.as_str(), val),
    ]);
    f.render_widget(
        Paragraph::new(line_runtime).wrap(Wrap { trim: true }),
        header_split[1],
    );

    let line_cfg_a = vec![
        Span::styled(tr("tui-hdr-provider"), style_dim()),
        Span::styled(truncate_preview(llm_provider, 22), val),
        Span::styled(" │ ", pipe),
        Span::styled(tr("tui-hdr-plan"), style_dim()),
        Span::styled(truncate_preview(llm_plan, 18), val),
        Span::styled(" │ ", pipe),
        Span::styled(tr("tui-hdr-permission"), style_dim()),
        Span::styled(permission_mode, val),
    ];
    let mut line_cfg_b = vec![
        Span::styled(tr("tui-hdr-approval"), style_dim()),
        Span::styled(
            if require_approval {
                tr("tui-approval-on-short")
            } else {
                tr("tui-approval-off-short")
            },
            if require_approval {
                style_assistant()
            } else {
                style_dim()
            },
        ),
    ];
    if debug {
        if let Some(k) = last_key {
            line_cfg_b.push(Span::styled(" │ ", pipe));
            line_cfg_b.push(Span::styled(
                format!("{}{}", tr("tui-hdr-key-prefix"), k),
                style_dim(),
            ));
        }
    }
    f.render_widget(
        Paragraph::new(Text::from(vec![
            Line::from(line_cfg_a),
            Line::from(line_cfg_b),
        ]))
        .wrap(Wrap { trim: true }),
        header_split[2],
    );

    // --- 中部：主对话（全宽终端滚动；Buddy 在底栏 Dock 内） ---
    let mid = outer[1];

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
    } else {
        let main_rect = mid;

        let ws_title: Span = if transcript_scroll_up > 0 {
            let mut a = FluentArgs::new();
            a.set("n", transcript_scroll_up as i64);
            Span::styled(
                tr_args("tui-workspace-scrolled", &a),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                tr("tui-workspace-title"),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
        };

        let workspace_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(ws_title);
        let inner = workspace_block.inner(main_rect);
        let available_height = inner.height as usize;
        main_avail_cell.set(available_height.max(1));

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner);

        let content_width = chunks[0].width as usize;

        if *workspace_cache_gen != transcript_gen
            || *workspace_cache_w != content_width
            || *workspace_cache_fold_rev != fold_layout_rev
            || *workspace_cache_executing != executing
            || *workspace_cache_working_secs != working_elapsed_secs
            || *workspace_cache_pulse_frame != pet_anim_frame
        {
            *workspace_cache_lines = layout_workspace(
                transcript,
                content_width,
                expanded_tool_folds,
                WorkspaceLiveLayout {
                    executing,
                    working_elapsed_secs,
                    pulse_frame: pet_anim_frame,
                },
            );
            *workspace_cache_gen = transcript_gen;
            *workspace_cache_w = content_width;
            *workspace_cache_fold_rev = fold_layout_rev;
            *workspace_cache_executing = executing;
            *workspace_cache_working_secs = working_elapsed_secs;
            *workspace_cache_pulse_frame = pet_anim_frame;
        }

        let welcome_wrapped: Vec<Line<'static>> = if transcript.is_empty() {
            welcome_lines(permission_mode, require_approval)
                .into_iter()
                .flat_map(|l| wrap_ratatui_line(l, content_width.max(8)))
                .collect()
        } else {
            Vec::new()
        };

        if transcript.is_empty() {
            workspace_line_count.set(welcome_wrapped.len().max(1));
        } else {
            workspace_line_count.set(workspace_cache_lines.len());
        }

        let avail = available_height.max(1);
        let body: Text = if transcript.is_empty() {
            let start =
                transcript_first_visible(welcome_wrapped.len(), avail, transcript_scroll_up);
            let end = (start + avail).min(welcome_wrapped.len());
            Text::from(welcome_wrapped[start..end].to_vec())
        } else {
            let start =
                transcript_first_visible(workspace_cache_lines.len(), avail, transcript_scroll_up);
            let end = (start + avail).min(workspace_cache_lines.len());
            Text::from(workspace_cache_lines[start..end].to_vec())
        };

        let total_for_bar = if transcript.is_empty() {
            welcome_wrapped.len()
        } else {
            workspace_cache_lines.len()
        };
        let scroll_body = workspace_scrollbar_text(
            total_for_bar,
            available_height.max(1),
            transcript_scroll_up,
            available_height.max(1),
        );

        f.render_widget(workspace_block, main_rect);
        f.render_widget(Paragraph::new(body).wrap(Wrap { trim: false }), chunks[0]);
        f.render_widget(Paragraph::new(scroll_body), chunks[1]);
    }

    // --- 底栏：横线 + Input Dock（快捷键见 `?` 帮助） ---
    let bottom = outer[2];
    let bottom_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(6)])
        .split(bottom);

    f.render_widget(
        Paragraph::new(horizontal_rule_line(bottom_split[0].width)),
        bottom_split[0],
    );

    let dock_rect = bottom_split[1];
    let dock_title = if pending_approval.is_some() {
        tr("tui-dock-approve")
    } else if rev_search.is_some() {
        tr("tui-dock-search")
    } else if !slash_candidates.is_empty() {
        tr("tui-dock-slash")
    } else {
        tr("tui-dock-prompt")
    };
    let dock_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(dock_title, style_brand()));

    let dock_inner = dock_block.inner(dock_rect);
    f.render_widget(dock_block, dock_rect);

    let body_rect = dock_inner;

    let show_buddy_here = show_buddy
        && pending_approval.is_none()
        && rev_search.is_none()
        && slash_candidates.is_empty();
    let (prompt_cell, buddy_cell) =
        if show_buddy_here && body_rect.width >= DOCK_BUDDY_AREA_WIDTH.saturating_add(14) {
            let prompt_w = body_rect.width.saturating_sub(DOCK_BUDDY_AREA_WIDTH);
            let buddy_x = body_rect.x.saturating_add(prompt_w);
            (
                Rect {
                    x: body_rect.x,
                    y: body_rect.y,
                    width: prompt_w,
                    height: body_rect.height,
                },
                Some(Rect {
                    x: buddy_x,
                    y: body_rect.y,
                    width: DOCK_BUDDY_AREA_WIDTH,
                    height: body_rect.height,
                }),
            )
        } else {
            (body_rect, None)
        };

    let input_inner_w = prompt_cell.width.max(1);

    let mut input_lines: Vec<Line> = vec![];
    // 主输入区：`(Paragraph 内行下标, 行内显示宽度列)`，供 `Frame::set_cursor` 把硬件光标放到 Prompt 内（IME 预编辑跟随）。
    let mut prompt_hw_cursor: Option<(usize, u16)> = None;

    if let Some(p) = pending_approval {
        input_lines.push(horizontal_rule_line(body_rect.width.max(1)));
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
            // 与 Claude `PromptInputFooterSuggestions` 类似：选中项尽量落在窗口中部。
            let start = if len <= MAX_SHOW {
                0usize
            } else {
                pick.saturating_sub(MAX_SHOW / 2)
                    .min(len.saturating_sub(MAX_SHOW))
            };
            let end = (start + MAX_SHOW).min(len);
            for idx in start..end {
                let item = &slash_candidates[idx];
                let is_sel = idx == pick;
                let pfx = if is_sel { "▸ " } else { "  " };
                let cmd_st = if is_sel { style_warn() } else { style_dim() };
                let cmd_w = text_display_width(item.display.as_str()).max(6).min(14);
                let desc_max = (input_inner_w as usize)
                    .saturating_sub(4 + cmd_w + 2)
                    .max(6);
                let desc = truncate_preview(&item.description, desc_max);
                input_lines.push(Line::from(vec![
                    Span::styled(pfx, style_dim()),
                    Span::styled(item.display.as_str(), cmd_st),
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

    if let Some(bc) = buddy_cell {
        let pet_lines = pet::pet_panel_lines(pet_anim_frame, executing);
        let pet_par = Paragraph::new(Text::from(pet_lines))
            .alignment(ratatui::layout::Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::LEFT)
                    .border_style(style_dim())
                    .title(Span::styled(tr("tui-buddy-title"), style_brand())),
            )
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

    #[test]
    fn renders_assistant_incremental_text_changes() {
        let backend = TestBackend::new(80, 24);
        let mut term = Terminal::new(backend).unwrap();
        let size = Rect::new(0, 0, 80, 24);

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

        let t1 = vec![TranscriptEntry::AssistantMarkdown("Hello".to_string())];
        term.draw(|f| {
            draw_tui_frame(
                f,
                DrawFrameCtx {
                    size,
                    bottom_h: 7,
                    show_buddy: false,
                    pet_anim_frame: 0,
                    working_dir_str: ".",
                    agent_type: &agent_type,
                    permission_mode: "default",
                    require_approval: true,
                    llm_provider: "mock",
                    llm_plan: "coding",
                    llm_model: "mock",
                    debug: false,
                    last_key: None,
                    pending_approval: None,
                    approval_menu_selected: 0,
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
                    size,
                    bottom_h: 7,
                    show_buddy: false,
                    pet_anim_frame: 0,
                    working_dir_str: ".",
                    agent_type: &agent_type,
                    permission_mode: "default",
                    require_approval: true,
                    llm_provider: "mock",
                    llm_plan: "coding",
                    llm_model: "mock",
                    debug: false,
                    last_key: None,
                    pending_approval: None,
                    approval_menu_selected: 0,
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
                },
            );
        })
        .unwrap();
        let s2 = buffer_to_string(&term);
        assert!(s2.contains("Hello world"));
    }

    #[test]
    fn approval_hint_is_three_way_no_session_option() {
        let backend = TestBackend::new(80, 24);
        let mut term = Terminal::new(backend).unwrap();
        let size = Rect::new(0, 0, 80, 24);

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

        term.draw(|f| {
            draw_tui_frame(
                f,
                DrawFrameCtx {
                    size,
                    bottom_h: 7,
                    show_buddy: false,
                    pet_anim_frame: 0,
                    working_dir_str: ".",
                    agent_type: &agent_type,
                    permission_mode: "default",
                    require_approval: true,
                    llm_provider: "mock",
                    llm_plan: "coding",
                    llm_model: "mock",
                    debug: false,
                    last_key: None,
                    pending_approval: Some(&pending),
                    approval_menu_selected: 1,
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
}
