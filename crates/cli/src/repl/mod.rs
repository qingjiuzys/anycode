//! 交互式流式终端（默认 `anycode` 无子命令、TTY）：默认 **备用屏全屏** ratatui；`ANYCODE_TERM_REPL_INLINE_LEGACY=1` 时为主缓冲 Inline（约占屏高 55%）+ 底栏。
//! 流式 REPL 输入区**不**提供 `/` 候选列表与灰色续写。
//!
//! Inline 下执行中正文可经 **`insert_before`** 进宿主 **scrollback**；不捕鼠（见 [`crate::term::terminal_guard`]）。

#![allow(unused_imports)] // 聚合 re-export：子模块与外部 crate 路径共用。

pub(crate) mod dock_render;
pub(crate) mod exec_parity;
pub(crate) mod inline;
pub(crate) mod line_state;
pub(crate) mod stream_app;
pub(crate) mod stream_events;
pub(crate) mod stream_paint;
pub(crate) mod stream_ratatui;
pub(crate) mod stream_render_msg;
pub(crate) mod stream_term;
pub(crate) mod stream_viewport;

pub(crate) use dock_render::{
    render_repl_dock_to_buffer, repl_dock_height, stream_dock_activity_prefix, ReplDockLayout,
};
pub(crate) use inline::{
    sanitize_stream_transcript_visual_noise, scrub_stream_transcript_llm_raw_dumps,
    stream_transcript_line_style,
};
pub(crate) use line_state::{
    reset_slash_state, stream_repl_scroll_reset_to_bottom, stream_transcript_page_step,
    stream_transcript_wheel_step, ReplCtl, ReplLineState, StreamTranscriptLayoutCache,
};
pub(crate) use stream_app::StreamReplUiSession;
pub(crate) use stream_events::{
    apply_stream_approval_key, apply_stream_user_question_key, handle_event,
    stream_repl_accept_key_event,
};
pub(crate) use stream_ratatui::{run_stream_repl_ui_thread, StreamReplAsyncCtl, StreamReplUiMsg};
pub(crate) use stream_render_msg::{drain_stream_repl_render_scrollback, StreamReplRenderMsg};
