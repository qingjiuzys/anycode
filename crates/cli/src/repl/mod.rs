//! Stream / Inline REPL（`anycode repl`）：ratatui `Viewport::Inline` + 底栏输入与 transcript 视口。

pub(crate) mod inline;
pub(crate) mod stream_ratatui;
pub(crate) mod stream_viewport;

pub(crate) use inline::{
    reset_slash_state, stream_repl_accept_key_event, stream_repl_scroll_reset_to_bottom,
    ReplLineState,
};
pub(crate) use stream_ratatui::{run_stream_repl_ui_thread, StreamReplAsyncCtl, StreamReplUiMsg};
