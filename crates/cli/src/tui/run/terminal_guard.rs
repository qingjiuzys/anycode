//! TUI 终端模式 RAII：panic / 提前 return 时仍尽量恢复 raw mode。
//!
//! ## ratatui 与终端缓冲
//!
//! 本 TUI 每帧重绘整块字符矩阵。若在**主缓冲**且首帧不清屏，旧 shell 画面可能与首帧输出**叠画**（与布局算法无关，属缓冲混排）。
//!
//! ### 不想 `Clear(All)` 时，现实可选方案
//!
//! 1. **DEC 备用屏**（`ANYCODE_TUI_ALT_SCREEN=1`）— **不擦**主缓冲 scrollback，换到独立画布；退出后 shell 画面恢复。这是唯一能稳定隔离「整屏矩阵重绘」与历史输出的常规手段。
//! 2. **主缓冲 + 不清屏** — `loop_inner` 使用 ratatui **`Viewport::Inline(屏高)`**，视口锚在 **shell 光标下方**，避免 `MoveTo(0,0)` + 全屏视口把 UI 画在屏顶、与下方历史叠成「重复底栏」；若仍叠画，可用 **`ANYCODE_TUI_CLEAR_ON_START=1`** 或备用屏。
//! 3. **（已弃用默认路径）分区绘制（DECSTBM）** — 旧版 `repl_stream_dock` 用滚动边距 + 手工 blit；**当前默认** `anycode` / `anycode repl` 与 TUI 主缓冲一致，走 **`Viewport::Inline` + ratatui**（见 `repl::stream_ratatui`）。全屏 ratatui 矩阵为 **`anycode tui`**。
//! 4. **增量写终端**（diff / 非整块矩阵）— 可规避叠画，但 **不是** ratatui 默认模型；需自研或换栈。
//!
//! ### 默认策略（与上表独立）
//!
//! - **默认主缓冲紧凑 TUI**（类似 Claude Code 行内界面）：与 shell scrollback 混排，布局为「上缘内容 + 底栏」紧凑栈。需要**独立全屏画布**时设 **`ANYCODE_TUI_ALT_SCREEN=1`** 或 `config.json` 里 **`tui.alternateScreen`: `true`**。
//! - **主缓冲** + `ANYCODE_TUI_CLEAR_ON_START` 未设时 **默认不清屏**（保留 scrollback 位置、减轻「一进就顶」）；叠画风险可用 **`ANYCODE_TUI_CLEAR_ON_START=1`** 首帧 `Clear(All)`。
//! - `ANYCODE_TUI_CLEAR_ON_START=1` — 首帧清屏再绘；`=0` 与未设相同（默认不清）。
//! - **备用屏**仍可在任意环境下用 **`ANYCODE_TUI_ALT_SCREEN=1`** 强制开启。
//!
//! **鼠标**：未设 `ANYCODE_TUI_MOUSE` 时，仅备用屏内捕获滚轮；主缓冲留给 scrollback。`=1` 主缓冲也捕获；`=0` 全关。
//!
//! **同步绘制**：默认每帧 `draw` 前后发送 CSI `?2026`（crossterm 同步更新），减轻多行刷新撕裂；`ANYCODE_TUI_SYNC_DRAW=0` 关闭。`End` 在 RAII [`Drop`] 中发出，与 `draw` 内 panic 也能配对。帧间增量由 ratatui `Terminal` 双缓冲与 `Buffer::diff` 完成（见 `tui::backend` 模块注释）。入口见 [`terminal_draw_with_optional_sync`]。

