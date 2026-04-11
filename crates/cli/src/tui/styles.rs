//! 终端语义色（不依赖 truecolor）。

use once_cell::sync::Lazy;
use ratatui::style::{Color, Modifier, Style};

/// 预计算的样式集合
struct ComputedStyles {
    dim: Style,
    brand: Style,
    user: Style,
    assistant: Style,
    assistant_prose: Style,
    tool: Style,
    tool_result: Style,
    error: Style,
    warn: Style,
}

impl ComputedStyles {
    fn new() -> Self {
        Self {
            dim: Style::default().fg(Color::Gray),
            brand: Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
            user: Style::default().fg(Color::LightCyan),
            assistant: Style::default()
                .fg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
            assistant_prose: Style::default().fg(Color::White),
            tool: Style::default()
                .fg(Color::LightYellow)
                .add_modifier(Modifier::BOLD),
            tool_result: Style::default().fg(Color::White),
            error: Style::default()
                .fg(Color::LightRed)
                .add_modifier(Modifier::BOLD),
            warn: Style::default()
                .fg(Color::LightMagenta)
                .add_modifier(Modifier::BOLD),
        }
    }
}

/// 全局预计算样式实例
static STYLES: Lazy<ComputedStyles> = Lazy::new(ComputedStyles::new);

pub(crate) fn style_dim() -> Style {
    STYLES.dim
}

pub(crate) fn style_brand() -> Style {
    STYLES.brand
}

pub(crate) fn style_user() -> Style {
    STYLES.user
}

pub(crate) fn style_assistant() -> Style {
    STYLES.assistant
}

/// 助手 Markdown 正文（对齐 Claude Code 终端：浅色正文，避免大段亮绿）
pub(crate) fn style_assistant_prose() -> Style {
    STYLES.assistant_prose
}

pub(crate) fn style_tool() -> Style {
    STYLES.tool
}

pub(crate) fn style_tool_result() -> Style {
    STYLES.tool_result
}

pub(crate) fn style_error() -> Style {
    STYLES.error
}

pub(crate) fn style_warn() -> Style {
    STYLES.warn
}

/// 欢迎卡外框（浅青，与品牌色同系、比旧 LightRed 更克制）。
pub(crate) fn style_welcome_border() -> Style {
    Style::default().fg(Color::LightCyan)
}
