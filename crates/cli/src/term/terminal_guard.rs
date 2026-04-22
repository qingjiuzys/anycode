//! 流式 ratatui 与终端：备用屏/主缓冲策略、环境解析（无全屏矩阵主循环 RAII）。
//!
//! ## ratatui 与终端缓冲
//!
//! 每帧对字符矩阵重绘。若在**主缓冲**且首帧不清屏，旧 shell 画面可能与首帧输出**叠画**。
//!
//! - **DEC 备用屏**（`ANYCODE_TERM_ALT_SCREEN=1` 等）— 不擦主缓冲 scrollback，换到独立画布。
//! - **主缓冲** — 流式 REPL 可用 **`Viewport::Inline`** 与宿主 scrollback 混排（见 [`crate::tasks::tasks_repl::run_interactive_tty_stream`]、`crate::repl::stream_ratatui`）。
//! - 同步更新 **CSI `?2026`** 等由 ratatui/crossterm 在各自绘制路径中处理；本文件仅保留**环境解析**与 `stream_repl_use_alternate_screen`。
//!
//! ### `anycode` / `anycode repl` 流式界面
//!
//! - **默认**流式 REPL 倾向备用屏 + 全屏视口（与 [`stream_repl_use_alternate_screen`] 一致）；主缓冲行内见 `ANYCODE_TERM_REPL_INLINE_LEGACY`。

use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

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

/// 每帧 `terminal.draw()` 前后是否包 CSI `?2026` 同步更新（减少撕裂感）。
/// 默认 **true**；异常终端可 `ANYCODE_TERM_SYNC_DRAW=0`（重命名见 CHANGELOG）。
#[allow(dead_code)]
pub(super) fn term_sync_draw_enabled() -> bool {
    match std::env::var("ANYCODE_TERM_SYNC_DRAW") {
        Err(_) => true,
        Ok(s) => {
            if let Some(b) = interpret_optional_env_bool(&s) {
                return b;
            }
            true
        }
    }
}

/// 主缓冲启动时是否执行 `Clear(ClearType::All)`。
/// 默认 **false**。`ANYCODE_TERM_CLEAR_ON_START=1` 时首帧清屏（重命名见 CHANGELOG）。
#[allow(dead_code)]
pub(super) fn term_main_buffer_clear_all_on_start() -> bool {
    match std::env::var("ANYCODE_TERM_CLEAR_ON_START") {
        Err(_) => false,
        Ok(s) => {
            if let Some(b) = interpret_optional_env_bool(&s) {
                return b;
            }
            false
        }
    }
}

/// `ANYCODE_TERM_ALT_SCREEN` 解析结果与运行环境组合后的决策（无 IO，可单测）。
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

/// 是否进入 DEC 备用屏（从 `config.json` 的 `terminal.alternateScreen` 等解析；当前 env 名见源码，重命名见 CHANGELOG）。
pub(super) fn term_use_alternate_screen_resolved(config_alternate_screen: Option<bool>) -> bool {
    let env_explicit = match std::env::var("ANYCODE_TERM_ALT_SCREEN") {
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
            "term: tmux -CC detected · alternate screen off by default · set ANYCODE_TERM_ALT_SCREEN=1 for isolated viewport"
        );
    }
    use_alt
}

