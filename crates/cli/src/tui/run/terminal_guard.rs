//! TUI 终端模式 RAII：panic / 提前 return 时仍尽量恢复 raw mode；**备用屏可选**（与 Claude Code 全屏策略对齐）。
//!
//! - **默认开启**终端鼠标报告（滚轮可滚动 Workspace）；若拖选复制受影响，设 `ANYCODE_TUI_MOUSE=0`（或 `false`/`no`/`off`）关闭。
//! - **主缓冲模式**（无备用屏，退出后会话留在终端滚动区，类 Claude 关 flicker）：
//!   - `ANYCODE_TUI_ALT_SCREEN=0`（或 `false` / `no` / `off`）
//!   - 或已设置 **`CLAUDE_CODE_NO_FLICKER=0`**（与 Claude Code `isFullscreenEnvEnabled` 的假值分支一致）

use crossterm::{
    cursor::{Hide, Show},
    event::{DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{stdout, Write};

/// 是否请求鼠标报告（滚轮等）。显式 `0` / `false` / `no` / `off` 关闭；其余非空值或未设置则开启。
fn env_mouse_capture_wanted() -> bool {
    match std::env::var("ANYCODE_TUI_MOUSE") {
        Err(_) => true,
        Ok(s) => {
            let s = s.trim().to_ascii_lowercase();
            !matches!(s.as_str(), "0" | "false" | "no" | "off")
        }
    }
}

/// `1`/`true`/`yes`/`on` → 使用备用屏；`0`/`false`/`no`/`off` → 主缓冲；其它非空字符串 → 默认开备用屏（减少误伤）。
fn parse_alt_screen_flag(v: &str) -> bool {
    match v.trim().to_ascii_lowercase().as_str() {
        "0" | "false" | "no" | "off" => false,
        "1" | "true" | "yes" | "on" => true,
        _ => true,
    }
}

/// 是否进入 DEC 备用屏。`ANYCODE_TUI_ALT_SCREEN` 优先；否则读 `CLAUDE_CODE_NO_FLICKER`；均未设非空则默认 `true`（保持既有 anycode 行为）。
pub(super) fn tui_use_alternate_screen() -> bool {
    if let Ok(v) = std::env::var("ANYCODE_TUI_ALT_SCREEN") {
        if !v.trim().is_empty() {
            return parse_alt_screen_flag(&v);
        }
    }
    if let Ok(v) = std::env::var("CLAUDE_CODE_NO_FLICKER") {
        if !v.trim().is_empty() {
            return parse_alt_screen_flag(&v);
        }
    }
    true
}

/// 与 `loop_inner` 正常收尾顺序一致：按需 `DisableMouseCapture` → `DisableBracketedPaste` → `disable_raw_mode` → `Show` → 按需 `LeaveAlternateScreen`。
pub(super) struct TuiTerminalGuard {
    mouse_capture: bool,
    alternate_screen: bool,
}

impl TuiTerminalGuard {
    pub(super) fn enter() -> anyhow::Result<Self> {
        let alternate_screen = tui_use_alternate_screen();
        enable_raw_mode()?;
        let mut out = stdout();
        if alternate_screen {
            execute!(out, EnterAlternateScreen)?;
        }
        // 隐藏硬件光标，避免与 ratatui 绘制的 ▌ 叠用；部分终端下 IME 会跟错光标位置。
        execute!(out, Hide)?;
        execute!(out, EnableBracketedPaste)?;
        let mouse_capture = env_mouse_capture_wanted();
        if mouse_capture {
            execute!(out, EnableMouseCapture)?;
            let _ = out.flush();
        }
        Ok(Self {
            mouse_capture,
            alternate_screen,
        })
    }

    pub(super) fn used_alternate_screen(&self) -> bool {
        self.alternate_screen
    }
}

/// ratatui 每帧重绘后，部分终端会丢失 DEC 鼠标模式（滚轮不再产生事件）；在 `draw` 之后幂等重发一次。
pub(super) fn refresh_mouse_capture_after_draw() -> std::io::Result<()> {
    if !env_mouse_capture_wanted() {
        return Ok(());
    }
    let mut out = stdout();
    execute!(out, EnableMouseCapture)?;
    out.flush()
}

impl Drop for TuiTerminalGuard {
    fn drop(&mut self) {
        let mut out = stdout();
        if self.mouse_capture {
            let _ = execute!(out, DisableMouseCapture);
        }
        let _ = execute!(out, DisableBracketedPaste);
        let _ = disable_raw_mode();
        let _ = execute!(out, Show);
        if self.alternate_screen {
            let _ = execute!(out, LeaveAlternateScreen);
        }
    }
}