use crossterm::{
    cursor::{Hide, Show},
    event::{DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, BeginSynchronizedUpdate, EndSynchronizedUpdate,
        EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use ratatui::backend::Backend;
use ratatui::{CompletedFrame, Frame, Terminal};
use std::io::{stdout, Write};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

/// 鼠标报告策略：
/// - **未设置** `ANYCODE_TUI_MOUSE`：仅在 **备用屏** 下开启捕获（滚轮在应用内滚 Workspace）；
///   主缓冲下不捕获，滚轮常由终端用于 **scrollback / 视口**，而非应用内虚拟滚动。
/// - `ANYCODE_TUI_MOUSE=1|true|yes|on`：主缓冲也捕获（滚轮走应用内滚动）。
/// - `=0|false|no|off`：始终关闭。
pub(super) fn tui_mouse_capture_enabled(alternate_screen: bool) -> bool {
    match std::env::var("ANYCODE_TUI_MOUSE") {
        Err(_) => alternate_screen,
        Ok(s) => {
            let s = s.trim().to_ascii_lowercase();
            if matches!(s.as_str(), "0" | "false" | "no" | "off") {
                return false;
            }
            if s.is_empty() {
                return alternate_screen;
            }
            true
        }
    }
}

fn is_env_defined_falsy(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "0" | "false" | "no" | "off"
    )
}

fn is_env_truthy(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// `Some` = 用户显式指定；`None` = 空串或无法识别的非空串 → 使用默认行为。
fn interpret_optional_env_bool(raw: &str) -> Option<bool> {
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

/// iTerm `tmux -CC` 启发式（控制模式下避免进备用屏）。
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
    // 已显式标了非 iTerm 终端时不再 spawn `tmux`。
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

/// 每帧 `terminal.draw()` 前后是否包 CSI `?2026` 同步更新（减少撕裂感）。
/// 默认 **true**；异常终端可 `ANYCODE_TUI_SYNC_DRAW=0`。
pub(super) fn tui_sync_draw_enabled() -> bool {
    match std::env::var("ANYCODE_TUI_SYNC_DRAW") {
        Err(_) => true,
        Ok(s) => {
            if let Some(b) = interpret_optional_env_bool(&s) {
                return b;
            }
            true
        }
    }
}

/// 主缓冲启动时是否执行 `Clear(ClearType::All)`（备用屏下 `loop_inner` 不调用）。
/// 默认 **false**（不清屏，避免视口被钉到屏顶、打断 scrollback）。`ANYCODE_TUI_CLEAR_ON_START=1` 时首帧清屏减轻与 shell 叠画。
pub(super) fn tui_main_buffer_clear_all_on_start() -> bool {
    match std::env::var("ANYCODE_TUI_CLEAR_ON_START") {
        Err(_) => false,
        Ok(s) => {
            if let Some(b) = interpret_optional_env_bool(&s) {
                return b;
            }
            false
        }
    }
}

/// `ANYCODE_TUI_ALT_SCREEN` 解析结果与运行环境组合后的决策（无 IO，可单测）。
///
/// - `explicit`：`Some(true/false)` 为可识别的显式值；`None` 为未设置、空串或无法识别 → **默认主缓冲（备用屏关）**。
/// - `tmux_control_mode`：保留参数供调用方记录；未显式指定时与上相同（默认关）。
pub(super) fn resolve_use_alternate_screen(
    explicit: Option<bool>,
    _tmux_control_mode: bool,
) -> bool {
    if let Some(b) = explicit {
        return b;
    }
    false
}

/// 是否进入 DEC 备用屏（独立整屏画布，不与主缓冲 scrollback 混排）。
///
/// `config_alternate_screen` 来自 `config.json` 的 `tui.alternateScreen`。仅当 `ANYCODE_TUI_ALT_SCREEN`
/// 未解析为 true/false 时采用（shell 里若只写 `VAR=0` 未 **export**，子进程看不到 env，会落到此项）。
///
/// 默认 **`false`**（主缓冲）。显式 `ANYCODE_TUI_ALT_SCREEN=1` 或配置 **`true`** 时进入备用屏。
pub(super) fn tui_use_alternate_screen_resolved(config_alternate_screen: Option<bool>) -> bool {
    let env_explicit = match std::env::var("ANYCODE_TUI_ALT_SCREEN") {
        Ok(v) => interpret_optional_env_bool(&v),
        Err(_) => None,
    };
    let explicit = env_explicit.or(config_alternate_screen);
    let tmux_cc = is_tmux_control_mode();
    let use_alt = resolve_use_alternate_screen(explicit, tmux_cc);
    if !use_alt
        && explicit.is_none()
        && tmux_cc
        && !LOGGED_TMUX_CC_DISABLE.swap(true, Ordering::Relaxed)
    {
        tracing::debug!(
            target: "anycode_cli",
            "tui: tmux -CC detected · alternate screen off by default · set ANYCODE_TUI_ALT_SCREEN=1 for isolated viewport"
        );
    }
    use_alt
}

/// 与 `loop_inner` 正常收尾顺序一致：按需 `DisableMouseCapture` → `DisableBracketedPaste` → `disable_raw_mode` → `Show` → 按需 `LeaveAlternateScreen`。
pub(super) struct TuiTerminalGuard {
    mouse_capture: bool,
    alternate_screen: bool,
}

impl TuiTerminalGuard {
    pub(super) fn enter(config_alternate_screen: Option<bool>) -> anyhow::Result<Self> {
        let alternate_screen = tui_use_alternate_screen_resolved(config_alternate_screen);
        enable_raw_mode()?;
        let mut out = stdout();
        if alternate_screen {
            execute!(out, EnterAlternateScreen)?;
        }
        // 隐藏硬件光标，避免与 ratatui 绘制的 ▌ 叠用；部分终端下 IME 会跟错光标位置。
        execute!(out, Hide)?;
        execute!(out, EnableBracketedPaste)?;
        let mouse_capture = tui_mouse_capture_enabled(alternate_screen);
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

/// 配对 CSI `?2026`：`Begin` 在构造时 best-effort 发送，`End` 在 [`Drop`] 时发送，避免 `draw` panic 后终端卡在同步更新模式。
struct SynchronizedUpdateGuard;

impl SynchronizedUpdateGuard {
    fn enter() -> Self {
        let _ = execute!(stdout(), BeginSynchronizedUpdate);
        Self
    }
}

impl Drop for SynchronizedUpdateGuard {
    fn drop(&mut self) {
        let _ = execute!(stdout(), EndSynchronizedUpdate);
    }
}

/// 在 [`Terminal::draw`] 前后可选包一层 CSI `?2026`；`sync` 与 [`tui_sync_draw_enabled`] 对齐。
pub(super) fn terminal_draw_with_optional_sync<B, F>(
    terminal: &mut Terminal<B>,
    sync: bool,
    draw: F,
) -> std::io::Result<CompletedFrame<'_>>
where
    B: Backend,
    F: FnOnce(&mut Frame<'_>),
{
    if sync {
        let _guard = SynchronizedUpdateGuard::enter();
        terminal.draw(draw)
    } else {
        terminal.draw(draw)
    }
}

/// ratatui 每帧重绘后，部分终端会丢失 DEC 鼠标模式（滚轮不再产生事件）；在 `draw` 之后幂等重发一次。
pub(super) fn refresh_mouse_capture_after_draw(alternate_screen: bool) -> std::io::Result<()> {
    if !tui_mouse_capture_enabled(alternate_screen) {
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
    use super::{
        interpret_optional_env_bool, is_env_defined_falsy, is_env_truthy,
        resolve_use_alternate_screen, tui_main_buffer_clear_all_on_start, tui_sync_draw_enabled,
        tui_use_alternate_screen_resolved,
    };
    use crossterm::execute;
    use crossterm::terminal::{BeginSynchronizedUpdate, EndSynchronizedUpdate};
    use std::sync::Mutex;

    #[test]
    fn synchronized_update_csi_round_trip_bytes() {
        let mut begin = Vec::new();
        execute!(&mut begin, BeginSynchronizedUpdate).unwrap();
        let mut end = Vec::new();
        execute!(&mut end, EndSynchronizedUpdate).unwrap();
        assert_eq!(begin.as_slice(), b"\x1b[?2026h");
        assert_eq!(end.as_slice(), b"\x1b[?2026l");
    }

    static ENV_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn with_cleared_tui_env<F: FnOnce()>(f: F) {
        let _g = ENV_TEST_LOCK.lock().expect("env test lock");
        let keys = ["ANYCODE_TUI_ALT_SCREEN", "ANYCODE_TUI_CLEAR_ON_START"];
        let saved: Vec<_> = keys.iter().map(|k| (*k, std::env::var_os(k))).collect();
        for k in keys {
            std::env::remove_var(k);
        }
        f();
        for (k, v) in saved {
            match v {
                Some(val) => std::env::set_var(k, val),
                None => std::env::remove_var(k),
            }
        }
    }

    #[test]
    fn clear_on_start_default_false_when_unset() {
        with_cleared_tui_env(|| {
            assert!(
                !tui_main_buffer_clear_all_on_start(),
                "unset ANYCODE_TUI_CLEAR_ON_START → no Clear(All) on main-buffer start"
            );
        });
    }

    #[test]
    fn sync_draw_follows_env() {
        let _g = ENV_TEST_LOCK.lock().expect("env test lock");
        let saved = std::env::var_os("ANYCODE_TUI_SYNC_DRAW");
        std::env::remove_var("ANYCODE_TUI_SYNC_DRAW");
        assert!(tui_sync_draw_enabled());
        std::env::set_var("ANYCODE_TUI_SYNC_DRAW", "0");
        assert!(!tui_sync_draw_enabled());
        match saved {
            Some(v) => std::env::set_var("ANYCODE_TUI_SYNC_DRAW", v),
            None => std::env::remove_var("ANYCODE_TUI_SYNC_DRAW"),
        }
    }

    #[test]
    fn resolve_alternate_screen_default_off_without_explicit() {
        assert!(!resolve_use_alternate_screen(None, false));
        assert!(!resolve_use_alternate_screen(None, true));
        assert!(resolve_use_alternate_screen(Some(true), true));
        assert!(!resolve_use_alternate_screen(Some(false), true));
    }

    #[test]
    fn alt_screen_follows_explicit_env() {
        let _g = ENV_TEST_LOCK.lock().expect("env test lock");
        let saved = std::env::var_os("ANYCODE_TUI_ALT_SCREEN");
        std::env::set_var("ANYCODE_TUI_ALT_SCREEN", "1");
        assert!(tui_use_alternate_screen_resolved(None));
        std::env::set_var("ANYCODE_TUI_ALT_SCREEN", "0");
        assert!(!tui_use_alternate_screen_resolved(None));
        match saved {
            Some(v) => std::env::set_var("ANYCODE_TUI_ALT_SCREEN", v),
            None => std::env::remove_var("ANYCODE_TUI_ALT_SCREEN"),
        }
    }

    #[test]
    fn alt_screen_config_used_when_env_unrecognized() {
        let _g = ENV_TEST_LOCK.lock().expect("env test lock");
        let saved = std::env::var_os("ANYCODE_TUI_ALT_SCREEN");
        std::env::set_var("ANYCODE_TUI_ALT_SCREEN", "maybe");
        assert!(!tui_use_alternate_screen_resolved(Some(false)));
        assert!(tui_use_alternate_screen_resolved(Some(true)));
        match saved {
            Some(v) => std::env::set_var("ANYCODE_TUI_ALT_SCREEN", v),
            None => std::env::remove_var("ANYCODE_TUI_ALT_SCREEN"),
        }
    }

    #[test]
    fn env_truthy_falsy() {
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
    fn interpret_optional_env_bool_unknown_is_none() {
        assert_eq!(interpret_optional_env_bool(""), None);
        assert_eq!(interpret_optional_env_bool("   "), None);
        assert_eq!(interpret_optional_env_bool("maybe"), None);
        assert_eq!(interpret_optional_env_bool("1"), Some(true));
        assert_eq!(interpret_optional_env_bool("0"), Some(false));
    }
}
