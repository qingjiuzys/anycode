//! TUI 终端模式 RAII：panic / 提前 return 时仍尽量恢复 raw mode。
//!
//! **是否进入备用屏** 与 Claude Code `isFullscreenEnvEnabled()`（`claude-code/src/utils/fullscreen.ts`）对齐：
//! 1. 显式假值（`0`/`false`/`no`/`off`）→ 主缓冲；
//! 2. 显式真值（`1`/`true`/`yes`/`on`）→ 备用屏（可覆盖 tmux -CC 自动关闭）；
//! 3. `tmux -CC`（iTerm 集成）→ 默认主缓冲；
//! 4. 否则 `USER_TYPE=ant` 时默认备用屏，外部用户默认主缓冲。
//!
//! 环境变量：`ANYCODE_TUI_ALT_SCREEN`（优先）→ `CLAUDE_CODE_NO_FLICKER`（与 Claude 共用）。
//! **鼠标**：默认开启；`ANYCODE_TUI_MOUSE=0` 关闭。

use crossterm::{
    cursor::{Hide, Show},
    event::{DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{stdout, Write};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

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

/// 对齐 `claude-code/src/utils/envUtils.ts`：`isEnvDefinedFalsy`
fn is_env_defined_falsy(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "0" | "false" | "no" | "off"
    )
}

/// 对齐 `isEnvTruthy`
fn is_env_truthy(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// `Some` = 用户显式指定；`None` = 空串或无法识别的非空串 → 交给下一层（与 Claude 仅识别真假值一致）。
fn interpret_flicker_var(raw: &str) -> Option<bool> {
    let t = raw.trim();
    if t.is_empty() {
        return None;
    }
    if is_env_defined_falsy(t) {
        return Some(false);
    }
    if is_env_truthy(t) {
        return Some(true);
    }
    None
}

/// iTerm `tmux -CC` 启发式（与 `fullscreen.ts` `isTmuxControlModeEnvHeuristic` 一致）。
fn is_tmux_control_mode_env_heuristic() -> bool {
    if std::env::var_os("TMUX").is_none() {
        return false;
    }
    if std::env::var("TERM_PROGRAM").unwrap_or_default() != "iTerm.app" {
        return false;
    }
    let term = std::env::var("TERM").unwrap_or_default();
    !term.starts_with("screen") && !term.starts_with("tmux")
}

fn probe_tmux_control_mode_sync() -> bool {
    if is_tmux_control_mode_env_heuristic() {
        return true;
    }
    if std::env::var_os("TMUX").is_none() {
        return false;
    }
    // 已显式标了非 iTerm 终端时不再 spawn（与 Claude 一致）。
    if std::env::var_os("TERM_PROGRAM").is_some() {
        return false;
    }
    let output = Command::new("tmux")
        .args(["display-message", "-p", "#{client_control_mode}"])
        .output();
    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim() == "1",
        _ => false,
    }
}

static TMUX_CC_CACHE: OnceLock<bool> = OnceLock::new();
static LOGGED_TMUX_CC_DISABLE: AtomicBool = AtomicBool::new(false);

fn is_tmux_control_mode() -> bool {
    *TMUX_CC_CACHE.get_or_init(probe_tmux_control_mode_sync)
}

/// 是否进入 DEC 备用屏（Claude「fullscreen」语义）。
pub(super) fn tui_use_alternate_screen() -> bool {
    if let Ok(v) = std::env::var("ANYCODE_TUI_ALT_SCREEN") {
        if let Some(b) = interpret_flicker_var(&v) {
            return b;
        }
    }
    if let Ok(v) = std::env::var("CLAUDE_CODE_NO_FLICKER") {
        if let Some(b) = interpret_flicker_var(&v) {
            return b;
        }
    }
    if is_tmux_control_mode() {
        if !LOGGED_TMUX_CC_DISABLE.swap(true, Ordering::Relaxed) {
            tracing::debug!(
                target: "anycode_cli",
                "tui: tmux -CC detected, alternate screen off · override with ANYCODE_TUI_ALT_SCREEN=1 or CLAUDE_CODE_NO_FLICKER=1"
            );
        }
        return false;
    }
    std::env::var("USER_TYPE").map(|s| s == "ant").unwrap_or(false)
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

#[cfg(test)]
mod tests {
    use super::{interpret_flicker_var, is_env_defined_falsy, is_env_truthy};

    #[test]
    fn env_truthy_falsy_match_claude_env_utils() {
        assert!(is_env_truthy("1"));
        assert!(is_env_truthy("YES"));
        assert!(!is_env_truthy("0"));
        assert!(!is_env_truthy(""));
        assert!(is_env_defined_falsy("off"));
        assert!(is_env_defined_falsy("NO"));
        assert!(!is_env_defined_falsy(""));
        assert!(!is_env_defined_falsy("1"));
    }

    #[test]
    fn interpret_flicker_unknown_is_none() {
        assert_eq!(interpret_flicker_var(""), None);
        assert_eq!(interpret_flicker_var("   "), None);
        assert_eq!(interpret_flicker_var("maybe"), None);
        assert_eq!(interpret_flicker_var("1"), Some(true));
        assert_eq!(interpret_flicker_var("0"), Some(false));
    }
}
