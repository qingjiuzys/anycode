//! Terminal semantic styles (truecolor, aligned with claude-code-rust `cli/ui.rs`).
//! With `NO_COLOR`, foregrounds fall back via [`super::palette`].

use once_cell::sync::Lazy;
use ratatui::style::{Modifier, Style};

use super::palette;

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
            dim: Style::default().fg(palette::muted()),
            brand: Style::default()
                .fg(palette::secondary())
                .add_modifier(Modifier::BOLD),
            user: Style::default().fg(palette::user_label()),
            assistant: Style::default()
                .fg(palette::assistant_label())
                .add_modifier(Modifier::BOLD),
            assistant_prose: Style::default().fg(palette::text()),
            tool: Style::default()
                .fg(palette::warning())
                .add_modifier(Modifier::BOLD),
            tool_result: Style::default().fg(palette::text()),
            error: Style::default()
                .fg(palette::error())
                .add_modifier(Modifier::BOLD),
            warn: Style::default()
                .fg(palette::warning())
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

/// 欢迎卡外框（紫系，对齐 claude-code-rust 品牌色）。
pub(crate) fn style_welcome_border() -> Style {
    Style::default().fg(palette::secondary())
}

/// 菜单 / 斜杠候选选中项、反向搜索光标条（对齐 claude-code-rust `print_prompt` 橙色 ▸）。
pub(crate) fn style_menu_selected() -> Style {
    Style::default()
        .fg(palette::accent())
        .add_modifier(Modifier::BOLD)
}

/// 与 Workspace / Dock 对齐的整行横线（muted 紫灰分隔线）。
pub(crate) fn style_horizontal_rule() -> Style {
    Style::default().fg(palette::divider())
}

/// 列表项样式：淡紫色圆点
pub(crate) fn style_list_bullet() -> Style {
    Style::default().fg(palette::list_bullet())
}

/// 分隔线样式：细微灰色
pub(crate) fn style_separator() -> Style {
    Style::default().fg(palette::separator())
}

/// 代码块样式：橙色加粗
pub(crate) fn style_code_block() -> Style {
    Style::default()
        .fg(palette::accent())
        .add_modifier(Modifier::BOLD)
}
