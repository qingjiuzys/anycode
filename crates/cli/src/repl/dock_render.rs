//! 流式 REPL 底栏布局与绘制（从 `inline` 拆出，与全屏 TUI Prompt dock 规则对齐）。

use std::time::Instant;

use crate::i18n::{tr, tr_args};
use crate::md_tui::{
    pad_end_to_display_width, text_display_width, truncate_to_display_width, wrap_string_to_width,
};
use crate::repl::line_state::ReplLineState;
use crate::repl::slash_ctx::slash_suggestions_for_ctx;
use crate::slash_commands;
use crate::tui::input::prompt_multiline_lines_and_cursor;
use crate::tui::styles::{
    style_dim, style_horizontal_rule, style_menu_selected, style_tool, style_warn,
};
use crate::tui::util::truncate_preview;
use fluent_bundle::FluentArgs;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Paragraph, Widget, Wrap};

/// 底栏布局参数（流式 Inline：**prompt 上下各一条固定满宽 `─`**，与 HUD / 执行态无关，避免输入区随状态「少一条线」跳动）。
/// 自上而下：**HUD → 上横线 → 输入 → 斜杠/审批 → 下横线 → 脚标**。活跃回合的 ✶/⎿ 在 HUD，脚标为 ctx / provider 等。
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

    /// 输入框**正下方**整行横线（斜杠候选 / 脚标在此行之下；与上横线成对固定）。
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

