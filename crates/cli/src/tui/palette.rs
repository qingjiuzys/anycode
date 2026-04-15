//! TrueColor palette aligned with claude-code-rust `src/cli/ui.rs` `colors`
//! (and message accents / dividers in the same module).
//!
//! Default ratatui output uses RGB; terminals without 24‑color support approximate. When `NO_COLOR`
//! is set, foreground colors fall back to ANSI-safe values.

use ratatui::style::Color;

/// Bump when default colors change so markdown LRU cache does not return stale styled output.
pub(crate) const PALETTE_CACHE_VERSION: u8 = 3;

pub(crate) fn no_color() -> bool {
    std::env::var_os("NO_COLOR").is_some()
}

/// Warm orange accent (`ACCENT` truecolor in claude-code-rust).
pub(crate) const ACCENT: (u8, u8, u8) = (255, 140, 66);

/// Soft purple secondary (`SECONDARY` truecolor).
pub(crate) const SECONDARY: (u8, u8, u8) = (147, 112, 219);

/// Claude name / H2-style heading accent (`print_claude_message` lavender).
pub(crate) const ASSISTANT_LABEL: (u8, u8, u8) = (200, 150, 255);

/// Horizontal rule / muted frame (`print_divider`).
pub(crate) const DIVIDER: (u8, u8, u8) = (100, 80, 120);

/// Code fence separator line (`print_claude_message`).
pub(crate) const CODE_FENCE: (u8, u8, u8) = (100, 90, 110); // 稍微亮一点的灰紫色

pub(crate) fn rgb(r: u8, g: u8, b: u8) -> Color {
    if no_color() {
        Color::Reset
    } else {
        Color::Rgb(r, g, b)
    }
}

pub(crate) fn accent() -> Color {
    rgb(ACCENT.0, ACCENT.1, ACCENT.2)
}

pub(crate) fn secondary() -> Color {
    rgb(SECONDARY.0, SECONDARY.1, SECONDARY.2)
}

pub(crate) fn assistant_label() -> Color {
    rgb(ASSISTANT_LABEL.0, ASSISTANT_LABEL.1, ASSISTANT_LABEL.2)
}

/// User / prompt chevron emphasis (claude-code-rust `print_user_message` / `print_prompt` orange family).
pub(crate) fn user_label() -> Color {
    accent()
}

/// HUD “thinking…” caption (`print_typing_indicator` gray text).
pub(crate) fn thinking_caption() -> Color {
    rgb(150, 150, 150)
}

/// Blockquote body: slightly warm gray-violet (secondary + muted).
pub(crate) fn blockquote_text() -> Color {
    rgb(170, 160, 185)
}

pub(crate) fn warning() -> Color {
    if no_color() {
        Color::Reset
    } else {
        Color::Yellow
    }
}

pub(crate) fn error() -> Color {
    if no_color() {
        Color::Reset
    } else {
        Color::Red
    }
}

pub(crate) fn text() -> Color {
    if no_color() {
        Color::Reset
    } else {
        Color::White
    }
}

pub(crate) fn muted() -> Color {
    if no_color() {
        Color::Reset
    } else {
        Color::Gray
    }
}

pub(crate) fn divider() -> Color {
    rgb(DIVIDER.0, DIVIDER.1, DIVIDER.2)
}

pub(crate) fn blockquote_rule() -> Color {
    divider()
}

pub(crate) fn code_fence_line() -> Color {
    rgb(CODE_FENCE.0, CODE_FENCE.1, CODE_FENCE.2)
}

/// Links: secondary purple (distinct from orange H1).
pub(crate) fn link() -> Color {
    secondary()
}

/// List item bullets: lighter purple for better readability
pub(crate) fn list_bullet() -> Color {
    rgb(180, 140, 220)
}

/// Separator lines between sections: subtle gray
pub(crate) fn separator() -> Color {
    rgb(120, 100, 140)
}
