//! 终端尺寸快速连变时的绘制防抖（全屏 TUI 与流式 Inline REPL 共用）。
//!
//! **首帧**：第一次观察到终端尺寸时 **不** 跳过绘制。
//!
//! ratatui 0.24 下 `Viewport::Inline` 在 `autoresize`/`resize` 时会 `append_lines`（换行顶屏），
//! 连续 resize 会把视口内容反复顶入宿主 scrollback；防抖合并可显著减轻「叠行/脏历史」。

use std::time::Instant;

const DEBOUNCE_MS: u64 = 150;

pub(crate) struct ResizeDebounce {
    last_resize: Instant,
    last_size: Option<(u16, u16)>,
    skip_render: bool,
}

impl ResizeDebounce {
    pub(crate) fn new() -> Self {
        Self {
            last_resize: Instant::now(),
            last_size: None,
            skip_render: false,
        }
    }

    /// 返回 **是否跳过本帧绘制**（`true` = 跳过）。
    pub(crate) fn update(&mut self, current_size: (u16, u16)) -> bool {
        let now = Instant::now();

        let size_changed = match self.last_size {
            Some((w, h)) => w != current_size.0 || h != current_size.1,
            None => true,
        };

        if size_changed {
            let elapsed = now.duration_since(self.last_resize).as_millis() as u64;
            let already_observed = self.last_size.is_some();
            self.last_size = Some(current_size);
            self.last_resize = now;

            if resize_burst_should_skip_draw(already_observed, elapsed, DEBOUNCE_MS) {
                self.skip_render = true;
                return true;
            }
            self.skip_render = false;
            return false;
        }

        if self.skip_render
            && now.duration_since(self.last_resize).as_millis() as u64 >= DEBOUNCE_MS
        {
            self.skip_render = false;
        }

        self.skip_render
    }
}

/// 仅当已经记录过至少一次尺寸，且距上次 resize 事件不足 `debounce_ms` 时跳过（首帧永不跳过）。
fn resize_burst_should_skip_draw(
    already_observed_size: bool,
    elapsed_ms_since_last_resize_event: u64,
    debounce_ms: u64,
) -> bool {
    already_observed_size && elapsed_ms_since_last_resize_event < debounce_ms
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_size_never_skipped_by_burst_rule() {
        assert!(!resize_burst_should_skip_draw(false, 0, 150));
        assert!(!resize_burst_should_skip_draw(false, 149, 150));
    }

    #[test]
    fn rapid_subsequent_resize_skips_inside_window() {
        assert!(resize_burst_should_skip_draw(true, 0, 150));
        assert!(resize_burst_should_skip_draw(true, 149, 150));
    }

    #[test]
    fn burst_allows_render_after_quiet_period() {
        assert!(!resize_burst_should_skip_draw(true, 150, 150));
        assert!(!resize_burst_should_skip_draw(true, 200, 150));
    }
}