fn repl_stream_user_question_block_h(
    width: u16,
    state: &ReplLineState,
    layout: ReplDockLayout,
) -> u16 {
    let Some(q) = state.pending_user_question.as_ref() else {
        return 0;
    };
    let w = width.max(8) as usize;
    let mut rows = 1u16;
    if !q.header.trim().is_empty() {
        rows = rows.saturating_add(1);
    }
    let qq = q.question.trim();
    if !qq.is_empty() {
        let qrows = if text_display_width(qq) <= w {
            1u16
        } else {
            wrap_string_to_width(qq, w.max(8))
                .len()
                .min(layout.approval_preview_wrap_cap()) as u16
        };
        rows = rows.saturating_add(qrows);
    }
    rows = rows.saturating_add(q.option_labels.len() as u16);
    rows = rows.saturating_add(1);
    rows.min(layout.approval_total_cap())
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ReplDockNatural {
    /// 与全屏 TUI Prompt HUD 对齐的 `✶`/`⎿` 两行（或收缩为 0～1 行）。
    pub(crate) hud_h: u16,
    pub(crate) rule_top_h: u16,
    pub(crate) input_h: u16,
    pub(crate) rule_bottom_h: u16,
    pub(crate) approval_h: u16,
    pub(crate) sugg_h: u16,
    pub(crate) status_h: u16,
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

pub(crate) fn repl_dock_compute_natural(
    area_width: u16,
    state: &ReplLineState,
    layout: ReplDockLayout,
) -> ReplDockNatural {
    // 与全屏 TUI 对齐：审批/选题 ✶+⎿ 两行；纯执行仅 ✶ 一行（贴近输入，不轮换 ⎿）。
    let hud_h = if state.pending_approval.is_some() || state.pending_user_question.is_some() {
        2u16
    } else if state.executing_since.is_some() {
        1u16
    } else {
        0u16
    };
    let status_h = if state.dock_status.is_empty() {
        0u16
    } else {
        1u16
    };
    let approval_h = repl_stream_approval_block_h(area_width, state, layout)
        .max(repl_stream_user_question_block_h(area_width, state, layout));
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
/// **流式 Inline**：返回值 = dock 内层总高度（含 prompt **上下**各一行固定 `─`；无外层 `Block`）。
pub(crate) fn repl_dock_height(area: Rect, state: &ReplLineState, layout: ReplDockLayout) -> u16 {
    let avail = area.height.saturating_sub(1);
    let nat = repl_dock_compute_natural(area.width.max(1), state, layout);
    let max_inner = avail.max(1);
    let target_inner = nat.sum().max(layout.min_dock_rows()).min(max_inner).max(1);
    let fitted = repl_dock_fit_into(target_inner, nat);
    let inner_h = fitted.sum().min(max_inner).max(1);
    inner_h.min(avail).max(layout.min_dock_rows())
}

/// 与全屏 TUI Prompt HUD 对齐：`hud_h==1` 仅 ✶；`hud_h>=2` 加 ⎿（[`repl_dock_compute_natural`] 按执行/审批设 1 或 2）。
fn render_stream_hud_to_buffer(buf: &mut Buffer, area: Rect, state: &ReplLineState, hud_h: u16) {
    if hud_h == 0 || area.height == 0 {
        return;
    }
    let pending = state.pending_approval.is_some() || state.pending_user_question.is_some();
    let exec = state.executing_since.is_some();
    let secs = state.executing_since.map(|t| t.elapsed().as_secs());
    let activity = crate::tui::hud_text::prompt_hud_activity_text(pending, exec, secs);
    let activity_line_style = Style::default().fg(crate::tui::palette::thinking_caption());
    let mut lines: Vec<Line> = vec![Line::from(vec![
        Span::styled(
            "✶ ",
            Style::default()
                .fg(crate::tui::palette::secondary())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(activity, activity_line_style),
    ])];
    if hud_h >= 2 {
        let slot = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as usize
            / 8)
            % crate::tui::hud_text::HUD_TIP_COUNT;
        let tip = crate::tui::hud_text::hud_tip_rotated(slot);
        lines.push(Line::from(vec![
            Span::styled("⎿ ", style_dim()),
            Span::styled(tip, style_dim()),
        ]));
    }
    let max_lines = area.height as usize;
    if lines.len() > max_lines {
        lines.truncate(max_lines.max(1));
    }
    Paragraph::new(Text::from(lines)).render(area, buf);
}

/// 底栏脚标：左 `dock_footer_left`（ctx / ? / scroll）+ 间隙 + 右 `dock_status`（与全屏 TUI 脚标同一视觉层次）。
fn stream_dock_status_line_spans(state: &ReplLineState, width: usize) -> Line<'static> {
    let w = width.max(8);
    let left = state.dock_footer_left.as_str();
    let stream_busy = state.executing_since.is_some()
        || state.pending_approval.is_some()
        || state.pending_user_question.is_some();
    // 执行/审批/选题时 HUD 已占 ✶ 行，不再在脚标右侧拼 provider·model·agent·审批，避免与 thinking 连成一长条。
    let right = if stream_busy {
        ""
    } else {
        state.dock_status.as_str()
    };
    if right.is_empty() {
        return Line::from(Span::styled(truncate_preview(left, w), style_dim()));
    }
    if left.is_empty() {
        return Line::from(Span::styled(
            truncate_preview(right, w),
            Style::default().fg(Color::White),
        ));
    }
    let lw = text_display_width(left);
    let rw = text_display_width(right);
    let gap = w.saturating_sub(lw + rw).max(1).min(200);
    if lw.saturating_add(gap).saturating_add(rw) > w {
        let merged = format!("{left} · {right}");
        return Line::from(Span::styled(truncate_preview(&merged, w), style_dim()));
    }
    Line::from(vec![
        Span::styled(left.to_string(), style_dim()),
        Span::styled(" ".repeat(gap), Style::default()),
        Span::styled(right.to_string(), Style::default().fg(Color::White)),
    ])
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
    if rule_bottom_h > 0 {
        constraints.push(Constraint::Length(rule_bottom_h));
    }
    if status_h > 0 {
        constraints.push(Constraint::Length(status_h));
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
    let rule_bottom_rect = if rule_bottom_h > 0 {
        let r = chunks[ci];
        ci += 1;
        Some(r)
    } else {
        None
    };
    let status_rect_opt = if status_h > 0 {
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
            .map(|_| Line::from(Span::styled(rule_txt.as_str(), style_horizontal_rule())))
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

    if let (Some(apr), Some(q)) = (approval_rect_opt, state.pending_user_question.as_ref()) {
        let preview_w = input_inner_w as usize;
        let mut input_lines: Vec<Line> = vec![Line::from(Span::styled(
            tr("ask-user-title"),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ))];
        let hdr = q.header.trim();
        if !hdr.is_empty() {
            input_lines.push(Line::from(Span::styled(hdr, style_dim())));
        }
        let qq = q.question.trim();
        if !qq.is_empty() {
            if text_display_width(qq) <= preview_w {
                input_lines.push(Line::from(Span::styled(qq, style_dim())));
            } else {
                for row in wrap_string_to_width(qq, preview_w.max(8)) {
                    input_lines.push(Line::from(Span::styled(row, style_dim())));
                }
            }
        }
        let n = q.option_labels.len().max(1);
        let pick = state.user_question_menu_selected % n;
        for (i, label) in q.option_labels.iter().enumerate() {
            let prefix = if i == pick { "❯ " } else { "  " };
            let st = if i == pick {
                style_menu_selected()
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
        Paragraph::new(Text::from(input_lines))
            .wrap(Wrap { trim: false })
            .render(apr, buf);
    } else if let (Some(apr), Some(p)) = (approval_rect_opt, state.pending_approval.as_ref()) {
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
                style_menu_selected()
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
                style_menu_selected()
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

    if let Some(rr) = rule_bottom_rect {
        let rule_w = dock_area.width.max(1) as usize;
        let rule_txt = "─".repeat(rule_w.min(512));
        let rule_lines: Vec<Line> = (0..rule_bottom_h)
            .map(|_| Line::from(Span::styled(rule_txt.as_str(), style_horizontal_rule())))
            .collect();
        Paragraph::new(Text::from(rule_lines)).render(rr, buf);
    }

    if let Some(sr) = status_rect_opt {
        let status_w = (sr.width as usize).max(4);
        let pre = stream_dock_activity_prefix(state);
        let line = if pre.is_empty() {
            stream_dock_status_line_spans(state, status_w)
        } else {
            let merged = format!("{}{}", pre, state.dock_status.as_str());
            Line::from(Span::styled(
                truncate_preview(merged.as_str(), status_w),
                style_dim(),
            ))
        };
        Paragraph::new(Text::from(line))
            .wrap(Wrap { trim: false })
            .render(sr, buf);
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

/// 回合结束后的短暂 **✶ 摘要** 前缀，拼在脚标 `dock_status` 前。执行中 / 待审批 / 选题时活动行在 HUD，此处返回空（与全屏 TUI 一致）。
pub(crate) fn stream_dock_activity_prefix(state: &ReplLineState) -> String {
    if state.executing_since.is_some()
        || state.pending_approval.is_some()
        || state.pending_user_question.is_some()
    {
        return String::new();
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
