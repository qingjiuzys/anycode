//! 终端语义色（不依赖 truecolor）。

use ratatui::style::{Color, Modifier, Style};

pub(crate) fn style_dim() -> Style {
    Style::default().fg(Color::DarkGray)
}

pub(crate) fn style_brand() -> Style {
    Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

pub(crate) fn style_user() -> Style {
    Style::default().fg(Color::Cyan)
}

pub(crate) fn style_assistant() -> Style {
    Style::default().fg(Color::Green)
}

/// 助手 Markdown 正文（对齐 Claude Code 终端：浅色正文，避免大段亮绿）
pub(crate) fn style_assistant_prose() -> Style {
    Style::default().fg(Color::White)
}

pub(crate) fn style_tool() -> Style {
    Style::default().fg(Color::Yellow)
}

pub(crate) fn style_tool_result() -> Style {
    Style::default().fg(Color::White)
}

pub(crate) fn style_error() -> Style {
    Style::default().fg(Color::Red)
}

pub(crate) fn style_warn() -> Style {
    Style::default().fg(Color::Magenta)
}
