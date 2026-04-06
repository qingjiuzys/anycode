//! 右侧小宠物：空闲静息，执行任务时简单帧动画（与顶栏 working 状态呼应）。

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use super::styles::{style_assistant, style_dim};

/// 工作帧序号（由外层用 `elapsed/250ms % 4` 驱动）。
pub(crate) fn pet_panel_lines(frame: u64, executing: bool) -> Vec<Line<'static>> {
    if executing {
        match frame % 4 {
            0 => vec![
                Line::from(Span::styled(" ·ᴗ· ", style_assistant())),
                Line::from(Span::styled(" /|\\ ", style_dim())),
                Line::from(Span::styled(" / \\ ", style_dim())),
            ],
            1 => vec![
                Line::from(Span::styled(" ·ω· ", style_assistant())),
                Line::from(Span::styled(" /|\\ ", style_dim())),
                Line::from(Span::styled("  |  ", style_dim())),
            ],
            2 => vec![
                Line::from(Span::styled(" ·ᴗ· ", style_assistant())),
                Line::from(Span::styled(" >|  ", style_dim())),
                Line::from(Span::styled(" / \\ ", style_dim())),
            ],
            _ => vec![
                Line::from(Span::styled(" ^▽^ ", style_assistant())),
                Line::from(Span::styled(" /|\\ ", style_dim())),
                Line::from(Span::styled(" / \\ ", style_dim())),
            ],
        }
    } else {
        vec![
            Line::from(Span::styled(" ·ᴗ· ", style_dim())),
            Line::from(Span::styled(" zzz ", Style::default().fg(Color::DarkGray))),
            Line::from(Span::styled("  ─  ", style_dim())),
        ]
    }
}