/// `anycode repl` 流式 ratatui 是否使用 **备用屏 + 全屏视口**。
///
/// **默认开**（与 claude-code-rust 式全屏矩阵对齐）：未显式配置时走备用屏 + `Terminal::new()`。
///
/// - **`ANYCODE_TERM_REPL_INLINE_LEGACY=1`**：恢复旧版主缓冲 **`Viewport::Inline`** + 宿主 `insert_before` 路径（紧凑混排）。
/// - **`ANYCODE_TERM_REPL_ALT_SCREEN`**：仅针对流式 REPL；可识别布尔串时优先生效（可显式关：`0`）。
/// - 否则若用户显式设置了 **`ANYCODE_TERM_ALT_SCREEN`** 或 **终端配置 alternateScreen**，与 [`term_use_alternate_screen_resolved`] 共用决策。
/// - 若以上均未显式指定，流式 REPL **默认 `true`**（备用屏）。
pub(crate) fn stream_repl_use_alternate_screen(config_alternate_screen: Option<bool>) -> bool {
    if let Ok(v) = std::env::var("ANYCODE_TERM_REPL_INLINE_LEGACY") {
        if let Some(b) = interpret_optional_env_bool(&v) {
            return !b;
        }
        if is_env_truthy(&v) {
            return false;
        }
    }
    if let Ok(v) = std::env::var("ANYCODE_TERM_REPL_ALT_SCREEN") {
        if let Some(b) = interpret_optional_env_bool(&v) {
            return b;
        }
    }
    let term_env_has_explicit = std::env::var("ANYCODE_TERM_ALT_SCREEN")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .and_then(|v| interpret_optional_env_bool(&v))
        .is_some();
    if term_env_has_explicit || config_alternate_screen.is_some() {
        return term_use_alternate_screen_resolved(config_alternate_screen);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::{
        interpret_optional_env_bool, is_env_defined_falsy, is_env_truthy,
        resolve_use_alternate_screen, stream_repl_use_alternate_screen,
        term_main_buffer_clear_all_on_start, term_sync_draw_enabled,
        term_use_alternate_screen_resolved,
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
        let keys = ["ANYCODE_TERM_ALT_SCREEN", "ANYCODE_TERM_CLEAR_ON_START"];
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
                !term_main_buffer_clear_all_on_start(),
                "unset ANYCODE_TERM_CLEAR_ON_START → no Clear(All) on main-buffer start"
            );
        });
    }

    #[test]
    fn sync_draw_follows_env() {
        let _g = ENV_TEST_LOCK.lock().expect("env test lock");
        let saved = std::env::var_os("ANYCODE_TERM_SYNC_DRAW");
        std::env::remove_var("ANYCODE_TERM_SYNC_DRAW");
        assert!(term_sync_draw_enabled());
        std::env::set_var("ANYCODE_TERM_SYNC_DRAW", "0");
        assert!(!term_sync_draw_enabled());
        match saved {
            Some(v) => std::env::set_var("ANYCODE_TERM_SYNC_DRAW", v),
            None => std::env::remove_var("ANYCODE_TERM_SYNC_DRAW"),
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
        let saved = std::env::var_os("ANYCODE_TERM_ALT_SCREEN");
        std::env::set_var("ANYCODE_TERM_ALT_SCREEN", "1");
        assert!(term_use_alternate_screen_resolved(None));
        std::env::set_var("ANYCODE_TERM_ALT_SCREEN", "0");
        assert!(!term_use_alternate_screen_resolved(None));
        match saved {
            Some(v) => std::env::set_var("ANYCODE_TERM_ALT_SCREEN", v),
            None => std::env::remove_var("ANYCODE_TERM_ALT_SCREEN"),
        }
    }

    #[test]
    fn alt_screen_config_used_when_env_unrecognized() {
        let _g = ENV_TEST_LOCK.lock().expect("env test lock");
        let saved = std::env::var_os("ANYCODE_TERM_ALT_SCREEN");
        std::env::set_var("ANYCODE_TERM_ALT_SCREEN", "maybe");
        assert!(!term_use_alternate_screen_resolved(Some(false)));
        assert!(term_use_alternate_screen_resolved(Some(true)));
        match saved {
            Some(v) => std::env::set_var("ANYCODE_TERM_ALT_SCREEN", v),
            None => std::env::remove_var("ANYCODE_TERM_ALT_SCREEN"),
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

    fn with_cleared_repl_alt_env<F: FnOnce()>(f: F) {
        let _g = ENV_TEST_LOCK.lock().expect("env test lock");
        let keys = [
            "ANYCODE_TERM_REPL_ALT_SCREEN",
            "ANYCODE_TERM_REPL_INLINE_LEGACY",
            "ANYCODE_TERM_ALT_SCREEN",
        ];
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
    fn stream_repl_alt_defaults_on_without_explicit_global() {
        with_cleared_repl_alt_env(|| {
            assert!(
                stream_repl_use_alternate_screen(None),
                "repl defaults to alternate screen when TUI env/config unset"
            );
        });
    }

    #[test]
    fn stream_repl_inline_legacy_forces_main_buffer() {
        with_cleared_repl_alt_env(|| {
            std::env::set_var("ANYCODE_TERM_REPL_INLINE_LEGACY", "1");
            assert!(!stream_repl_use_alternate_screen(None));
        });
    }

    #[test]
    fn stream_repl_alt_follows_tui_env_when_explicit() {
        with_cleared_repl_alt_env(|| {
            std::env::set_var("ANYCODE_TERM_ALT_SCREEN", "0");
            assert!(!stream_repl_use_alternate_screen(None));
            std::env::set_var("ANYCODE_TERM_ALT_SCREEN", "1");
            assert!(stream_repl_use_alternate_screen(None));
        });
    }

    #[test]
    fn stream_repl_alt_stream_env_overrides_tui() {
        with_cleared_repl_alt_env(|| {
            std::env::set_var("ANYCODE_TERM_ALT_SCREEN", "1");
            std::env::set_var("ANYCODE_TERM_REPL_ALT_SCREEN", "0");
            assert!(!stream_repl_use_alternate_screen(None));
        });
    }

    #[test]
    fn stream_repl_alt_config_used_when_no_tui_env() {
        with_cleared_repl_alt_env(|| {
            assert!(!stream_repl_use_alternate_screen(Some(false)));
            assert!(stream_repl_use_alternate_screen(Some(true)));
        });
    }
}
