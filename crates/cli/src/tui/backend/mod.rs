//! TUI 终端后端相关说明与单测。
//!
//! ## 与「Ink 式丝滑」的关系
//!
//! ratatui [`Terminal`](ratatui::Terminal) 内部维护 **双缓冲**，每帧通过
//! [`Buffer::diff`](ratatui::buffer::Buffer::diff) 仅将 **变更的格子** 交给
//! [`Backend::draw`](ratatui::backend::Backend::draw)。因此帧间写盘 **已是增量**，
//! 与 Claude Code（Ink）在「每帧全屏重刷字符矩阵」意义上的差异已大部分由框架承担。
//!
//! 进一步自定义 `Backend` 的收益主要在：首帧策略、CSI 同步（见 `ANYCODE_TUI_SYNC_DRAW`）、
//! 或极端终端的样式重置优化；**不是**简单再实现一层无差别的 cell diff。
//!
//! 若未来需要 **DECSTBM 分区**（全屏 TUI 仅占用部分行），属于另一套布局边界，见
//! [`crate::repl::stream_ratatui`]。

#[cfg(test)]
mod tests {
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::Color;
    use ratatui::widgets::{Block, Paragraph, Widget};

    /// 稳态下仅变更区域应产生较少 diff 单元（非全缓冲）。
    #[test]
    fn buffer_diff_idle_small_change_yields_few_cells() {
        let area = Rect::new(0, 0, 40, 10);
        let mut a = Buffer::empty(area);
        Paragraph::new("hello")
            .block(Block::default())
            .render(area, &mut a);

        let mut b = a.clone();
        let cell = b.get_mut(0, 0);
        cell.set_char('H');
        cell.set_fg(Color::Red);

        let n = a.diff(&b).len();
        assert!(
            n < (area.width as usize * area.height as usize) / 2,
            "expected sparse diff on small edit, got {n} cells",
        );
    }

    /// 从空缓冲到首帧绘制，diff 非空（首帧接管视口时 ratatui 会写若干变更 cell）。
    #[test]
    fn buffer_diff_first_paint_from_empty_non_trivial() {
        let area = Rect::new(0, 0, 8, 3);
        let prev = Buffer::empty(area);
        let mut next = Buffer::empty(area);
        Paragraph::new("fill")
            .block(Block::default())
            .render(area, &mut next);
        let n = prev.diff(&next).len();
        assert!(
            n >= 4,
            "expected non-trivial diff on first paint from empty, got {n} cells"
        );
    }
}
