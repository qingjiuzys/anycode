//! Tokio / `ReplSink` → UI 线程：显式渲染与 scrollback 增量（与 `StreamReplUiMsg` 反向）。

/// 由异步侧或 `ReplSink` 发送，**仅** UI 线程 `try_recv` 消费（与 ratatui `draw` 同线程）。
#[derive(Debug)]
pub(crate) enum StreamReplRenderMsg {
    /// 主缓冲下经 `insert_before` 进入宿主 scrollback 的文本增量（通常为 `build_stream_turn_plain` 的 delta）。
    ScrollbackChunk(String),
    /// 清空已排队但未刷入终端的 scrollback 片段（如 `/clear`）。
    ClearScrollback,
    /// Tokio 侧仅更新了 dock / transcript 相关快照时显式发「需与 scrollback 同帧走 `paint_stream_frame`」的占位（`drain` 不修改 staging；与 `ScrollbackChunk` 同序时可保证随后 `draw_stream_frame` 看到最新 `ReplLineState`）。
    #[allow(dead_code)]
    DockInvalidate,
}

/// 非阻塞排空 `render_rx`，将 scrollback 片段追加到 `staging`（`ClearScrollback` 清空 staging）。
pub(crate) fn drain_stream_repl_render_scrollback(
    render_rx: &std::sync::mpsc::Receiver<StreamReplRenderMsg>,
    staging: &mut Vec<String>,
) {
    while let Ok(msg) = render_rx.try_recv() {
        match msg {
            StreamReplRenderMsg::ScrollbackChunk(s) => {
                if !s.is_empty() {
                    staging.push(s);
                }
            }
            StreamReplRenderMsg::ClearScrollback => staging.clear(),
            StreamReplRenderMsg::DockInvalidate => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drain_preserves_order_and_clear() {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut staging = Vec::new();
        tx.send(StreamReplRenderMsg::ScrollbackChunk("a".into()))
            .unwrap();
        tx.send(StreamReplRenderMsg::ScrollbackChunk("b".into()))
            .unwrap();
        drain_stream_repl_render_scrollback(&rx, &mut staging);
        assert_eq!(staging.concat(), "ab");

        tx.send(StreamReplRenderMsg::ClearScrollback).unwrap();
        tx.send(StreamReplRenderMsg::ScrollbackChunk("x".into()))
            .unwrap();
        drain_stream_repl_render_scrollback(&rx, &mut staging);
        assert_eq!(staging, vec!["x".to_string()]);
    }

    #[test]
    fn drain_preserves_scrollback_order_with_dock_invalidate_between() {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut staging = Vec::new();
        tx.send(StreamReplRenderMsg::ScrollbackChunk("a".into()))
            .unwrap();
        tx.send(StreamReplRenderMsg::DockInvalidate).unwrap();
        tx.send(StreamReplRenderMsg::ScrollbackChunk("b".into()))
            .unwrap();
        drain_stream_repl_render_scrollback(&rx, &mut staging);
        assert_eq!(staging.concat(), "ab");
    }
}
