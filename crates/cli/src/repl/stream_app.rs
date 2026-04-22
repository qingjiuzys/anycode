//! Stream REPL 轴心初始化：聚合共享 `ReplLineState` 与 UI ↔ 工作线程通道。
//! 轴心线程跑 [`super::stream_ratatui::run_stream_repl_ui_thread`]（`StreamReplUiSession`）；Tokio `select!` 在从属
//! `current_thread` 运行时上执行（见 [`crate::tasks::tasks_repl::run_interactive_tty_stream`]）。

use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc::unbounded_channel;

use crate::repl::{ReplLineState, StreamReplAsyncCtl, StreamReplRenderMsg, StreamReplUiMsg};
use crate::term::{stream_repl_use_alternate_screen, PendingApproval, PendingUserQuestion};

/// 轴心线程单次 `run` 所需句柄（通道 + 共享 `ReplLineState` + 审批/选题队列）。
pub(crate) struct StreamReplUiSession {
    pub state: Arc<Mutex<ReplLineState>>,
    pub to_worker: tokio::sync::mpsc::UnboundedSender<StreamReplUiMsg>,
    pub ctrl_rx: mpsc::Receiver<StreamReplAsyncCtl>,
    pub render_rx: mpsc::Receiver<StreamReplRenderMsg>,
    pub approval_rx: Arc<Mutex<Option<tokio::sync::mpsc::Receiver<PendingApproval>>>>,
    pub question_rx: Arc<Mutex<Option<tokio::sync::mpsc::Receiver<PendingUserQuestion>>>>,
    pub repl_debug_events: bool,
    pub use_alternate_screen: bool,
}

pub(crate) struct StreamReplAxis {
    pub state: Arc<Mutex<ReplLineState>>,
    pub ui_to_worker_tx: tokio::sync::mpsc::UnboundedSender<StreamReplUiMsg>,
    pub ui_to_worker_rx: tokio::sync::mpsc::UnboundedReceiver<StreamReplUiMsg>,
    pub ctrl_tx: mpsc::Sender<StreamReplAsyncCtl>,
    pub ctrl_rx: mpsc::Receiver<StreamReplAsyncCtl>,
    pub stream_render_tx: mpsc::Sender<StreamReplRenderMsg>,
    pub stream_render_rx: mpsc::Receiver<StreamReplRenderMsg>,
    pub use_repl_alt: bool,
}

pub(crate) fn init_stream_repl_axis(config_tui_alt: Option<bool>) -> StreamReplAxis {
    let use_repl_alt = stream_repl_use_alternate_screen(config_tui_alt);
    let mut line0 = ReplLineState::default();
    line0.stream_repl_host_scrollback = !use_repl_alt;
    let state = Arc::new(Mutex::new(line0));
    let (ui_to_worker_tx, ui_to_worker_rx) = unbounded_channel::<StreamReplUiMsg>();
    let (ctrl_tx, ctrl_rx) = mpsc::channel::<StreamReplAsyncCtl>();
    let (stream_render_tx, stream_render_rx) = mpsc::channel::<StreamReplRenderMsg>();
    StreamReplAxis {
        state,
        ui_to_worker_tx,
        ui_to_worker_rx,
        ctrl_tx,
        ctrl_rx,
        stream_render_tx,
        stream_render_rx,
        use_repl_alt,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::term::stream_repl_use_alternate_screen;

    /// 锁定：轴心初始化与 `terminal_guard` 决策一致，且 `stream_repl_host_scrollback` 仅 Inline 遗留路径为 true。
    #[test]
    fn axis_matches_alt_resolution_and_host_scrollback_flag() {
        let axis = init_stream_repl_axis(None);
        assert_eq!(axis.use_repl_alt, stream_repl_use_alternate_screen(None));
        let st = axis.state.lock().unwrap();
        assert_eq!(st.stream_repl_host_scrollback, !axis.use_repl_alt);
    }
}
